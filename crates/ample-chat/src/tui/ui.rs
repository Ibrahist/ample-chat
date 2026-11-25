use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::App;

/// Renders the full TUI layout:
/// - Top header bar (Title, TLS security status, connection state)
/// - Center: Left messages pane + Right online users pane
/// - Bottom: Input text box with live cursor and help instructions
pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Messages & Users
            Constraint::Length(3), // Input box
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);
    render_body(f, app, chunks[1]);
    render_input(f, app, chunks[2]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let tls_badge = if app.is_tls {
        Span::styled(
            " [TLS SECURE] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [PLAINTEXT TRANSPORT] ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    };

    let status_badge = if app.connected {
        Span::styled(" ● Connected ", Style::default().fg(Color::Green))
    } else {
        Span::styled(
            format!(" ○ {} ", app.status),
            Style::default().fg(Color::Red),
        )
    };

    let title_line = Line::from(vec![
        Span::styled(
            " ample-chat ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("─ "),
        tls_badge,
        Span::raw("─ "),
        status_badge,
        Span::raw(format!("─ User: @{}", app.username)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title_line)
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(block, area);
}

fn render_body(f: &mut Frame, app: &App, area: Rect) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(80), // Messages
            Constraint::Percentage(20), // Users
        ])
        .split(area);

    render_messages(f, app, body_chunks[0]);
    render_users(f, app, body_chunks[1]);
}

fn render_messages(f: &mut Frame, app: &App, area: Rect) {
    // Auto-scroll to show latest messages
    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll = if app.messages.len() > visible_height {
        (app.messages.len() - visible_height) as u16
    } else {
        0
    };

    let messages_widget = Paragraph::new(
        app.messages
            .iter()
            .map(|msg| {
                let time_short = if msg.timestamp.len() >= 19 {
                    &msg.timestamp[11..19]
                } else {
                    &msg.timestamp
                };

                let sender_style = if msg.is_self {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                };

                let content_span = if msg.failed {
                    Span::styled(
                        &msg.content,
                        Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::ITALIC),
                    )
                } else {
                    Span::styled(&msg.content, Style::default().fg(Color::White))
                };

                Line::from(vec![
                    Span::styled(
                        format!("[{time_short}] "),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(format!("{}: ", msg.sender), sender_style),
                    content_span,
                ])
            })
            .collect::<Vec<_>>(),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Messages ({}) ", app.messages.len()))
            .style(Style::default().fg(Color::DarkGray)),
    )
    .scroll((scroll, 0))
    .wrap(Wrap { trim: false });

    f.render_widget(messages_widget, area);
}

fn render_users(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .users
        .iter()
        .map(|u| {
            let is_self = u.username.eq_ignore_ascii_case(&app.username);
            let style = if is_self {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(vec![
                Span::styled(" • ", Style::default().fg(Color::Green)),
                Span::styled(&u.username, style),
            ]))
        })
        .collect();

    let users_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Online ({}) ", app.users.len()))
            .style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(users_list, area);
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Send Message (Enter to send • Esc to exit) ")
                .style(Style::default().fg(Color::Cyan)),
        );

    f.render_widget(input, area);

    // Place cursor in input box
    f.set_cursor_position((area.x + app.cursor_pos as u16 + 1, area.y + 1));
}
