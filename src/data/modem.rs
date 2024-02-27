use serial::{prelude::*, CharSize, FlowControl, StopBits};

#[derive(Clone, Debug, PartialEq)]
pub struct Modem {
    pub device: String,
    pub baud_rate: usize,

    pub char_size: CharSize,
    pub stop_bits: StopBits,
    pub parity: serial::Parity,

    pub flow_control: FlowControl,

    pub init_string: String,
    pub dial_string: String,
}

impl Default for Modem {
    fn default() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            device: "COM1".to_string(),
            #[cfg(not(target_os = "windows"))]
            device: "/dev/ttyS0".to_string(),
            baud_rate: 9600,
            char_size: CharSize::Bits8,
            stop_bits: StopBits::Stop1,
            parity: serial::Parity::ParityNone,
            flow_control: FlowControl::FlowNone,
            init_string: "ATZ".to_string(),
            dial_string: "ATDT".to_string(),
        }
    }
}
