use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    // Navigation
    NextPanel,  // Tab - Next panel
    PrevPanel,  // Shift+Tab - Previous panel
    NextItem,   // Down arrow, j
    PrevItem,   // Up arrow, k
    NextDevice, // Right arrow, l
    PrevDevice, // Left arrow, h

    // Settings
    ShowOptions,    // F2 - Show options window
    SaveSettings,   // F5 - Save current settings
    ReloadSettings, // F6 - Reload settings from config

    // Control
    Quit,  // 'q' or Ctrl+C
    Reset, // 'r' - Reset statistics
    Pause, // Space - Pause/resume

    // Display modes
    ToggleTrafficUnits, // 'u' - Cycle through traffic unit types (speeds)
    ToggleDataUnits,    // 'U' - Cycle through data unit types (totals)
    ToggleGraphs,       // 'g' - Toggle graph display
    ToggleMultiple,     // Enter - Toggle between single/multiple device view
    ZoomIn,             // '+' - Zoom graph scale
    ZoomOut,            // '-' - Zoom graph scale

    // Config adjustments (for F2 options)
    IncreaseRefresh, // '>' - Increase refresh rate (decrease interval)
    DecreaseRefresh, // '<' - Decrease refresh rate (increase interval)
    IncreaseAverage, // ']' - Increase average window
    DecreaseAverage, // '[' - Decrease average window

    // Unknown/unhandled
    Unknown,
}

impl InputEvent {
    pub fn from_key_event(key_event: KeyEvent) -> Self {
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Tab, KeyModifiers::NONE) => Self::NextPanel,
            (KeyCode::Tab, KeyModifiers::SHIFT) => Self::PrevPanel,
            (KeyCode::BackTab, _) => Self::PrevPanel,

            (KeyCode::Down | KeyCode::Char('j'), _) => Self::NextItem,
            (KeyCode::Up | KeyCode::Char('k'), _) => Self::PrevItem,
            (KeyCode::Right | KeyCode::Char('l'), _) => Self::NextDevice,
            (KeyCode::Left | KeyCode::Char('h'), _) => Self::PrevDevice,

            (KeyCode::Enter, _) => Self::ToggleMultiple,

            (KeyCode::F(2), _) => Self::ShowOptions,
            (KeyCode::F(5), _) => Self::SaveSettings,
            (KeyCode::F(6), _) => Self::ReloadSettings,

            (KeyCode::Char('q'), _) => Self::Quit,
            (KeyCode::Char('r'), _) => Self::Reset,
            (KeyCode::Char(' '), _) => Self::Pause,
            (KeyCode::Char('u'), _) => Self::ToggleTrafficUnits,
            (KeyCode::Char('U'), _) => Self::ToggleDataUnits,
            (KeyCode::Char('g'), _) => Self::ToggleGraphs,
            (KeyCode::Char('+'), _) => Self::ZoomIn,
            (KeyCode::Char('-'), _) => Self::ZoomOut,
            (KeyCode::Char('>'), _) => Self::IncreaseRefresh,
            (KeyCode::Char('<'), _) => Self::DecreaseRefresh,
            (KeyCode::Char(']'), _) => Self::IncreaseAverage,
            (KeyCode::Char('['), _) => Self::DecreaseAverage,

            (KeyCode::Esc, _) => Self::Quit,

            _ => Self::Unknown,
        }
    }
}
