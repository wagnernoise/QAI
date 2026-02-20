use anyhow::Result;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode,
        KeyModifiers, KeyboardEnhancementFlags, MouseEventKind,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use crate::tui::state::{App, Screen, ChatFocus, MENU_ITEMS};
use crate::tui::providers::Provider;
use crate::tui::api::{save_api_token, fetch_ollama_models, stream_message, StreamRequest};
use crate::agent::ReActAgent;
use crate::tui::draw::draw;
use crate::tui::input::{TextInput, handle_text_input_key};
use crate::tui::util::strip_model_tags;
use arboard::Clipboard;
use std::{io, path::PathBuf, time::Instant};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn run(prompt_path: PathBuf) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // Try to enable kitty keyboard protocol so terminals that support it
    // (iTerm2, Ghostty, WezTerm, Alacritty, etc.) report Shift+Enter correctly.
    let kitty_supported = execute!(
        stdout,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        )
    ).is_ok();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(prompt_path);
    let result = event_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    if kitty_supported {
        let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
    }
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    result
}

// ── Event loop ────────────────────────────────────────────────────────────────

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut event_stream = EventStream::new();
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(50));
    // Channel for streaming token chunks: sender given to spawn, receiver polled here
    let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<Option<String>>();

    loop {
        terminal.draw(|f| draw(f, app))?;

        tokio::select! {
            // 50 ms tick — redraws and clears timed status messages
            _ = tick.tick() => {
                if let Some(saved_at) = app.token_saved_at {
                    if saved_at.elapsed() >= std::time::Duration::from_secs(3) {
                        if app.status == "✓ API token saved" {
                            app.status = String::new();
                        }
                        app.token_saved_at = None;
                    }
                }
            }

            // Incoming streaming token chunk
            Some(chunk) = stream_rx.recv() => {
                match chunk {
                    Some(token) => {
                        // Append token to last assistant message
                        if let Some((role, content)) = app.messages.last_mut() {
                            if role == "assistant" {
                                content.push_str(&token);
                            } else {
                                app.messages.push(("assistant".to_string(), token));
                            }
                        } else {
                            app.messages.push(("assistant".to_string(), token));
                        }
                        app.status = String::new();
                    }
                    None => {
                        // Stream finished
                        app.streaming = false;
                        app.status = String::new();
                    }
                }
            }

            // Keyboard / terminal events
            Some(Ok(event)) = event_stream.next() => {
                // Trackpad / mouse scroll — works on any screen
                if let Event::Mouse(mouse) = &event {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            if app.screen == Screen::Chat {
                                app.chat_scroll = app.chat_scroll.saturating_sub(3);
                                app.chat_scroll_manual = true;
                            } else if app.screen == Screen::Show {
                                app.scroll_offset = app.scroll_offset.saturating_sub(3);
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if app.screen == Screen::Chat {
                                app.chat_scroll = app.chat_scroll.saturating_add(3);
                                app.chat_scroll_manual = true;
                            } else if app.screen == Screen::Show {
                                app.scroll_offset = app.scroll_offset.saturating_add(3);
                            }
                        }
                        // Scrollbar click or drag — hit-test against the right edge of conv_rect or input_rect
                        // Also track mouse selection inside the conversation area
                        MouseEventKind::Down(_) => {
                            if app.screen == Screen::Chat {
                                // Input scrollbar hit-test
                                let ir = app.input_rect;
                                let input_scrollbar_col = ir.x + ir.width.saturating_sub(1);
                                if mouse.column == input_scrollbar_col && ir.height > 2
                                    && mouse.row >= ir.y && mouse.row < ir.y + ir.height
                                    && app.input_max_scroll_stored > 0 {
                                    let track_top = ir.y + 1;
                                    let track_bottom = ir.y + ir.height.saturating_sub(2);
                                    let track_len = track_bottom.saturating_sub(track_top) as usize;
                                    if track_len > 0 {
                                        let ratio = (mouse.row.saturating_sub(track_top)) as f32 / track_len as f32;
                                        app.input_scroll = (ratio * app.input_max_scroll_stored as f32).round() as u16;
                                    }
                                }
                                let r = app.conv_rect;
                                let scrollbar_col = r.x + r.width.saturating_sub(1);
                                if mouse.column == scrollbar_col && r.height > 2 {
                                    let track_top = r.y + 1;
                                    let track_bottom = r.y + r.height.saturating_sub(2);
                                    let track_len = track_bottom.saturating_sub(track_top) as usize;
                                    if track_len > 0 && mouse.row >= track_top && mouse.row <= track_bottom {
                                        let ratio = (mouse.row - track_top) as f32 / track_len as f32;
                                        let new_scroll = (ratio * app.conv_max_scroll as f32).round() as u16;
                                        app.chat_scroll = new_scroll;
                                        app.chat_scroll_manual = true;
                                    }
                                } else if mouse.column >= r.x && mouse.column < r.x + r.width
                                    && mouse.row >= r.y && mouse.row < r.y + r.height {
                                    // Start a new text selection — store as content line index
                                    let inner_top = r.y + 1;
                                    let eff = if app.chat_scroll_manual {
                                        app.chat_scroll.min(app.conv_max_scroll)
                                    } else {
                                        app.conv_max_scroll
                                    };
                                    let idx = (mouse.row.saturating_sub(inner_top) as usize)
                                        .saturating_add(eff as usize);
                                    app.sel_start = Some(idx);
                                    app.sel_end = Some(idx);
                                }
                            }
                        }
                        MouseEventKind::Drag(_) => {
                            if app.screen == Screen::Chat {
                                // Input scrollbar drag hit-test
                                let ir = app.input_rect;
                                let input_scrollbar_col = ir.x + ir.width.saturating_sub(1);
                                if mouse.column == input_scrollbar_col && ir.height > 2
                                    && mouse.row >= ir.y && mouse.row < ir.y + ir.height
                                    && app.input_max_scroll_stored > 0 {
                                    let track_top = ir.y + 1;
                                    let track_bottom = ir.y + ir.height.saturating_sub(2);
                                    let track_len = track_bottom.saturating_sub(track_top) as usize;
                                    if track_len > 0 {
                                        let ratio = (mouse.row.saturating_sub(track_top)) as f32 / track_len as f32;
                                        app.input_scroll = (ratio * app.input_max_scroll_stored as f32).round() as u16;
                                    }
                                }
                                let r = app.conv_rect;
                                let scrollbar_col = r.x + r.width.saturating_sub(1);
                                if mouse.column == scrollbar_col && r.height > 2 {
                                    let track_top = r.y + 1;
                                    let track_bottom = r.y + r.height.saturating_sub(2);
                                    let track_len = track_bottom.saturating_sub(track_top) as usize;
                                    if track_len > 0 && mouse.row >= track_top && mouse.row <= track_bottom {
                                        let ratio = (mouse.row - track_top) as f32 / track_len as f32;
                                        let new_scroll = (ratio * app.conv_max_scroll as f32).round() as u16;
                                        app.chat_scroll = new_scroll;
                                        app.chat_scroll_manual = true;
                                    }
                                } else if app.sel_start.is_some()
                                    && mouse.column >= r.x && mouse.column < r.x + r.width
                                    && mouse.row >= r.y && mouse.row < r.y + r.height {
                                    // Extend selection — store as content line index
                                    let inner_top = r.y + 1;
                                    let eff = if app.chat_scroll_manual {
                                        app.chat_scroll.min(app.conv_max_scroll)
                                    } else {
                                        app.conv_max_scroll
                                    };
                                    let idx = (mouse.row.saturating_sub(inner_top) as usize)
                                        .saturating_add(eff as usize);
                                    app.sel_end = Some(idx);
                                }
                            }
                        }
                        MouseEventKind::Up(_) => {
                            // Selection finalized on mouse-up; do NOT auto-copy.
                            // User copies explicitly with Ctrl+C / Cmd+C.
                        }
                        _ => {}
                    }
                }
                if let Event::Key(key) = event {
                    match &app.screen {
                        Screen::Menu => match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                            KeyCode::Up => {
                                let i = app.menu_state.selected().unwrap_or(0);
                                app.menu_state.select(Some(i.saturating_sub(1)));
                            }
                            KeyCode::Down => {
                                let i = app.menu_state.selected().unwrap_or(0);
                                app.menu_state.select(Some((i + 1).min(MENU_ITEMS.len() - 1)));
                            }
                            KeyCode::Enter => {
                                let i = app.menu_state.selected().unwrap_or(0);
                                match i {
                                    0 => app.screen = Screen::Info,
                                    1 => { app.scroll_offset = 0; app.screen = Screen::Show; }
                                    2 => app.screen = Screen::Validate,
                                    3 => app.screen = Screen::Tools,
                                    4 => app.screen = Screen::Chat,
                                    5 => return Ok(()),
                                    _ => {}
                                }
                            }
                            _ => {}
                        },
                        Screen::Info | Screen::Validate => {
                            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                                app.screen = Screen::Menu;
                            }
                        }
                        Screen::Tools => match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::Menu,
                            KeyCode::Up | KeyCode::Char('k') => {
                                let i = app.tools_provider_index.saturating_sub(1);
                                app.tools_provider_index = i;
                                app.tools_provider_list_state.select(Some(i));
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let i = (app.tools_provider_index + 1).min(Provider::all().len() - 1);
                                app.tools_provider_index = i;
                                app.tools_provider_list_state.select(Some(i));
                            }
                            KeyCode::Enter => {
                                app.provider_index = app.tools_provider_index;
                                app.provider_list_state.select(Some(app.tools_provider_index));
                                app.screen = Screen::Chat;
                            }
                            _ => {}
                        },
                        Screen::Show => match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::Menu,
                            KeyCode::Down | KeyCode::Char('j') => app.scroll_offset += 1,
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.scroll_offset = app.scroll_offset.saturating_sub(1)
                            }
                            _ => {}
                        },
                        Screen::Chat => {
                            handle_chat_key(app, key, stream_tx.clone()).await?;
                        }
                    }
                }
            }
        }
    }
}

async fn handle_chat_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    stream_tx: mpsc::UnboundedSender<Option<String>>,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            if app.streaming {
                if let Some(t) = app.last_esc_at {
                    if t.elapsed() <= std::time::Duration::from_secs(1) {
                        // Second ESC within 1 s — cancel inference
                        app.cancel_token.cancel();
                        app.cancel_token = CancellationToken::new();
                        app.streaming = false;
                        app.status = "⛔ Inference stopped".to_string();
                        app.last_esc_at = None;
                        return Ok(());
                    }
                }
                app.last_esc_at = Some(Instant::now());
                app.status = "Press ESC again to stop inference".to_string();
            } else {
                app.screen = Screen::Menu;
            }
        }
        KeyCode::F(2) => {
            app.agent_mode = !app.agent_mode;
            app.status = if app.agent_mode {
                "🤖 Agent Mode ON (ReAct loop) — F2 to toggle".to_string()
            } else {
                "💬 Chat Mode — F2 to enable Agent Mode".to_string()
            };
        }
        KeyCode::Tab => {
            let is_custom = app.selected_provider() == Provider::Custom;
            let is_ollama = app.selected_provider() == Provider::Ollama;
            app.chat_focus = match app.chat_focus {
                ChatFocus::ProviderList => {
                    if is_ollama {
                        // fetch models when entering model list
                        if app.ollama_models.is_empty() {
                            fetch_ollama_models(app).await;
                        }
                        ChatFocus::ModelList
                    } else if is_custom {
                        ChatFocus::CustomUrl
                    } else {
                        ChatFocus::Token
                    }
                }
                ChatFocus::ModelList => ChatFocus::Token,
                ChatFocus::Token => ChatFocus::Message,
                ChatFocus::CustomUrl => ChatFocus::Message,
                ChatFocus::Message => ChatFocus::Conversation,
                ChatFocus::Conversation => ChatFocus::ProviderList,
            };
        }
        KeyCode::BackTab => {
            let is_custom = app.selected_provider() == Provider::Custom;
            let is_ollama = app.selected_provider() == Provider::Ollama;
            app.chat_focus = match app.chat_focus {
                ChatFocus::ProviderList => ChatFocus::Conversation,
                ChatFocus::ModelList => ChatFocus::ProviderList,
                ChatFocus::Token => {
                    if is_ollama {
                        ChatFocus::ModelList
                    } else {
                        ChatFocus::ProviderList
                    }
                }
                ChatFocus::CustomUrl => ChatFocus::ProviderList,
                ChatFocus::Conversation => ChatFocus::Message,
                ChatFocus::Message => {
                    if is_custom {
                        ChatFocus::CustomUrl
                    } else {
                        ChatFocus::Token
                    }
                }
            };
        }
        // Conversation scroll: PageUp / PageDown always, Alt+↑/↓ always
        KeyCode::PageUp => {
            app.chat_scroll = app.chat_scroll.saturating_sub(5);
            app.chat_scroll_manual = true;
        }
        KeyCode::PageDown => {
            app.chat_scroll = app.chat_scroll.saturating_add(5);
            // if user scrolled back to bottom, disable manual mode
            // (exact bottom check happens in draw_chat; here just keep manual=true)
            app.chat_scroll_manual = true;
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
            app.chat_scroll = app.chat_scroll.saturating_sub(1);
            app.chat_scroll_manual = true;
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
            app.chat_scroll = app.chat_scroll.saturating_add(1);
            app.chat_scroll_manual = true;
        }
        KeyCode::End => {
            // Jump to bottom and re-enable auto-scroll
            app.chat_scroll_manual = false;
        }
        KeyCode::Up => match app.chat_focus {
            ChatFocus::Conversation => {
                app.chat_scroll = app.chat_scroll.saturating_sub(3);
                app.chat_scroll_manual = true;
            }
            ChatFocus::ProviderList => {
                let i = app.provider_index.saturating_sub(1);
                app.provider_index = i;
                app.provider_list_state.select(Some(i));
                // reset model list when provider changes
                app.ollama_models.clear();
                app.model_input.clear();
                app.model_list_state.select(Some(0));
            }
            ChatFocus::ModelList => {
                let max = app.ollama_models.len().saturating_sub(1);
                let i = app.model_list_state.selected().unwrap_or(0).saturating_sub(1);
                let i = i.min(max);
                app.model_list_state.select(Some(i));
                if let Some(m) = app.ollama_models.get(i) {
                    app.model_input = m.clone();
                }
            }
            ChatFocus::Message => { handle_text_input_key(&mut app.message_input, key, app.input_inner_width); }
            _ => {}
        },
        KeyCode::Down => match app.chat_focus {
            ChatFocus::Conversation => {
                app.chat_scroll = app.chat_scroll.saturating_add(3);
                app.chat_scroll_manual = true;
            }
            ChatFocus::ProviderList => {
                let i = (app.provider_index + 1).min(Provider::all().len() - 1);
                app.provider_index = i;
                app.provider_list_state.select(Some(i));
                // reset model list when provider changes
                app.ollama_models.clear();
                app.model_input.clear();
                app.model_list_state.select(Some(0));
            }
            ChatFocus::ModelList => {
                let max = app.ollama_models.len().saturating_sub(1);
                let i = (app.model_list_state.selected().unwrap_or(0) + 1).min(max);
                app.model_list_state.select(Some(i));
                if let Some(m) = app.ollama_models.get(i) {
                    app.model_input = m.clone();
                }
            }
            ChatFocus::Message => { handle_text_input_key(&mut app.message_input, key, app.input_inner_width); }
            _ => {}
        },
        // Shift+Enter inserts a newline. On terminals without kitty protocol,
        // Shift+Enter arrives as plain Enter with no modifiers, so we also
        // accept Ctrl+J (ASCII LF) as a universal alternative.
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
            if app.chat_focus == ChatFocus::Message {
                app.message_input.insert_newline();
            }
        }
        KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.chat_focus == ChatFocus::Message {
                app.message_input.insert_newline();
            }
        }
        KeyCode::Enter => match app.chat_focus {
            ChatFocus::ProviderList => {
                // confirm provider; if Ollama fetch models
                if app.selected_provider() == Provider::Ollama {
                    fetch_ollama_models(app).await;
                    app.chat_focus = ChatFocus::ModelList;
                } else {
                    app.chat_focus = ChatFocus::Message;
                }
            }
            ChatFocus::ModelList => {
                // confirm model selection, move to message
                app.chat_focus = ChatFocus::Message;
            }
            ChatFocus::Message => {
                if app.streaming { return Ok(()); }
                let msg = app.message_input_text();
                let msg = msg.trim().to_string();
                if !msg.is_empty() {
                    app.messages.push(("user".to_string(), msg.clone()));
                    app.message_input = TextInput::new();
                    app.input_scroll = 0;
                    app.status = "Streaming…".to_string();
                    app.streaming = true;
                    app.chat_scroll_manual = false;
                    app.chat_scroll = 0;
                    // Spawn streaming task; sends tokens via channel
                    let provider = app.selected_provider();
                    let token = app.api_token.clone();
                    let custom_url = app.custom_url.clone();
                    let model = app.active_model();
                    let system_prompt = app.prompt_content.clone();
                    let history: Vec<(String, String)> = app.messages.clone();
                    let tx = stream_tx.clone();
                    let cancel = app.cancel_token.clone();
                    let agent_mode = app.agent_mode;
                    tokio::spawn(async move {
                        if agent_mode {
                            let agent = ReActAgent::new(
                                provider, token, custom_url, model, system_prompt,
                            );
                            // last message is the user task
                            let task = history.last().map(|(_, c)| c.clone()).unwrap_or_default();
                            if let Err(e) = agent.run(task, tx.clone()).await {
                                let _ = tx.send(Some(format!("\n[Agent error: {e}]")));
                                let _ = tx.send(None);
                            }
                        } else if let Err(e) = stream_message(StreamRequest {
                            provider, api_token: token, custom_url, model, system_prompt, history,
                            tx: tx.clone(), cancel,
                        }).await {
                            let _ = tx.send(Some(format!("\n[Error: {e}]")));
                            let _ = tx.send(None);
                        }
                    });
                }
            }
            _ => {}
        },
        KeyCode::Backspace => match app.chat_focus {
            ChatFocus::Token => { app.api_token.pop(); }
            ChatFocus::CustomUrl => { app.custom_url.pop(); }
            ChatFocus::Message => { handle_text_input_key(&mut app.message_input, key, app.input_inner_width); }
            _ => {}
        },
        KeyCode::Char(c) => {
            let ctrl_c = key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c';
            let cmd_c  = key.modifiers.contains(KeyModifiers::SUPER)   && c == 'c';
            // On macOS the standard copy binding is Cmd+C; on Windows/Linux it is Ctrl+C.
            let is_copy = if cfg!(target_os = "macos") { cmd_c } else { ctrl_c };
            // Ctrl+C always navigates back to menu on all platforms when no selection exists.
            if is_copy || ctrl_c {
                // If there is an active selection, copy it; otherwise go back to menu (Ctrl+C only)
                if let (Some(first_line), Some(last_line)) = (app.sel_start, app.sel_end) {
                    let (first_line, last_line) = if first_line <= last_line {
                        (first_line, last_line)
                    } else {
                        (last_line, first_line)
                    };
                    let r = app.conv_rect;
                    let conv_inner_width = r.width.saturating_sub(3) as usize;
                    let mut rendered: Vec<String> = Vec::new();
                    for (role, content) in &app.messages {
                        let label = if role == "user" { "You" } else { "QA-Bot" };
                        rendered.push(format!(" {label}: "));
                        let display = if role == "assistant" {
                            std::borrow::Cow::Owned(strip_model_tags(content))
                        } else {
                            std::borrow::Cow::Borrowed(content.as_str())
                        };
                        for line in display.lines() {
                            let text = format!("   {line}");
                            if conv_inner_width > 0 && text.chars().count() > conv_inner_width {
                                let chars: Vec<char> = text.chars().collect();
                                for chunk in chars.chunks(conv_inner_width) {
                                    rendered.push(chunk.iter().collect());
                                }
                            } else {
                                rendered.push(text);
                            }
                        }
                        rendered.push(String::new());
                    }
                    let selected: Vec<&str> = rendered.iter().enumerate()
                        .filter(|(i, _)| *i >= first_line && *i <= last_line)
                        .map(|(_, l)| l.as_str())
                        .collect();
                    if !selected.is_empty() {
                        let text = selected.join("\n");
                        if let Ok(mut cb) = Clipboard::new() {
                            let _ = cb.set_text(text);
                            app.status = "📋 Copied to clipboard".to_string();
                        }
                    }
                    app.sel_start = None;
                    app.sel_end = None;
                } else if ctrl_c {
                    app.screen = Screen::Menu;
                    return Ok(());
                }
                return Ok(());
            }
            match app.chat_focus {
                ChatFocus::Token => {
                    app.api_token.push(c);
                    // Save token on every keystroke
                    let _ = save_api_token(&app.api_token);
                    app.api_token_saved = true;
                    app.token_saved_at = Some(Instant::now());
                    app.status = "✓ API token saved".to_string();
                }
                ChatFocus::CustomUrl => app.custom_url.push(c),
                ChatFocus::Message => { handle_text_input_key(&mut app.message_input, key, app.input_inner_width); }
                _ => {}
            }
        }
        _ => {
            // Forward any other key events to the message TextInput when focused
            if app.chat_focus == ChatFocus::Message {
                handle_text_input_key(&mut app.message_input, key, app.input_inner_width);
            }
        }
    }
    Ok(())
}

