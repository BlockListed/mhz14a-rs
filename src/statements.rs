pub const HELP: &str = r#"mhz14a-rs [--help] [--version] [--license] [--ignore-checksum] [--path <path>] 
    mhz14a-rs - Read data from the mhz14a co2 sensor.
    
    --version - Show version information.
    --license - Show license information.
    --ignore-checksum - Ignore data checksums.
    --path <path> - Specify a custom path for the serial interface."#;

pub const LICENSE: &str = include_str!("../LICENSE");

pub const VERSION: &str = concat!("mhz14a-rs ", env!("CARGO_PKG_VERSION"));
