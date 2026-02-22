// Event handler module
// Handles keyboard and mouse events, delegating state changes to StateManager

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::agent::ReActAgent;
use crate::tui::api::{fetch_ollama_models, stream_message, StreamRequest};
use crate::tui::input::handle_text_input_key;
use crate::tui::providers::Provider;
use crate::tui::state_manager::StateManager;
use crate::{App, ChatFocus, Screen};
use arboard::Clipboard;

pub async fn handle_menu_key(
    app: &mut App,
    key: &KeyEvent,
    state_manager: &mut StateManager,
) -> Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()), // Signal to exit
        KeyCode::Up => {
            let i = app.menu_state.selected().unwrap_or(0);
            app.menu_state.select(Some(i.saturating_sub(1)));
        }
        KeyCode::Down => {
            let i = app.menu_state.selected().unwrap_or(0);
            app.menu_state.select(Some((i + 1).min(crate::tui::state::MENU_ITEMS.len() - 1)));
        }
        KeyCode::Enter => {
            let i = app.menu_state.selected().unwrap_or(0);
            match i {
                0 => state_manager.navigate_to_info(),
                1 => state_manager.navigate_to_show(),
                2 => state_manager.navigate_to_validate(),
                3 => state_manager.navigate_to_tools(),
                4 => state_manager.navigate_to_chat(),
                5 => return Ok(()), // Quit
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}

pub async fn handle_info_key(
    key: &KeyEvent,
    state_manager: &mut StateManager,
) -> Result<()> {
    if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
        state_manager.navigate_to_menu();
    }
    Ok(())
}

pub async fn handle_show_key(
    app: &mut App,
    key: &KeyEvent,
    state_manager: &mut StateManager,
) -> Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => state_manager.navigate_to_menu(),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_offset += 1,
        KeyCode::Up | KeyCode::Char('k') => {
            app.scroll_offset = app.scroll_offset.saturating_sub(1)
        }
        _ => {}
    }
    Ok(())
}

pub async fn handle_tools_key(
    app: &mut App,
    key: &KeyEvent,
    state_manager: &mut StateManager,
) -> Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => state_manager.navigate_to_menu(),
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
            state_manager.navigate_to_chat();
        }
        _ => {}
    }
    Ok(())
}

pub async fn handle_chat_key(
    app: &mut App,
    key: &KeyEvent,
    stream_tx: mpsc::UnboundedSender<Option<String>>,
    state_manager: &mut StateManager,
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
                app.last_esc_at = Some(std::time::Instant::now());
                app.status = "Press ESC again to stop inference".to_string();
            } else {
                state_manager.navigate_to_menu();
            }
        }
        KeyCode::F(2) => {
            state_manager.toggle_agent_mode();
        }
        KeyCode::Tab => {
            state_manager.cycle_chat_focus(true);
        }
        KeyCode::BackTab => {
            state_manager.cycle_chat_focus(false);
        }
        KeyCode::PageUp => {
            state_manager.page_up();
        }
        KeyCode::PageDown => {
            state_manager.page_down();
        }
        KeyCode::End => {
            // Jump to bottom and re-enable auto-scroll
            app.chat_scroll_manual = false;
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
            state_manager.scroll_up();
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
            state_manager.scroll_down();
        }
        KeyCode::Up => match app.chat_focus {
            ChatFocus::Conversation => {
                state_manager.scroll_up();
            }
            ChatFocus::ProviderList => {
                state_manager.select_previous_provider();
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
            ChatFocus::Message => { handle_text_input_key(&mut app.message_input, *key, app.input_inner_width); }
            _ => {}
        },
        KeyCode::Down => match app.chat_focus {
            ChatFocus::Conversation => {
                state_manager.scroll_down();
            }
            ChatFocus::ProviderList => {
                state_manager.select_next_provider();
            }
            ChatFocus::ModelList => {
                let max = app.ollama_models.len().saturating_sub(1);
                let i = (app.model_list_state.selected().unwrap_or(0) + 1).min(max);
                app.model_list_state.select(Some(i));
                if let Some(m) = app.ollama_models.get(i) {
                    app.model_input = m.clone();
                }
            }
            ChatFocus::Message => { handle_text_input_key(&mut app.message_input, *key, app.input_inner_width); }
            _ => {}
        },
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
            ChatFocus::CustomUrl => {
                // confirm custom URL; for Ollama, fetch models from the new server
                if app.selected_provider() == Provider::Ollama {
                    fetch_ollama_models(app).await;
                    app.chat_focus = ChatFocus::ModelList;
                } else {
                    app.chat_focus = ChatFocus::Token;
                }
            }
            ChatFocus::Message => {
                if app.streaming { return Ok(()); }
                let msg = app.message_input_text();
                let msg = msg.trim().to_string();
                if !msg.is_empty() {
                    state_manager.add_user_message(msg.clone());

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
                            // last message is the current user task; pass full history for memory
                            let task = history.last().map(|(_, c)| c.clone()).unwrap_or_default();
                            let prior = history[..history.len().saturating_sub(1)].to_vec();
                            if let Err(e) = agent.run(task, prior, tx.clone()).await {
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
            ChatFocus::Token => { state_manager.remove_token_char(); }
            ChatFocus::CustomUrl => { state_manager.remove_url_char(); }
            ChatFocus::Message => { handle_text_input_key(&mut app.message_input, *key, app.input_inner_width); }
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
                            std::borrow::Cow::Owned(crate::tui::util::strip_model_tags(content))
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
                    state_manager.navigate_to_menu();
                    return Ok(());
                }
                return Ok(());
            }
            match app.chat_focus {
                ChatFocus::Token => {
                    state_manager.add_token_char(c);
                }
                ChatFocus::CustomUrl => state_manager.add_url_char(c),
                ChatFocus::Message => { handle_text_input_key(&mut app.message_input, *key, app.input_inner_width); }
                _ => {}
            }
        }
        _ => {
            // Forward any other key events to the message TextInput when focused
            if app.chat_focus == ChatFocus::Message {
                handle_text_input_key(&mut app.message_input, *key, app.input_inner_width);
            }
        }
    }
    Ok(())
}

pub async fn handle_validate_key(
    key: &KeyEvent,
    state_manager: &mut StateManager,
) -> Result<()> {
    if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
        state_manager.navigate_to_menu();
    }
    Ok(())
}

pub async fn handle_mouse_event(
    app: &mut App,
    mouse: &MouseEvent,
    state_manager: &mut StateManager,
) -> Result<()> {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            state_manager.scroll_up();
        }
        MouseEventKind::ScrollDown => {
            state_manager.scroll_down();
        }
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
    Ok(())
}