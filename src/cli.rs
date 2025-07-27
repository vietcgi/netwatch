use clap::Parser;

#[derive(Parser, Default)]
#[command(name = "netwatch", about = "A modern network traffic monitor")]
#[command(version, long_about = None)]
pub struct Args {
    /// Network devices to monitor (default: auto-detect all)
    pub devices: Vec<String>,

    /// List available network interfaces and exit
    #[arg(short, long)]
    pub list: bool,

    /// Average window in seconds
    #[arg(short = 'a', long = "average", default_value = "300")]
    pub average_window: u32,

    /// Max incoming bandwidth scaling (kBit/s, 0 = auto)
    #[arg(short = 'i', long = "incoming", default_value = "0")]
    pub max_incoming: u64,

    /// Max outgoing bandwidth scaling (kBit/s, 0 = auto)  
    #[arg(short = 'o', long = "outgoing", default_value = "0")]
    pub max_outgoing: u64,

    /// Refresh interval in milliseconds
    #[arg(short = 't', long = "interval", default_value = "500")]
    pub refresh_interval: u64,

    /// Traffic unit format (h=human-bit, H=human-byte, b=bit, B=byte, k=kbit, K=kbyte, m=mbit, M=mbyte, g=gbit, G=gbyte)
    #[arg(short = 'u', long = "unit", default_value = "k")]
    pub traffic_unit: TrafficUnit,

    /// Data unit format (same as -u but for totals)
    #[arg(short = 'U', long = "data-unit", default_value = "M")]
    pub data_unit: DataUnit,

    /// Show multiple devices without graphs
    #[arg(short = 'm', long = "multiple")]
    pub multiple_devices: bool,

    /// Log traffic data to file
    #[arg(short = 'f', long = "file")]
    pub log_file: Option<String>,

    /// Test mode - print statistics once and exit (bypass TUI)
    #[arg(long)]
    pub test: bool,

    /// Show dashboard data without TUI (debug mode)
    #[arg(long)]
    pub debug_dashboard: bool,

    /// Show before/after comparison of dashboard enhancements
    #[arg(long)]
    pub show_comparison: bool,

    /// Show overview panel data in text mode (no TUI)
    #[arg(long)]
    pub show_overview: bool,

    /// Force terminal mode (bypass TUI for testing)
    #[arg(long)]
    pub force_terminal: bool,

    /// Force SRE forensics terminal mode
    #[arg(long)]
    pub sre_terminal: bool,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq, Default)]
pub enum TrafficUnit {
    #[value(name = "h")]
    #[default]
    HumanBit, // Auto-scale bits
    #[value(name = "H")]
    HumanByte, // Auto-scale bytes
    #[value(name = "b")]
    Bit, // Bit/s
    #[value(name = "B")]
    Byte, // Byte/s
    #[value(name = "k")]
    KiloBit, // kBit/s
    #[value(name = "K")]
    KiloByte, // kByte/s
    #[value(name = "m")]
    MegaBit, // MBit/s
    #[value(name = "M")]
    MegaByte, // MByte/s
    #[value(name = "g")]
    GigaBit, // GBit/s
    #[value(name = "G")]
    GigaByte, // GByte/s
}

pub use TrafficUnit as DataUnit;

impl TrafficUnit {
    #[must_use]
    pub fn next(&self) -> Self {
        match self {
            Self::HumanBit => Self::HumanByte,
            Self::HumanByte => Self::Bit,
            Self::Bit => Self::Byte,
            Self::Byte => Self::KiloBit,
            Self::KiloBit => Self::KiloByte,
            Self::KiloByte => Self::MegaBit,
            Self::MegaBit => Self::MegaByte,
            Self::MegaByte => Self::GigaBit,
            Self::GigaBit => Self::GigaByte,
            Self::GigaByte => Self::HumanBit,
        }
    }

    #[must_use]
    pub fn to_string(&self) -> &'static str {
        match self {
            Self::HumanBit => "h",
            Self::HumanByte => "H",
            Self::Bit => "b",
            Self::Byte => "B",
            Self::KiloBit => "k",
            Self::KiloByte => "K",
            Self::MegaBit => "m",
            Self::MegaByte => "M",
            Self::GigaBit => "g",
            Self::GigaByte => "G",
        }
    }

    #[must_use]
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "h" => Some(Self::HumanBit),
            "H" => Some(Self::HumanByte),
            "b" => Some(Self::Bit),
            "B" => Some(Self::Byte),
            "k" => Some(Self::KiloBit),
            "K" => Some(Self::KiloByte),
            "m" => Some(Self::MegaBit),
            "M" => Some(Self::MegaByte),
            "g" => Some(Self::GigaBit),
            "G" => Some(Self::GigaByte),
            _ => None,
        }
    }
}
