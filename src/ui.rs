use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, RoomMode};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(3), Constraint::Min(9), Constraint::Length(3), Constraint::Length(3),
    ]).split(area);
    header(frame, app, chunks[0]);
    body(frame, app, chunks[1]);
    frame.render_widget(Paragraph::new(app.notice.as_str()).wrap(Wrap { trim: true }).block(Block::default().title(" Notice ").borders(Borders::ALL)), chunks[2]);
    frame.render_widget(Paragraph::new(format!("> {}", app.input)).block(Block::default().title(" Message / command ").borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan))), chunks[3]);
    frame.set_cursor_position((chunks[3].x + (app.input.chars().count() as u16 + 2).min(chunks[3].width.saturating_sub(2)), chunks[3].y + 1));
}

fn header(frame: &mut Frame, app: &App, area: Rect) {
    let mode = match app.mode { RoomMode::Host => "HOST", RoomMode::Guest => "GUEST" };
    let status = if app.connected { "CONNECTED" } else { "SETUP" };
    let title = Line::from(vec![Span::styled(" SOUNDLANE ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)), Span::raw(format!("  {mode}  ·  {status}  ·  {}", app.source))]);
    frame.render_widget(Paragraph::new(title).block(Block::default().borders(Borders::ALL)), area);
}

fn body(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(68), Constraint::Percentage(32)]).split(area);
    let chat = if app.chat.is_empty() { vec![ListItem::new(Span::styled("No messages yet — type below to chat.", Style::default().fg(Color::DarkGray)))] }
        else { app.chat.iter().map(|line| ListItem::new(Line::from(vec![Span::styled(format!("{}: ", line.from), if line.own { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::Green) }), Span::raw(&line.text)])).collect() };
    frame.render_widget(List::new(chat).block(Block::default().title(" Room chat ").borders(Borders::ALL)), columns[0]);
    let mut info = vec![Line::from(vec![Span::styled("Source: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&app.source)]), Line::from(format!("Codec: Opus · {} kb/s", app.bitrate)), Line::from(format!("Uptime: {}s", app.elapsed().as_secs())), Line::from(""), Line::from(Span::styled("Participants", Style::default().add_modifier(Modifier::BOLD)))];
    if app.peers.is_empty() { info.push(Line::from(Span::styled("Waiting for a peer", Style::default().fg(Color::DarkGray)))); }
    for peer in &app.peers { info.push(Line::from(format!("● {}  {}", peer.name, peer.rtt_ms.map(|ms| format!("{ms:.0} ms")).unwrap_or_else(|| "—".into())))); }
    if app.mode == RoomMode::Host { info.push(Line::from("")); info.push(Line::from(Span::styled("/invite to show code", Style::default().fg(Color::Yellow)))); }
    frame.render_widget(Paragraph::new(Text::from(info)).wrap(Wrap { trim: true }).block(Block::default().title(" Session ").borders(Borders::ALL)), columns[1]);
}
