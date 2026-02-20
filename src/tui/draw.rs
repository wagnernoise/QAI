use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame, Terminal,
};
use crate::tui::state::{App, Screen, ChatFocus, MENU_ITEMS};
use crate::tui::providers::Provider;
use crate::tui::util::strip_model_tags;

// ── Drawing ───────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Background
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(15, 15, 25))),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    draw_header(f, chunks[0]);

    match app.screen {
        Screen::Menu => draw_menu(f, chunks[1], app),
        Screen::Info => draw_info(f, chunks[1], app),
        Screen::Show => draw_show(f, chunks[1], app),
        Screen::Validate => draw_validate(f, chunks[1], app),
        Screen::Tools => draw_tools(f, chunks[1], app),
        Screen::Chat => draw_chat(f, chunks[1], app),
    }

    draw_footer(f, chunks[2], app);
}

fn draw_header(f: &mut Frame, area: Rect) {
    let banner = vec![
        Line::from(vec![
            Span::styled("  ██████╗  █████╗ ██╗", Style::default().fg(Color::Cyan)),
            Span::styled("  ", Style::default()),
            Span::styled("QA Automation AI Agent", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(" ██╔═══██╗██╔══██╗██║", Style::default().fg(Color::Cyan)),
            Span::styled("  v", Style::default().fg(Color::DarkGray)),
            Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(Span::styled(" ██║   ██║███████║██║", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled(" ██║▄▄ ██║██╔══██║██║", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled(" ╚██████╔╝██║  ██║██║", Style::default().fg(Color::Cyan))),
    ];

    let header = Paragraph::new(banner)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .alignment(Alignment::Left);
    f.render_widget(header, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let hint = match &app.screen {
        Screen::Menu => " ↑↓ Navigate   Enter Select   q Quit ",
        Screen::Show => " ↑↓/j/k Scroll   q/Esc Back ",
        Screen::Chat => " Tab Next field   Enter Send   Esc Back ",
        _ => " q/Esc Back ",
    };
    let footer = Paragraph::new(hint)
        .style(Style::default().fg(Color::DarkGray).bg(Color::Rgb(15, 15, 25)))
        .alignment(Alignment::Center);
    f.render_widget(footer, area);
}

fn draw_menu(f: &mut Frame, area: Rect, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let icon = match i {
                0 => "  ",
                1 => "  ",
                2 => "  ",
                3 => "  ",
                4 => "  ",
                5 => "  ",
                _ => "  ",
            };
            ListItem::new(Line::from(vec![
                Span::styled(icon, Style::default().fg(Color::Cyan)),
                Span::raw(*label),
            ]))
        })
        .collect();

    let mut state = app.menu_state;
    let list = List::new(items)
        .block(
            Block::default()
                .title(" Menu ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, outer[0], &mut state);

    // Right panel: welcome text
    let welcome = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Welcome to QAI",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  QA Automation AI Agent manager.",
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Use the menu to inspect, validate,",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  or chat with the QA-Bot via API.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
    )
    .wrap(Wrap { trim: false });
    f.render_widget(welcome, outer[1]);
}

fn draw_info(f: &mut Frame, area: Rect, app: &App) {
    let exists = app.prompt_path.exists();
    let size = std::fs::metadata(&app.prompt_path)
        .map(|m| format!("{} bytes", m.len()))
        .unwrap_or_else(|_| "N/A".to_string());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Prompt path : ", Style::default().fg(Color::DarkGray)),
            Span::styled(app.prompt_path.display().to_string(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  Exists      : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if exists { "yes" } else { "no" },
                Style::default().fg(if exists { Color::Green } else { Color::Red }),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Size        : ", Style::default().fg(Color::DarkGray)),
            Span::styled(size, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Version     : ", Style::default().fg(Color::DarkGray)),
            Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  README      : ", Style::default().fg(Color::DarkGray)),
            Span::styled("README.md", Style::default().fg(Color::White)),
        ]),
    ];

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Info ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn draw_show(f: &mut Frame, area: Rect, app: &App) {
    let p = Paragraph::new(app.prompt_content.as_str())
        .block(
            Block::default()
                .title(" System Prompt ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));
    f.render_widget(p, area);
}

fn draw_validate(f: &mut Frame, area: Rect, app: &App) {
    let required = ["## ENVIRONMENT", "### PRIMARY OBJECTIVE", "### MODE SELECTION PRIMER"];
    let mut lines = vec![Line::from("")];
    let mut all_ok = true;
    for marker in required {
        let found = app.prompt_content.contains(marker);
        if !found { all_ok = false; }
        lines.push(Line::from(vec![
            Span::styled(
                if found { "  ✔ " } else { "  ✘ " },
                Style::default().fg(if found { Color::Green } else { Color::Red }),
            ),
            Span::raw(marker),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        if all_ok { "  Validation passed." } else { "  Validation failed." },
        Style::default()
            .fg(if all_ok { Color::Green } else { Color::Red })
            .add_modifier(Modifier::BOLD),
    )));

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Validate ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn draw_tools(f: &mut Frame, area: Rect, app: &mut App) {
    let providers = Provider::all();

    // Layout: left list | right detail
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // ── Left: provider list ───────────────────────────────────────────────────
    let items: Vec<ListItem> = providers
        .iter()
        .map(|p| ListItem::new(p.label()))
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .title(" AI Providers ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, cols[0], &mut app.tools_provider_list_state);

    // ── Right: detail panel ───────────────────────────────────────────────────
    let selected = &providers[app.tools_provider_index];
    let detail_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", selected.label()),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Info       : ", Style::default().fg(Color::DarkGray)),
            Span::styled(selected.description(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  API URL    : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if selected.api_url().is_empty() { "(enter custom URL in Chat screen)" } else { selected.api_url() },
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Model      : ", Style::default().fg(Color::DarkGray)),
            Span::styled(selected.default_model(), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Enter to open Chat with this provider selected.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  ↑/↓ or j/k: navigate   Enter: open Chat   q/Esc: back",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let detail = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .title(" Provider Details ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(detail, cols[1]);
}

fn draw_chat(f: &mut Frame, area: Rect, app: &mut App) {
    let providers = Provider::all();
    let is_custom = app.selected_provider() == Provider::Custom;
    let is_ollama = app.selected_provider() == Provider::Ollama;

    // Layout: left sidebar (config) | right (conversation)
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    // ── Left: config panel ────────────────────────────────────────────────────
    // Rows: provider list | model list (Ollama only) | custom url (Custom only) | token | model display
    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),                                      // provider list
            Constraint::Length(if is_ollama { 6 } else { 0 }),       // model list (Ollama)
            Constraint::Length(if is_custom { 3 } else { 0 }),       // custom url
            Constraint::Length(3),                                   // token
            Constraint::Length(3),                                   // active model display
        ])
        .split(cols[0]);

    // Provider list
    let provider_focused = app.chat_focus == ChatFocus::ProviderList;
    let provider_items: Vec<ListItem> = providers
        .iter()
        .map(|p| ListItem::new(p.label()))
        .collect();
    let mut pstate = app.provider_list_state;
    let provider_list = List::new(provider_items)
        .block(
            Block::default()
                .title(" Provider (↑/↓ Enter) ")
                .title_style(Style::default().fg(if provider_focused { Color::Yellow } else { Color::DarkGray }))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if provider_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) })),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(provider_list, left_rows[0], &mut pstate);

    // Ollama model list
    if is_ollama {
        let model_focused = app.chat_focus == ChatFocus::ModelList;
        if app.ollama_models.is_empty() {
            let hint = Paragraph::new(Span::styled(
                " Press Enter or Tab to fetch models",
                Style::default().fg(Color::DarkGray),
            ))
            .block(
                Block::default()
                    .title(" Model (↑/↓ Enter) ")
                    .title_style(Style::default().fg(if model_focused { Color::Yellow } else { Color::DarkGray }))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if model_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) })),
            );
            f.render_widget(hint, left_rows[1]);
        } else {
            let model_items: Vec<ListItem> = app
                .ollama_models
                .iter()
                .map(|m| ListItem::new(m.as_str()))
                .collect();
            let mut mstate = app.model_list_state;
            let model_list = List::new(model_items)
                .block(
                    Block::default()
                        .title(" Model (↑/↓ Enter) ")
                        .title_style(Style::default().fg(if model_focused { Color::Yellow } else { Color::DarkGray }))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(if model_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) })),
                )
                .highlight_style(Style::default().fg(Color::Black).bg(Color::Green))
                .highlight_symbol("▶ ");
            f.render_stateful_widget(model_list, left_rows[1], &mut mstate);
        }
    }

    // Custom URL field
    if is_custom {
        let url_focused = app.chat_focus == ChatFocus::CustomUrl;
        let url_block = Block::default()
            .title(" Endpoint URL ")
            .title_style(Style::default().fg(if url_focused { Color::Yellow } else { Color::DarkGray }))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if url_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) }));
        let url_p = Paragraph::new(app.custom_url.as_str())
            .block(url_block)
            .style(Style::default().fg(Color::White));
        f.render_widget(url_p, left_rows[2]);
    }

    // Token field
    let token_display: String = if app.api_token.is_empty() {
        String::new()
    } else {
        "•".repeat(app.api_token.len().min(20))
    };
    let token_focused = app.chat_focus == ChatFocus::Token;
    let token_title = if is_ollama { " API Token (optional) " } else { " API Token " };
    let token_block = Block::default()
        .title(token_title)
        .title_style(Style::default().fg(if token_focused { Color::Yellow } else { Color::DarkGray }))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if token_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) }));
    let token_p = Paragraph::new(token_display.as_str())
        .block(token_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(token_p, left_rows[3]);

    // Active model display
    let active_model = app.active_model();
    let model_display = Paragraph::new(Span::styled(
        format!(" {active_model}"),
        Style::default().fg(Color::Green),
    ))
    .block(
        Block::default()
            .title(" Active Model ")
            .title_style(Style::default().fg(Color::DarkGray))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
    );
    f.render_widget(model_display, left_rows[4]);

    // ── Right: conversation + input ───────────────────────────────────────────
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(8), Constraint::Length(1)])
        .split(cols[1]);

    // Conversation history
    let mut conv_lines: Vec<Line> = Vec::new();
    for (role, content) in &app.messages {
        let (label, color) = if role == "user" {
            ("You", Color::Cyan)
        } else {
            ("QA-Bot", Color::Green)
        };
        conv_lines.push(Line::from(Span::styled(
            format!(" {label}: "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
        let display_content = if role == "assistant" {
            std::borrow::Cow::Owned(strip_model_tags(content))
        } else {
            std::borrow::Cow::Borrowed(content.as_str())
        };
        for line in display_content.lines() {
            conv_lines.push(Line::from(Span::styled(
                format!("   {line}"),
                Style::default().fg(Color::White),
            )));
        }
        conv_lines.push(Line::from(""));
    }
    if !app.status.is_empty() {
        conv_lines.push(Line::from(Span::styled(
            format!(" {}", app.status),
            Style::default().fg(Color::Yellow),
        )));
    }

    // Scroll logic: manual overrides auto-scroll to bottom
    let conv_area_height = right_rows[0].height.saturating_sub(2) as usize; // subtract borders
    // The available width for text inside the bordered Paragraph (subtract 2 for borders, 1 for scrollbar)
    let conv_inner_width = right_rows[0].width.saturating_sub(3) as usize;
    // Count rendered lines accounting for word-wrap: each Line whose text width exceeds
    // conv_inner_width wraps into ceil(width / inner_width) rendered rows.
    let total_lines: usize = conv_lines.iter().map(|line| {
        let text_width: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
        if conv_inner_width == 0 || text_width == 0 {
            1
        } else {
            text_width.div_ceil(conv_inner_width)
        }
    }).sum::<usize>().max(1);
    let max_scroll = if total_lines > conv_area_height {
        (total_lines - conv_area_height) as u16
    } else {
        0
    };
    let effective_scroll = if app.chat_scroll_manual {
        app.chat_scroll.min(max_scroll)
    } else {
        max_scroll
    };

    // Apply selection highlight — sel_start/sel_end are content line indices (scroll-independent)
    if let (Some(s), Some(e)) = (app.sel_start, app.sel_end) {
        let (first_sel, last_sel) = if s <= e { (s, e) } else { (e, s) };
        // Walk rendered lines (accounting for wrap) to find which logical indices to highlight
        let mut rendered_idx: usize = 0;
        for line in conv_lines.iter_mut() {
            let text_width: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
            let wrapped = if conv_inner_width == 0 || text_width == 0 { 1 } else { text_width.div_ceil(conv_inner_width) };
            for _ in 0..wrapped {
                if rendered_idx >= first_sel && rendered_idx <= last_sel {
                    for span in line.spans.iter_mut() {
                        span.style = span.style.bg(Color::Rgb(60, 80, 120));
                    }
                }
                rendered_idx += 1;
            }
        }
    }

    let conv_focused = app.chat_focus == ChatFocus::Conversation;
    let conv_title = if app.chat_scroll_manual {
        " Conversation  [↑/↓ scroll — End to resume auto-scroll] "
    } else if conv_focused {
        " Conversation  [focused — ↑/↓ to scroll] "
    } else {
        " Conversation  [Tab to focus] "
    };

    let conv = Paragraph::new(conv_lines)
        .block(
            Block::default()
                .title(conv_title)
                .title_style(Style::default().fg(if conv_focused { Color::Yellow } else { Color::Cyan }).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if conv_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) })),
        )
        .wrap(Wrap { trim: false })
        .scroll((effective_scroll, 0));
    app.conv_rect = right_rows[0];
    app.conv_max_scroll = max_scroll;
    f.render_widget(conv, right_rows[0]);

    // Scrollbar
    if total_lines > conv_area_height {
        let mut scrollbar_state = ScrollbarState::new(max_scroll as usize)
            .position(effective_scroll as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        f.render_stateful_widget(scrollbar, right_rows[0], &mut scrollbar_state);
    }

    // Message input with visible cursor
    let msg_focused = app.chat_focus == ChatFocus::Message;
    let (before, cursor_ch, after) = app.message_input.split_at_cursor();
    let cursor_style = if msg_focused {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    // Build multi-line input: split text into logical lines, inserting cursor marker in the right line
    let input_inner_width = right_rows[1].width.saturating_sub(3) as usize; // -2 borders -1 scrollbar
    app.input_inner_width = input_inner_width.max(1);
    let input_inner_height = right_rows[1].height.saturating_sub(2) as usize;

    // Build lines from the full text, placing cursor highlight at the correct position
    let full_before = before.to_string();
    let full_after = after.to_string();
    let cursor_char = cursor_ch.to_string();

    // Split before+cursor+after into logical lines (by '\n'), then each logical line
    // further wraps into rendered rows of `input_inner_width` chars.
    let combined = format!("{full_before}\x00{cursor_char}\x00{full_after}");
    let logical_lines: Vec<&str> = combined.split('\n').collect();
    let mut input_lines: Vec<Line> = Vec::new();
    for logical in &logical_lines {
        // Find cursor marker positions
        if let Some(c0) = logical.find('\x00') {
            let rest = &logical[c0 + 1..];
            if let Some(c1) = rest.find('\x00') {
                let seg_before = &logical[..c0];
                let seg_cursor = &rest[..c1];
                let seg_after = &rest[c1 + 1..];
                input_lines.push(Line::from(vec![
                    Span::styled(seg_before.to_string(), Style::default().fg(Color::White)),
                    Span::styled(seg_cursor.to_string(), cursor_style),
                    Span::styled(seg_after.to_string(), Style::default().fg(Color::White)),
                ]));
            } else {
                input_lines.push(Line::from(Span::styled(logical.replace('\x00', ""), Style::default().fg(Color::White))));
            }
        } else {
            input_lines.push(Line::from(Span::styled(logical.to_string(), Style::default().fg(Color::White))));
        }
    }

    // Count total rendered rows (accounting for word-wrap)
    let input_total_rows: usize = input_lines.iter().map(|line| {
        let w: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
        if input_inner_width == 0 || w == 0 { 1 } else { w.div_ceil(input_inner_width) }
    }).sum::<usize>().max(1);
    let input_max_scroll = if input_total_rows > input_inner_height {
        (input_total_rows - input_inner_height) as u16
    } else {
        0
    };

    // Compute cursor row for auto-scroll
    let cursor_row = app.message_input.cursor_row(input_inner_width);
    if (cursor_row as usize) < app.input_scroll as usize {
        app.input_scroll = cursor_row;
    } else if input_inner_height > 0 && (cursor_row as usize) >= app.input_scroll as usize + input_inner_height {
        app.input_scroll = cursor_row.saturating_sub((input_inner_height as u16).saturating_sub(1));
    }
    let effective_input_scroll = app.input_scroll.min(input_max_scroll);

    let input_widget = Paragraph::new(input_lines)
        .block(
            Block::default()
                .title(" Message  [Shift+Enter or Ctrl+J: newline] ")
                .title_style(Style::default().fg(if msg_focused { Color::Yellow } else { Color::DarkGray }))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if msg_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) })),
        )
        .wrap(Wrap { trim: false })
        .scroll((effective_input_scroll, 0));
    f.render_widget(input_widget, right_rows[1]);
    // Store input rect and max_scroll for mouse hit-testing
    app.input_rect = right_rows[1];
    app.input_max_scroll_stored = input_max_scroll;

    // Input scrollbar
    if input_total_rows > input_inner_height {
        let mut input_scrollbar_state = ScrollbarState::new(input_max_scroll as usize)
            .position(effective_input_scroll as usize);
        let input_scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        f.render_stateful_widget(input_scrollbar, right_rows[1], &mut input_scrollbar_state);
    }

    // Cursor hint
    let hint = Paragraph::new(Span::styled(
        " Tab: cycle focus   ↑/↓: scroll/navigate   Enter: select/send   Shift+Enter or Ctrl+J: newline   End: auto-scroll   Esc: menu ",
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(hint, right_rows[2]);
}

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Render the current app state into an in-memory buffer using `TestBackend`.
/// Useful for unit tests that need to assert on rendered output without a real terminal.
pub fn render_to_buffer(app: &mut App, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    terminal.draw(|f| draw(f, app)).expect("draw");
    terminal.backend().buffer().clone()
}

