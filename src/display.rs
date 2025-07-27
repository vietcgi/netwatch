use crate::{
    cli::{DataUnit, TrafficUnit},
    config::Config,
    device::{Device, NetworkReader},
    input::InputEvent,
    logger::TrafficLogger,
    stats::StatsCalculator,
};
use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame, Terminal,
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub struct DisplayState {
    pub current_device_index: usize,
    pub devices: Vec<Device>,
    pub show_multiple: bool,
    pub show_graphs: bool,
    pub paused: bool,
    pub traffic_unit: TrafficUnit,
    pub data_unit: DataUnit,
    pub max_incoming: u64, // 0 = auto-scale
    pub max_outgoing: u64, // 0 = auto-scale
    pub zoom_level: f64,   // Graph zoom multiplier
    pub show_options: bool,
    pub settings_message: Option<String>,
}

impl DisplayState {
    pub fn new(devices: Vec<String>, config: &Config) -> Self {
        let devices: Vec<Device> = devices.into_iter().map(Device::new).collect();

        Self {
            current_device_index: 0,
            devices,
            show_multiple: config.multiple_devices,
            show_graphs: true,
            paused: false,
            traffic_unit: config.get_traffic_unit(),
            data_unit: config.get_data_unit(),
            max_incoming: config.max_incoming,
            max_outgoing: config.max_outgoing,
            zoom_level: 1.0,
            show_options: false,
            settings_message: None,
        }
    }
}

pub fn run_ui(
    interfaces: Vec<String>,
    reader: Box<dyn NetworkReader>,
    mut config: Config,
    log_file: Option<String>,
) -> Result<()> {
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut state = DisplayState::new(interfaces, &config);
    let mut stats_calculators: HashMap<String, StatsCalculator> = HashMap::new();
    let mut logger = if log_file.is_some() {
        Some(TrafficLogger::new(log_file)?)
    } else {
        None
    };

    // Initialize stats calculators for each device
    for device in &state.devices {
        stats_calculators.insert(
            device.name.clone(),
            StatsCalculator::new(Duration::from_secs(config.average_window as u64)),
        );
    }

    let refresh_interval = Duration::from_millis(config.refresh_interval);
    let mut last_update = Instant::now();

    loop {
        // Handle input events - scale polling based on refresh rate for performance
        let poll_interval = if config.high_performance {
            (config.refresh_interval / 5).clamp(100, 200)
        } else {
            (config.refresh_interval / 10).clamp(50, 100)
        };
        if event::poll(Duration::from_millis(poll_interval))? {
            if let Event::Key(key_event) = event::read()? {
                let input_event = InputEvent::from_key_event(key_event);

                if handle_input(&mut state, &mut stats_calculators, input_event, &mut config)? {
                    break; // Quit requested
                }
            }
        }

        // Update network statistics
        if !state.paused && last_update.elapsed() >= refresh_interval {
            let mut high_traffic_detected = false;

            for device in &mut state.devices {
                if device.update(reader.as_ref()).is_err() {
                    // Device unavailable, continue with others
                    continue;
                }

                if let Some(calculator) = stats_calculators.get_mut(&device.name) {
                    calculator.add_sample(device.stats.clone());

                    // Check for high traffic conditions (>100MB/s total or >1000 packets/s)
                    let (speed_in, speed_out) = calculator.current_speed();
                    let (packets_in, packets_out) = calculator.total_packets();
                    let total_speed = speed_in + speed_out;
                    if total_speed > 100_000_000 || packets_in + packets_out > 1000 {
                        high_traffic_detected = true;
                    }

                    // Log traffic if logger is enabled
                    if let Some(ref mut logger) = logger {
                        let _ = logger.log_traffic(&device.name, calculator);
                    }
                }
            }

            // Auto-enable high performance security monitoring under heavy load
            if high_traffic_detected && !config.high_performance {
                crate::security::enable_high_performance_security(true);
            }

            last_update = Instant::now();
        }

        // Draw UI
        terminal.draw(|f| {
            draw_ui(f, &state, &stats_calculators, &config);
        })?;
    }

    Ok(())
}

fn handle_input(
    state: &mut DisplayState,
    stats_calculators: &mut HashMap<String, StatsCalculator>,
    event: InputEvent,
    config: &mut Config,
) -> Result<bool> {
    // Handle dashboard-specific events
    match event {
        InputEvent::NextPanel
        | InputEvent::PrevPanel
        | InputEvent::NextItem
        | InputEvent::PrevItem => {
            // These are dashboard-specific events, ignore in legacy mode
            return Ok(false);
        }
        _ => {}
    }

    // If options window is open, handle settings changes
    if state.show_options {
        match event {
            InputEvent::ShowOptions | InputEvent::Quit => {
                state.show_options = false;
                state.settings_message = None; // Clear status message
                return Ok(false);
            }
            // Allow users to change settings while in options window
            InputEvent::ToggleTrafficUnits => {
                state.traffic_unit = state.traffic_unit.next();
                return Ok(false);
            }
            InputEvent::ToggleDataUnits => {
                state.data_unit = state.data_unit.next();
                return Ok(false);
            }
            // These settings only change display mode, which would be confusing
            // in options window - disable them for better UX
            InputEvent::ToggleGraphs | InputEvent::ToggleMultiple => {
                // Ignore view-changing commands while in options
                return Ok(false);
            }
            InputEvent::Pause => {
                state.paused = !state.paused;
                return Ok(false);
            }
            InputEvent::ZoomIn => {
                state.zoom_level = (state.zoom_level * 1.5).min(10.0);
                return Ok(false);
            }
            InputEvent::ZoomOut => {
                state.zoom_level = (state.zoom_level / 1.5).max(0.1);
                return Ok(false);
            }
            InputEvent::SaveSettings => {
                // Update config with current state values
                config.traffic_format = state.traffic_unit.to_string().to_string();
                config.data_format = state.data_unit.to_string().to_string();
                config.multiple_devices = state.show_multiple;
                config.max_incoming = state.max_incoming;
                config.max_outgoing = state.max_outgoing;

                // Save to file
                match config.save() {
                    Ok(_) => {
                        state.settings_message =
                            Some("✅ Settings saved to ~/.netwatch".to_string())
                    }
                    Err(e) => state.settings_message = Some(format!("❌ Save failed: {e}")),
                }
                return Ok(false);
            }
            InputEvent::IncreaseRefresh => {
                config.refresh_interval = (config.refresh_interval.saturating_sub(50)).max(50); // Min 50ms
                return Ok(false);
            }
            InputEvent::DecreaseRefresh => {
                config.refresh_interval = (config.refresh_interval + 50).min(2000); // Max 2000ms
                return Ok(false);
            }
            InputEvent::IncreaseAverage => {
                config.average_window = (config.average_window + 30).min(1800); // Max 30 minutes
                return Ok(false);
            }
            InputEvent::DecreaseAverage => {
                config.average_window = (config.average_window.saturating_sub(30)).max(30); // Min 30 seconds
                return Ok(false);
            }
            InputEvent::ReloadSettings => {
                // Reload settings from config file
                match Config::load() {
                    Ok(new_config) => {
                        *config = new_config;
                        // Update state with reloaded config
                        state.traffic_unit = config.get_traffic_unit();
                        state.data_unit = config.get_data_unit();
                        state.show_multiple = config.multiple_devices;
                        state.max_incoming = config.max_incoming;
                        state.max_outgoing = config.max_outgoing;
                        state.settings_message =
                            Some("✅ Settings reloaded from ~/.netwatch".to_string());
                    }
                    Err(e) => {
                        state.settings_message = Some(format!("❌ Reload failed: {e}"));
                    }
                }
                return Ok(false);
            }
            _ => {
                // Ignore navigation and other non-settings commands
                return Ok(false);
            }
        }
    }

    match event {
        InputEvent::Quit => return Ok(true),

        InputEvent::NextDevice => {
            if !state.devices.is_empty() {
                state.current_device_index = (state.current_device_index + 1) % state.devices.len();
            }
        }

        InputEvent::PrevDevice => {
            if !state.devices.is_empty() {
                state.current_device_index = if state.current_device_index == 0 {
                    state.devices.len() - 1
                } else {
                    state.current_device_index - 1
                };
            }
        }

        InputEvent::Reset => {
            // Reset statistics for current device
            if let Some(device) = state.devices.get(state.current_device_index) {
                if let Some(calculator) = stats_calculators.get_mut(&device.name) {
                    calculator.reset();
                }
            }
        }

        InputEvent::Pause => {
            state.paused = !state.paused;
        }

        InputEvent::ToggleTrafficUnits => {
            state.traffic_unit = state.traffic_unit.next();
        }

        InputEvent::ToggleDataUnits => {
            state.data_unit = state.data_unit.next();
        }

        InputEvent::ToggleGraphs => {
            state.show_graphs = !state.show_graphs;
        }

        InputEvent::ToggleMultiple => {
            state.show_multiple = !state.show_multiple;
        }

        InputEvent::ZoomIn => {
            state.zoom_level = (state.zoom_level * 1.5).min(10.0);
        }

        InputEvent::ZoomOut => {
            state.zoom_level = (state.zoom_level / 1.5).max(0.1);
        }

        InputEvent::ShowOptions => {
            state.show_options = !state.show_options;
        }

        InputEvent::SaveSettings => {
            // Update config with current state values
            config.traffic_format = state.traffic_unit.to_string().to_string();
            config.data_format = state.data_unit.to_string().to_string();
            config.multiple_devices = state.show_multiple;
            config.max_incoming = state.max_incoming;
            config.max_outgoing = state.max_outgoing;

            // Save to file
            if let Err(e) = config.save() {
                eprintln!("Failed to save settings: {e}");
            }
        }

        InputEvent::ReloadSettings => {
            // Reload settings from config file
            if let Ok(new_config) = Config::load() {
                *config = new_config;
                // Update state with reloaded config
                state.traffic_unit = config.get_traffic_unit();
                state.data_unit = config.get_data_unit();
                state.show_multiple = config.multiple_devices;
                state.max_incoming = config.max_incoming;
                state.max_outgoing = config.max_outgoing;
            }
        }

        InputEvent::IncreaseRefresh
        | InputEvent::DecreaseRefresh
        | InputEvent::IncreaseAverage
        | InputEvent::DecreaseAverage => {
            // These are only handled in options window
        }

        InputEvent::NextPanel
        | InputEvent::PrevPanel
        | InputEvent::NextItem
        | InputEvent::PrevItem => {
            // These are dashboard-specific, already handled above
        }

        InputEvent::Unknown => {
            // Ignore unknown input
        }
    }

    Ok(false)
}

fn draw_ui(
    f: &mut Frame,
    state: &DisplayState,
    stats_calculators: &HashMap<String, StatsCalculator>,
    config: &Config,
) {
    if state.show_multiple {
        draw_multiple_devices_view(f, state, stats_calculators);
    } else {
        draw_single_device_view(f, state, stats_calculators, config);
    }
}

fn draw_single_device_view(
    f: &mut Frame,
    state: &DisplayState,
    stats_calculators: &HashMap<String, StatsCalculator>,
    config: &Config,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header with device name
            Constraint::Min(10),   // Main graphs/stats area
            Constraint::Length(3), // Status/help line
        ])
        .split(f.area());

    // Get current device and its stats
    if let Some(device) = state.devices.get(state.current_device_index) {
        // Header
        draw_header(f, chunks[0], &device.name, state.paused);

        // Main content area
        if state.show_graphs {
            // TODO: Draw traffic graphs
            draw_placeholder_graphs(f, chunks[1], device, stats_calculators, state);
        } else {
            // TODO: Draw statistics table
            draw_placeholder_stats(f, chunks[1], device, stats_calculators, state);
        }

        // Status line
        draw_status_line(f, chunks[2], state);

        // Options overlay (if shown)
        if state.show_options {
            draw_options_overlay(f, f.area(), state, config);
        }
    }
}

fn draw_multiple_devices_view(
    f: &mut Frame,
    state: &DisplayState,
    stats_calculators: &HashMap<String, StatsCalculator>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Device list
            Constraint::Length(3), // Status/help line
        ])
        .split(f.area());

    // Header
    let header_text = if state.paused {
        "netwatch - Multiple Devices View [PAUSED]"
    } else {
        "netwatch - Multiple Devices View"
    };

    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Device list area
    if state.devices.is_empty() {
        let no_devices = Paragraph::new("No network devices found")
            .block(Block::default().borders(Borders::ALL).title("Devices"))
            .style(Style::default().fg(Color::Red));
        f.render_widget(no_devices, chunks[1]);
    } else {
        draw_devices_table(f, chunks[1], state, stats_calculators);
    }

    // Status line
    draw_multiple_devices_status_line(f, chunks[2], state);
}

fn draw_devices_table(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    state: &DisplayState,
    stats_calculators: &HashMap<String, StatsCalculator>,
) {
    // Create table header
    let mut table_content = String::new();
    table_content.push_str("┌─────────────────┬──────────────┬──────────────┬──────────────┬──────────────┬─────────────────┐\n");
    table_content.push_str("│     Device      │   Current    │   Current    │   Average    │   Average    │      Total      │\n");
    table_content.push_str("│                 │   In (↓)     │   Out (↑)    │   In (↓)     │   Out (↑)    │   In/Out        │\n");
    table_content.push_str("├─────────────────┼──────────────┼──────────────┼──────────────┼──────────────┼─────────────────┤\n");

    // Add device rows
    for (i, device) in state.devices.iter().enumerate() {
        let is_selected = i == state.current_device_index;
        let prefix = if is_selected { "►" } else { " " };

        if let Some(calculator) = stats_calculators.get(&device.name) {
            let (current_in, current_out) = calculator.current_speed();
            let (avg_in, avg_out) = calculator.average_speed();
            let (total_in, total_out) = calculator.total_bytes();

            table_content.push_str(&format!(
                "│{} {:13} │ {:>11}/s │ {:>11}/s │ {:>11}/s │ {:>11}/s │ {:>7}/{:<7} │\n",
                prefix,
                truncate_device_name(&device.name, 13),
                format_bytes_short(current_in),
                format_bytes_short(current_out),
                format_bytes_short(avg_in),
                format_bytes_short(avg_out),
                format_bytes_short(total_in),
                format_bytes_short(total_out)
            ));
        } else {
            table_content.push_str(&format!(
                "│{} {:13} │ {:>12} │ {:>12} │ {:>12} │ {:>12} │ {:>15} │\n",
                prefix,
                truncate_device_name(&device.name, 13),
                "No data",
                "No data",
                "No data",
                "No data",
                "No data"
            ));
        }
    }

    table_content.push_str("└─────────────────┴──────────────┴──────────────┴──────────────┴──────────────┴─────────────────┘\n");
    table_content.push_str(
        "\nUse arrow keys to select device, Enter to view details, 'r' to reset selected device",
    );

    let devices_table = Paragraph::new(table_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Network Devices"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(devices_table, area);
}

fn draw_multiple_devices_status_line(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    _state: &DisplayState,
) {
    let help_text = vec![Line::from(vec![
        Span::styled("Press ", Style::default().fg(Color::Gray)),
        Span::styled(
            "'q'",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to quit, ", Style::default().fg(Color::Gray)),
        Span::styled(
            "arrows",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to select device, ", Style::default().fg(Color::Gray)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" for details, ", Style::default().fg(Color::Gray)),
        Span::styled(
            "'r'",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to reset", Style::default().fg(Color::Gray)),
    ])];

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(help, area);
}

fn draw_header(f: &mut Frame, area: ratatui::layout::Rect, device_name: &str, paused: bool) {
    let status = if paused { " [PAUSED]" } else { "" };
    let title = format!("netwatch - Network Traffic Monitor [{device_name}]{status}");

    let header = Paragraph::new(title)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(header, area);
}

fn draw_placeholder_graphs(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    device: &Device,
    stats_calculators: &HashMap<String, StatsCalculator>,
    state: &DisplayState,
) {
    if let Some(calculator) = stats_calculators.get(&device.name) {
        // Split area into stats section and graph section
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Stats display area
                Constraint::Min(10),   // Graph area
            ])
            .split(area);

        // Draw statistics summary
        draw_stats_summary(f, chunks[0], device, calculator);

        // Draw the actual graphs
        draw_traffic_graphs_internal(f, chunks[1], calculator, state);
    } else {
        let no_data = Paragraph::new("No statistics available for this device")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Traffic Monitor"),
            )
            .style(Style::default().fg(Color::Red));
        f.render_widget(no_data, area);
    }
}

fn draw_stats_summary(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    device: &Device,
    calculator: &StatsCalculator,
) {
    let (current_in, current_out) = calculator.current_speed();
    let (avg_in, avg_out) = calculator.average_speed();
    let (_min_in, _min_out) = calculator.min_speed();
    let (max_in, max_out) = calculator.max_speed();

    let stats_text = format!(
        "📶 Device: {}     Current Traffic: 📥 {}/s down  📤 {}/s up\nAverages: 📊 {}/s down  📊 {}/s up     Peak: 📈 {}/s down  📈 {}/s up",
        device.name,
        format_bytes(current_in),
        format_bytes(current_out),
        format_bytes(avg_in),
        format_bytes(avg_out),
        format_bytes(max_in),
        format_bytes(max_out)
    );

    let stats_widget = Paragraph::new(stats_text)
        .block(Block::default().borders(Borders::ALL).title("Statistics"))
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(stats_widget, area);
}

pub fn draw_traffic_graphs(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    device_name: &str,
    calculator: &StatsCalculator,
    dashboard_state: &crate::dashboard::DashboardState,
) {
    // Create a compatibility DisplayState for the existing function
    let state = DisplayState {
        current_device_index: dashboard_state.current_device_index,
        devices: dashboard_state.devices.clone(),
        show_multiple: false,
        show_graphs: true,
        paused: dashboard_state.paused,
        traffic_unit: dashboard_state.traffic_unit.clone(),
        data_unit: dashboard_state.data_unit.clone(),
        max_incoming: dashboard_state.max_incoming,
        max_outgoing: dashboard_state.max_outgoing,
        zoom_level: dashboard_state.zoom_level,
        show_options: false,
        settings_message: None,
    };

    draw_traffic_graphs_with_device_name(f, area, device_name, calculator, &state);
}

fn draw_traffic_graphs_with_device_name(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    device_name: &str,
    calculator: &StatsCalculator,
    state: &DisplayState,
) {
    // Split into incoming and outgoing graph areas
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Get graph data
    let graph_data_in = calculator.graph_data_in();
    let graph_data_out = calculator.graph_data_out();

    // Draw incoming traffic graph with device name
    draw_single_graph_with_device(
        f,
        chunks[0],
        &format!("{device_name} - Incoming"),
        graph_data_in,
        Color::Green,
        calculator.max_speed().0, // max incoming
        state,
    );

    // Draw outgoing traffic graph with device name
    draw_single_graph_with_device(
        f,
        chunks[1],
        &format!("{device_name} - Outgoing"),
        graph_data_out,
        Color::Red,
        calculator.max_speed().1, // max outgoing
        state,
    );
}

fn draw_traffic_graphs_internal(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    calculator: &StatsCalculator,
    state: &DisplayState,
) {
    // Split into incoming and outgoing graph areas
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Get graph data
    let graph_data_in = calculator.graph_data_in();
    let graph_data_out = calculator.graph_data_out();

    // Draw incoming traffic graph
    draw_single_graph(
        f,
        chunks[0],
        "Incoming Traffic",
        graph_data_in,
        Color::Green,
        calculator.max_speed().0, // max incoming
        state,
    );

    // Draw outgoing traffic graph
    draw_single_graph(
        f,
        chunks[1],
        "Outgoing Traffic",
        graph_data_out,
        Color::Red,
        calculator.max_speed().1, // max outgoing
        state,
    );
}

fn draw_single_graph_with_device(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    title: &str,
    data: &std::collections::VecDeque<(f64, f64)>,
    color: Color,
    max_value: u64,
    state: &DisplayState,
) {
    if data.is_empty() {
        let no_data = Paragraph::new("Collecting data...")
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(no_data, area);
        return;
    }

    if data.len() < 2 {
        let waiting = Paragraph::new(format!("Waiting for more data... ({})", data.len()))
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(waiting, area);
        return;
    }

    // Calculate bounds with smart scaling first
    let min_x = 0.0; // Left side starts at "now" (time 0)
    let max_x = 60.0; // Right side goes to "60 seconds ago"

    // Calculate Y-axis bounds based on network capacity tiers
    let data_max = data
        .iter()
        .map(|(_, y)| *y)
        .filter(|y| y.is_finite() && *y >= 0.0)
        .fold(0.0, f64::max);
    let actual_max = if data_max > 0.0 {
        data_max as u64
    } else if max_value > 0 {
        max_value
    } else {
        1024 // 1KB minimum
    };

    // Use network capacity scale for graph bounds, adjusted by zoom level
    let base_max_y = get_network_capacity_scale(actual_max) as f64;
    let max_y = if state.zoom_level > 0.0 && state.zoom_level.is_finite() {
        base_max_y / state.zoom_level // Higher zoom = smaller Y range = "zoomed in"
    } else {
        base_max_y // Fallback if zoom_level is invalid
    };

    // Convert data to chart format and sort by time (newest to oldest for proper line drawing)
    let chart_data: Vec<(f64, f64)> = data
        .iter()
        .cloned()
        .filter(|(x, y)| x.is_finite() && y.is_finite() && *x >= 0.0 && *y >= 0.0)
        .collect();
    let mut chart_data = chart_data;

    // If no valid data after filtering, show waiting message
    if chart_data.is_empty() {
        let waiting = Paragraph::new("Waiting for valid data...")
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(waiting, area);
        return;
    }

    chart_data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)); // Sort by time, safe fallback

    // Create dataset
    let dataset = Dataset::default()
        .name(title)
        .marker(ratatui::symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color))
        .data(&chart_data);

    // Try to create chart, fallback to ASCII if it fails
    let chart = Chart::new(vec![dataset])
        .block(Block::default().borders(Borders::ALL).title(format!(
            "{} (Max: {}) - Use ↑/↓ to switch devices",
            title,
            format_bytes(max_value)
        )))
        .x_axis(
            Axis::default()
                .title("Time")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_x, max_x])
                .labels(vec!["Now", "30s ago", "1 min ago"]),
        )
        .y_axis(
            Axis::default()
                .title("Speed")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, max_y])
                .labels(create_smart_y_labels(max_y)),
        );

    // If chart rendering fails, use ASCII fallback
    if area.width < 20 || area.height < 8 {
        draw_ascii_graph_with_device(f, area, title, data, color, max_value);
    } else {
        f.render_widget(chart, area);
    }
}

fn draw_single_graph(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    title: &str,
    data: &std::collections::VecDeque<(f64, f64)>,
    color: Color,
    max_value: u64,
    state: &DisplayState,
) {
    if data.is_empty() {
        let no_data = Paragraph::new("Collecting data...")
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(no_data, area);
        return;
    }

    if data.len() < 2 {
        let waiting = Paragraph::new(format!("Waiting for more data... ({})", data.len()))
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(waiting, area);
        return;
    }

    // Calculate bounds with smart scaling first
    let min_x = 0.0; // Left side starts at "now" (time 0)
    let max_x = 60.0; // Right side goes to "60 seconds ago"

    // Calculate Y-axis bounds based on network capacity tiers
    let data_max = data
        .iter()
        .map(|(_, y)| *y)
        .filter(|y| y.is_finite() && *y >= 0.0)
        .fold(0.0, f64::max);
    let actual_max = if data_max > 0.0 {
        data_max as u64
    } else if max_value > 0 {
        max_value
    } else {
        1024 // 1KB minimum
    };

    // Use network capacity scale for graph bounds, adjusted by zoom level
    let base_max_y = get_network_capacity_scale(actual_max) as f64;
    let max_y = if state.zoom_level > 0.0 && state.zoom_level.is_finite() {
        base_max_y / state.zoom_level // Higher zoom = smaller Y range = "zoomed in"
    } else {
        base_max_y // Fallback if zoom_level is invalid
    };

    // Convert data to chart format and sort by time (newest to oldest for proper line drawing)
    let chart_data: Vec<(f64, f64)> = data
        .iter()
        .cloned()
        .filter(|(x, y)| x.is_finite() && y.is_finite() && *x >= 0.0 && *y >= 0.0)
        .collect();
    let mut chart_data = chart_data;

    // If no valid data after filtering, show waiting message
    if chart_data.is_empty() {
        let waiting = Paragraph::new("Waiting for valid data...")
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(waiting, area);
        return;
    }

    chart_data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)); // Sort by time, safe fallback

    // Create dataset
    let dataset = Dataset::default()
        .name(title)
        .marker(ratatui::symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color))
        .data(&chart_data);

    // Try to create chart, fallback to ASCII if it fails
    let chart = Chart::new(vec![dataset])
        .block(Block::default().borders(Borders::ALL).title(format!(
            "{} (Max: {}) - Use ↑/↓ to switch devices",
            title,
            format_bytes(max_value)
        )))
        .x_axis(
            Axis::default()
                .title("Time")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_x, max_x])
                .labels(vec!["Now", "30s ago", "1 min ago"]),
        )
        .y_axis(
            Axis::default()
                .title("Speed")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, max_y])
                .labels(create_smart_y_labels(max_y)),
        );

    // If chart rendering fails, use ASCII fallback
    if area.width < 20 || area.height < 8 {
        draw_ascii_graph(f, area, title, data, color, max_value);
    } else {
        f.render_widget(chart, area);
    }
}

fn draw_ascii_graph_with_device(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    title: &str,
    data: &std::collections::VecDeque<(f64, f64)>,
    color: Color,
    max_value: u64,
) {
    if data.is_empty() {
        let no_data = Paragraph::new("No data available")
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(no_data, area);
        return;
    }

    // Create ASCII bar graph
    let graph_height = (area.height.saturating_sub(3)) as usize; // Account for borders and title
    let graph_width = (area.width.saturating_sub(2)) as usize; // Account for borders

    if graph_height == 0 || graph_width == 0 {
        let too_small = Paragraph::new("Area too small for graph")
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(too_small, area);
        return;
    }

    // Get max value for scaling
    let data_max = data
        .iter()
        .map(|(_, y)| *y)
        .filter(|y| y.is_finite() && *y >= 0.0)
        .fold(0.0, f64::max);
    let scale_max = if data_max > 0.0 {
        data_max
    } else {
        max_value as f64
    };

    // Sample data points across the width
    let mut graph_lines = vec![String::new(); graph_height];
    let step = if data.len() > graph_width {
        data.len() / graph_width
    } else {
        1
    };

    for (i, chunk) in data.iter().step_by(step).take(graph_width).enumerate() {
        let (_, value) = chunk;
        let normalized = if scale_max > 0.0 {
            (value / scale_max * graph_height as f64) as usize
        } else {
            0
        };
        let bar_height = normalized.min(graph_height);

        // Draw vertical bar using Unicode block characters
        for (row, line) in graph_lines.iter_mut().enumerate().take(graph_height) {
            let char_to_use = if (graph_height - row - 1) < bar_height {
                match ((graph_height - row - 1) * 8) % 8 {
                    0..=1 => "█", // Full block
                    2..=3 => "▇", // 7/8 block
                    4..=5 => "▆", // 3/4 block
                    6..=7 => "▅", // 5/8 block
                    _ => "█",
                }
            } else {
                " "
            };

            if i < line.len() {
                line.replace_range(i..=i, char_to_use);
            } else {
                while line.len() < i {
                    line.push(' ');
                }
                line.push_str(char_to_use);
            }
        }
    }

    // Add current value and max info
    let current_val = data.back().map(|(_, v)| *v).unwrap_or(0.0);
    let info_line = format!(
        "Current: {}/s | Max: {}/s",
        format_bytes(current_val as u64),
        format_bytes(scale_max as u64)
    );

    // Combine all lines
    let mut all_lines = graph_lines;
    all_lines.push(String::new()); // Empty line
    all_lines.push(info_line);

    let graph_text: Vec<ratatui::text::Line> = all_lines
        .into_iter()
        .map(|line| {
            ratatui::text::Line::from(ratatui::text::Span::styled(
                line,
                Style::default().fg(color),
            ))
        })
        .collect();

    let ascii_graph = Paragraph::new(graph_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("📊 {title} (ASCII) - Use ↑/↓ to switch devices")),
        )
        .style(Style::default().fg(color));

    f.render_widget(ascii_graph, area);
}

fn draw_ascii_graph(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    title: &str,
    data: &std::collections::VecDeque<(f64, f64)>,
    color: Color,
    max_value: u64,
) {
    if data.is_empty() {
        let no_data = Paragraph::new("No data available")
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(no_data, area);
        return;
    }

    // Create ASCII bar graph
    let graph_height = (area.height.saturating_sub(3)) as usize; // Account for borders and title
    let graph_width = (area.width.saturating_sub(2)) as usize; // Account for borders

    if graph_height == 0 || graph_width == 0 {
        let too_small = Paragraph::new("Area too small for graph")
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(too_small, area);
        return;
    }

    // Get max value for scaling
    let data_max = data
        .iter()
        .map(|(_, y)| *y)
        .filter(|y| y.is_finite() && *y >= 0.0)
        .fold(0.0, f64::max);
    let scale_max = if data_max > 0.0 {
        data_max
    } else {
        max_value as f64
    };

    // Sample data points across the width
    let mut graph_lines = vec![String::new(); graph_height];
    let step = if data.len() > graph_width {
        data.len() / graph_width
    } else {
        1
    };

    for (i, chunk) in data.iter().step_by(step).take(graph_width).enumerate() {
        let (_, value) = chunk;
        let normalized = if scale_max > 0.0 {
            (value / scale_max * graph_height as f64) as usize
        } else {
            0
        };
        let bar_height = normalized.min(graph_height);

        // Draw vertical bar using Unicode block characters
        for (row, line) in graph_lines.iter_mut().enumerate().take(graph_height) {
            let char_to_use = if (graph_height - row - 1) < bar_height {
                match ((graph_height - row - 1) * 8) % 8 {
                    0..=1 => "█", // Full block
                    2..=3 => "▇", // 7/8 block
                    4..=5 => "▆", // 3/4 block
                    6..=7 => "▅", // 5/8 block
                    _ => "█",
                }
            } else {
                " "
            };

            if i < line.len() {
                line.replace_range(i..=i, char_to_use);
            } else {
                while line.len() < i {
                    line.push(' ');
                }
                line.push_str(char_to_use);
            }
        }
    }

    // Add current value and max info
    let current_val = data.back().map(|(_, v)| *v).unwrap_or(0.0);
    let info_line = format!(
        "Current: {}/s | Max: {}/s",
        format_bytes(current_val as u64),
        format_bytes(scale_max as u64)
    );

    // Combine all lines
    let mut all_lines = graph_lines;
    all_lines.push(String::new()); // Empty line
    all_lines.push(info_line);

    let graph_text: Vec<ratatui::text::Line> = all_lines
        .into_iter()
        .map(|line| {
            ratatui::text::Line::from(ratatui::text::Span::styled(
                line,
                Style::default().fg(color),
            ))
        })
        .collect();

    let ascii_graph = Paragraph::new(graph_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("📊 {title} (ASCII) - Use ↑/↓ to switch devices")),
        )
        .style(Style::default().fg(color));

    f.render_widget(ascii_graph, area);
}

fn draw_placeholder_stats(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    device: &Device,
    stats_calculators: &HashMap<String, StatsCalculator>,
    state: &DisplayState,
) {
    if let Some(calculator) = stats_calculators.get(&device.name) {
        draw_detailed_stats_table(
            f,
            area,
            device,
            calculator,
            &state.traffic_unit,
            &state.data_unit,
        );
    } else {
        let no_data = Paragraph::new("No statistics available for this device")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Statistics Table"),
            )
            .style(Style::default().fg(Color::Red));
        f.render_widget(no_data, area);
    }
}

fn draw_detailed_stats_table(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    device: &Device,
    calculator: &StatsCalculator,
    traffic_unit: &TrafficUnit,
    data_unit: &DataUnit,
) {
    // Get statistics
    let (current_in, current_out) = calculator.current_speed();
    let (avg_in, avg_out) = calculator.average_speed();
    let (min_in, min_out) = calculator.min_speed();
    let (max_in, max_out) = calculator.max_speed();
    let (total_bytes_in, total_bytes_out) = calculator.total_bytes();
    let (total_packets_in, total_packets_out) = calculator.total_packets();

    // Create table content
    let table_content = format!(
        "Device: {}\n\
        \n\
        ┌─────────────────────────────┬──────────────────┬──────────────────┐\n\
        │         Statistic           │    Incoming      │    Outgoing      │\n\
        ├─────────────────────────────┼──────────────────┼──────────────────┤\n\
        │ Current Speed               │ {:>15}/s │ {:>15}/s │\n\
        │ Average Speed               │ {:>15}/s │ {:>15}/s │\n\
        │ Minimum Speed               │ {:>15}/s │ {:>15}/s │\n\
        │ Maximum Speed               │ {:>15}/s │ {:>15}/s │\n\
        ├─────────────────────────────┼──────────────────┼──────────────────┤\n\
        │ Total Bytes                 │ {:>16} │ {:>16} │\n\
        │ Total Packets               │ {:>16} │ {:>16} │\n\
        └─────────────────────────────┴──────────────────┴──────────────────┘\n\
        \n\
        Network Interface Statistics - Press 'g' to toggle back to graphs",
        device.name,
        format_bytes_with_unit(current_in, traffic_unit),
        format_bytes_with_unit(current_out, traffic_unit),
        format_bytes_with_unit(avg_in, traffic_unit),
        format_bytes_with_unit(avg_out, traffic_unit),
        format_bytes_with_unit(min_in, traffic_unit),
        format_bytes_with_unit(min_out, traffic_unit),
        format_bytes_with_unit(max_in, traffic_unit),
        format_bytes_with_unit(max_out, traffic_unit),
        format_bytes_with_unit(total_bytes_in, data_unit),
        format_bytes_with_unit(total_bytes_out, data_unit),
        format_number(total_packets_in),
        format_number(total_packets_out),
    );

    let stats_table = Paragraph::new(table_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Detailed Network Statistics"),
        )
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(stats_table, area);
}

fn draw_status_line(f: &mut Frame, area: ratatui::layout::Rect, _state: &DisplayState) {
    let help_text = vec![Line::from(vec![
        Span::styled("Press ", Style::default().fg(Color::Gray)),
        Span::styled(
            "'q'",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to quit, ", Style::default().fg(Color::Gray)),
        Span::styled(
            "arrows",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to switch devices, ", Style::default().fg(Color::Gray)),
        Span::styled(
            "'r'",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to reset, ", Style::default().fg(Color::Gray)),
        Span::styled(
            "space",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to pause", Style::default().fg(Color::Gray)),
    ])];

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(help, area);
}

// Helper function for formatting bytes
fn format_bytes(bytes: u64) -> String {
    format_bytes_with_unit(bytes, &TrafficUnit::HumanByte)
}

// Helper function for formatting bytes with specific unit
fn format_bytes_with_unit(bytes: u64, unit: &TrafficUnit) -> String {
    match unit {
        TrafficUnit::HumanBit => {
            let bits = bytes * 8;
            format_human_readable(bits, &["bit", "Kbit", "Mbit", "Gbit", "Tbit"], 1000.0)
        }
        TrafficUnit::HumanByte => {
            format_human_readable(bytes, &["B", "KB", "MB", "GB", "TB"], 1024.0)
        }
        TrafficUnit::Bit => format!("{} bit", bytes * 8),
        TrafficUnit::Byte => format!("{bytes} B"),
        TrafficUnit::KiloBit => format!("{:.2} kbit", (bytes * 8) as f64 / 1000.0),
        TrafficUnit::KiloByte => format!("{:.2} KB", bytes as f64 / 1024.0),
        TrafficUnit::MegaBit => format!("{:.2} Mbit", (bytes * 8) as f64 / 1_000_000.0),
        TrafficUnit::MegaByte => format!("{:.2} MB", bytes as f64 / 1_048_576.0),
        TrafficUnit::GigaBit => format!("{:.2} Gbit", (bytes * 8) as f64 / 1_000_000_000.0),
        TrafficUnit::GigaByte => format!("{:.2} GB", bytes as f64 / 1_073_741_824.0),
    }
}

fn format_human_readable(value: u64, units: &[&str], divisor: f64) -> String {
    let mut size = value as f64;
    let mut unit_index = 0;

    while size >= divisor && unit_index < units.len() - 1 {
        size /= divisor;
        unit_index += 1;
    }

    if size >= 100.0 {
        format!("{:.0} {}", size, units[unit_index])
    } else if size >= 10.0 {
        format!("{:.1} {}", size, units[unit_index])
    } else {
        format!("{:.2} {}", size, units[unit_index])
    }
}

// Helper function for formatting large numbers with commas
fn format_number(num: u64) -> String {
    let num_str = num.to_string();
    let mut result = String::new();
    let chars: Vec<char> = num_str.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }

    result
}

// Helper function for formatting bytes in a shorter format (for tables)
fn format_bytes_short(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if size >= 100.0 {
        format!("{:.0}{}", size, UNITS[unit_index])
    } else if size >= 10.0 {
        format!("{:.1}{}", size, UNITS[unit_index])
    } else {
        format!("{:.2}{}", size, UNITS[unit_index])
    }
}

// Helper function to truncate device names for table display
fn truncate_device_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}...", &name[..max_len.saturating_sub(3)])
    }
}

// Determine appropriate network capacity scale based on actual traffic
fn get_network_capacity_scale(actual_max: u64) -> u64 {
    // Convert to bits per second for network capacity comparison
    let actual_bits = actual_max * 8;

    // Network capacity tiers (in bits per second)
    let tiers = vec![
        1_000_000,       // 1 Mbps
        10_000_000,      // 10 Mbps
        100_000_000,     // 100 Mbps
        1_000_000_000,   // 1 Gbps
        10_000_000_000,  // 10 Gbps
        40_000_000_000,  // 40 Gbps
        100_000_000_000, // 100 Gbps
    ];

    // Find the next tier above actual usage
    for &tier in &tiers {
        if actual_bits <= tier {
            return tier / 8; // Convert back to bytes per second
        }
    }

    // If higher than all tiers, use 100 Gbps
    100_000_000_000 / 8
}

// Create network-capacity-aware Y-axis labels for bounds [0.0, max_y]
fn create_smart_y_labels(max_y: f64) -> Vec<ratatui::text::Span<'static>> {
    let capacity_scale = max_y as u64; // max_y is already the capacity scale

    // Labels for Y-axis bounds [0.0, max_y]
    // First label = 0.0 (bottom), Last label = max_y (top)
    let labels = vec![
        "0 B/s".into(),                                               // 0.0 (bottom)
        format!("{}/s", format_bytes(capacity_scale / 4)).into(),     // 25% (lower)
        format!("{}/s", format_bytes(capacity_scale / 2)).into(),     // 50% (middle)
        format!("{}/s", format_bytes(capacity_scale * 3 / 4)).into(), // 75% (upper)
        format!("{}/s", format_bytes(capacity_scale)).into(),         // max_y (top)
    ];

    labels
}

fn draw_options_overlay(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    state: &DisplayState,
    config: &Config,
) {
    // Create a centered popup area
    let popup_area = centered_rect(60, 70, area);

    // Clear the area
    let clear = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(clear, popup_area);

    // Create options content
    let options_text = format!(
        "═══════════════════ OPTIONS ═══════════════════\n\
        \n\
        Current Settings:\n\
        \n\
        • Traffic Unit:     {:?}\n\
        • Data Unit:        {:?}\n\
        • Show Graphs:      {}\n\
        • Multiple View:    {}\n\
        • Paused:           {}\n\
        • Zoom Level:       {:.1}x\n\
        • Max Incoming:     {} (0 = auto)\n\
        • Max Outgoing:     {} (0 = auto)\n\
        • Average Window:   {}s\n\
        • Refresh Rate:     {}ms\n\
        • Devices:          {}\n\
        \n\
        ──────────── Interactive Controls ───────────────\n\
        \n\
        You can change settings while this window is open:\n\
        \n\
        • 'u' - Cycle traffic units (speeds - changes above ↑)\n\
        • 'U' - Cycle data units (totals - changes above ↑)\n\
        • Space - Pause/resume monitoring\n\
        • '+/-' - Zoom graph scale\n\
        • '</>' - Slower/Faster refresh rate\n\
        • '[/]' - Shorter/Longer average window\n\
        • F5 - Save current settings to file\n\
        • F6 - Reload settings from file\n\
        \n\
        ────────── View Controls (when closed) ──────────\n\
        \n\
        • 'g' - Toggle graphs/stats view\n\
        • Enter - Toggle single/multiple view\n\
        • Arrow keys - Navigate devices\n\
        • 'r' - Reset statistics\n\
        \n\
        Press F2 or ESC to close this options window\n\
        \n\
        {}",
        state.traffic_unit,
        state.data_unit,
        if state.show_graphs { "Yes" } else { "No" },
        if state.show_multiple { "Yes" } else { "No" },
        if state.paused { "Yes" } else { "No" },
        state.zoom_level,
        state.max_incoming,
        state.max_outgoing,
        config.average_window,
        config.refresh_interval,
        config.devices,
        state.settings_message.as_deref().unwrap_or("")
    );

    let options_popup = Paragraph::new(options_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Options & Settings")
                .style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black));

    f.render_widget(options_popup, popup_area);
}

// Helper function to create a centered rectangle
fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
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
