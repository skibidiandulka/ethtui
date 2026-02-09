use crate::app::App;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table},
};

pub fn render(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(2)])
        .margin(1)
        .split(frame.area());

    let body = chunks[0];
    let footer = chunks[1];

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(10)])
        .split(body);

    render_devices(app, frame, body_chunks[0]);
    render_details(app, frame, body_chunks[1]);
    render_footer(app, frame, footer);

    if let Some(err) = &app.last_error {
        render_error_popup(frame, err);
    }
}

fn render_devices(app: &mut App, frame: &mut Frame, area: Rect) {
    let rows: Vec<Row> = app
        .devices
        .iter()
        .map(|d| {
            let carrier = d.carrier.map(|c| if c { "1" } else { "0" }).unwrap_or("?");
            let speed = d
                .speed_mbps
                .map(|s| format!("{s}"))
                .unwrap_or_else(|| "-".into());

            Row::new(vec![
                Cell::from(d.name.clone()),
                Cell::from(d.operstate.clone()),
                Cell::from(carrier),
                Cell::from(speed),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Length(9),
        Constraint::Length(7),
        Constraint::Length(7),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec![
                Cell::from("Iface").style(Style::default().fg(Color::Yellow)),
                Cell::from("State").style(Style::default().fg(Color::Yellow)),
                Cell::from("Carrier").style(Style::default().fg(Color::Yellow)),
                Cell::from("Speed").style(Style::default().fg(Color::Yellow)),
            ])
            .style(Style::new().bold())
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(" Ethernet ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .border_type(BorderType::Thick),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_stateful_widget(table, area, &mut app.devices_state);
}

fn render_details(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL)
        .border_style(Style::default())
        .border_type(BorderType::default());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = if let Some(d) = app.selected_device() {
        let mut lines = Vec::new();

        lines.push(Line::from(vec![
            Span::from("Interface: ").bold(),
            Span::from(d.name.clone()),
        ]));
        lines.push(Line::from(vec![
            Span::from("State: ").bold(),
            Span::from(d.operstate.clone()),
        ]));
        lines.push(Line::from(vec![
            Span::from("Carrier: ").bold(),
            Span::from(
                d.carrier
                    .map(|c| if c { "1" } else { "0" })
                    .unwrap_or("?")
                    .to_string(),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::from("Speed: ").bold(),
            Span::from(
                d.speed_mbps
                    .map(|s| format!("{s} Mb/s"))
                    .unwrap_or_else(|| "-".into()),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::from("MAC: ").bold(),
            Span::from(d.mac.clone().unwrap_or_else(|| "-".into())),
        ]));
        lines.push(Line::from(""));

        lines.push(Line::from(Span::from("IPv4: ").bold()));
        if d.ipv4.is_empty() {
            lines.push(Line::from("  -"));
        } else {
            for ip in &d.ipv4 {
                lines.push(Line::from(format!("  {ip}")));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::from("IPv6: ").bold()));
        if d.ipv6.is_empty() {
            lines.push(Line::from("  -"));
        } else {
            for ip in &d.ipv6 {
                lines.push(Line::from(format!("  {ip}")));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::from("Gateway v4: ").bold(),
            Span::from(d.gateway_v4.clone().unwrap_or_else(|| "-".into())),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::from("DNS: ").bold()));
        if d.dns.is_empty() {
            lines.push(Line::from("  -"));
        } else {
            for s in &d.dns {
                lines.push(Line::from(format!("  {s}")));
            }
        }

        Text::from(lines)
    } else {
        Text::from(vec![
            Line::from("No ethernet devices found."),
            Line::from(""),
            Line::from("This TUI only lists physical, non-wifi interfaces."),
        ])
    };

    let p = Paragraph::new(text)
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(p, inner);
}

fn render_footer(_app: &App, frame: &mut Frame, area: Rect) {
    let text = Line::from(vec![
        Span::from("j/k").bold(),
        Span::from(" move  "),
        Span::from("r").bold(),
        Span::from(" refresh  "),
        Span::from("q").bold(),
        Span::from(" quit"),
    ]);

    let p = Paragraph::new(text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(p, area);
}

fn render_error_popup(frame: &mut Frame, msg: &str) {
    let area = centered_rect(80, 40, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Error ")
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let p = Paragraph::new(msg)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(p, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
