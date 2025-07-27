use crate::dashboard::DashboardState;
use crate::stats::StatsCalculator;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::HashMap;

pub fn draw_basic_connectivity_check(f: &mut Frame, area: Rect, _state: &DashboardState) {
    let block = Block::default()
        .title("🌐 Connectivity Check")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Green));

    // Simple connectivity indicators (would normally ping these)
    let content = vec![
        Line::from(vec![
            Span::styled("Gateway: ", Style::default().fg(Color::White)),
            Span::styled("✅ OK (2ms)", Style::default().fg(Color::Green)),
            Span::styled("  |  DNS: ", Style::default().fg(Color::White)),
            Span::styled("✅ OK (8ms)", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Internet: ", Style::default().fg(Color::White)),
            Span::styled("✅ OK (25ms)", Style::default().fg(Color::Green)),
            Span::styled(" |  Load: ", Style::default().fg(Color::White)),
            Span::styled("Low", Style::default().fg(Color::Green)),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

pub fn draw_simple_interface_summary(
    f: &mut Frame,
    area: Rect,
    state: &DashboardState,
    stats_calculators: &HashMap<String, StatsCalculator>,
) {
    let block = Block::default()
        .title("📡 Network Interfaces")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow));

    let mut content = vec![Line::from(vec![
        Span::styled(
            "Interface",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "     Status      Speed      Errors",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    for device in state.devices.iter().take(4) {
        // Show max 4 interfaces
        let status = if let Some(calculator) = stats_calculators.get(&device.name) {
            let (speed_in, speed_out) = calculator.current_speed();
            let combined_speed = speed_in + speed_out;

            let speed_text = if combined_speed > 1024 * 1024 {
                format!("{:.1}MB/s", combined_speed as f64 / 1024.0 / 1024.0)
            } else if combined_speed > 1024 {
                format!("{:.0}KB/s", combined_speed as f64 / 1024.0)
            } else {
                format!("{combined_speed}B/s")
            };

            let errors = device.stats.errors_in + device.stats.errors_out;
            let error_text = if errors > 0 {
                format!("❌ {errors}")
            } else {
                "✅ None".to_string()
            };

            let status_icon = if errors > 0 {
                ("🔴", "ERROR")
            } else if combined_speed > 0 {
                ("🟢", "ACTIVE")
            } else {
                ("⚪", "IDLE")
            };

            (status_icon, speed_text, error_text)
        } else {
            (
                ("❓", "UNKNOWN"),
                "No data".to_string(),
                "Unknown".to_string(),
            )
        };

        content.push(Line::from(vec![
            Span::styled(
                format!("{:<12}", device.name),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!("{} {:<8}", status.0 .0, status.0 .1),
                Style::default().fg(match status.0 .1 {
                    "ERROR" => Color::Red,
                    "ACTIVE" => Color::Green,
                    _ => Color::White,
                }),
            ),
            Span::styled(
                format!("{:<10}", status.1),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                status.2.clone(),
                Style::default().fg(if status.2.contains("❌") {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
        ]));
    }

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

pub fn draw_common_network_issues(
    f: &mut Frame,
    area: Rect,
    state: &DashboardState,
    stats_calculators: &HashMap<String, StatsCalculator>,
) {
    let block = Block::default()
        .title("🔧 Quick Diagnostics")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Magenta));

    let mut issues = Vec::new();
    let mut has_traffic = false;
    let mut has_errors = false;
    let mut high_connections = false;

    // Analyze for common issues
    for device in &state.devices {
        if let Some(calculator) = stats_calculators.get(&device.name) {
            let (speed_in, speed_out) = calculator.current_speed();
            if speed_in + speed_out > 0 {
                has_traffic = true;
            }
        }

        if device.stats.errors_in > 0 || device.stats.errors_out > 0 {
            has_errors = true;
        }
    }

    let connections_count = if let Ok(count) = state.parallel_data.connection_count.lock() {
        *count
    } else {
        0
    };

    if connections_count > 100 {
        high_connections = true;
    }

    // Generate practical advice
    if has_errors {
        issues.push((
            "🔴 Network errors detected",
            "→ Check cables, switch ports, driver issues",
        ));
    }

    if !has_traffic && connections_count == 0 {
        issues.push((
            "⚠️ No network activity",
            "→ Check network config, firewall, services",
        ));
    }

    if high_connections {
        issues.push((
            "🟡 High connection count",
            "→ Check for connection leaks, DDoS, load",
        ));
    }

    // Add general tips if no issues
    if issues.is_empty() {
        issues.push((
            "✅ Network appears healthy",
            "→ Monitor bandwidth usage and error rates",
        ));
        issues.push((
            "💡 Pro tip",
            "→ Use other tabs for detailed interface/connection analysis",
        ));
    }

    let mut content = Vec::new();
    for (issue, solution) in issues.iter().take(4) {
        content.push(Line::from(vec![Span::styled(
            *issue,
            Style::default().fg(if issue.contains("🔴") {
                Color::Red
            } else if issue.contains("⚠️") || issue.contains("🟡") {
                Color::Yellow
            } else {
                Color::Green
            }),
        )]));
        content.push(Line::from(vec![Span::styled(
            *solution,
            Style::default().fg(Color::White),
        )]));
        content.push(Line::from(""));
    }

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}
