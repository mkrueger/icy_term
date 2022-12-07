use super::FrameType;
use std::error::Error;

#[derive(Debug, Clone, Copy)]
pub enum TransmissionError {
    // Cancel,
    //InvalidMode(u8),
    InvalidSubpacket(u8),
    InvalidFrameType(u8),
    ZPADExected(u8),
    ZLDEExected(u8),
    UnknownHeaderType(u8),
    CRC16Mismatch(u16, u16),
    CRC32Mismatch(u32, u32),
    ZDataBeforeZFILE,
    UnsupportedFrame(FrameType),
    HexNumberExpected,
}

impl std::fmt::Display for TransmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // TransmissionError::Cancel => write!(f, "transmission canceled"),
            // TransmissionError::InvalidMode(m) => write!(f, "invalid x/y modem mode: {}", m),
            TransmissionError::InvalidSubpacket(m) => {
                write!(f, "don't understand subpacket {0}/x{0:X}", m)
            }
            TransmissionError::InvalidFrameType(ft) => write!(f, "invalid frame type {}", ft),
            TransmissionError::ZPADExected(b) => write!(
                f,
                "ZPAD expected got {} (0x{:X})",
                char::from_u32(*b as u32).unwrap(),
                b
            ),
            TransmissionError::ZLDEExected(b) => write!(
                f,
                "ZDLE expected got {} (0x{:X})",
                char::from_u32(*b as u32).unwrap(),
                b
            ),
            TransmissionError::UnknownHeaderType(ht) => write!(f, "unknown header type {}", ht),
            TransmissionError::CRC16Mismatch(crc, check_crc) => write!(
                f,
                "crc16 mismatch got {:04X} expected {:04X}",
                crc, check_crc
            ),
            TransmissionError::CRC32Mismatch(crc, check_crc) => write!(
                f,
                "crc32 mismatch got {:08X} expected {:08X}",
                crc, check_crc
            ),
            TransmissionError::ZDataBeforeZFILE => write!(f, "Got ZDATA before ZFILE"),
            TransmissionError::UnsupportedFrame(ft) => write!(f, "unsupported frame {:?}", ft),
            TransmissionError::HexNumberExpected => write!(f, "hex number expected"),
        }
    }
}

impl Error for TransmissionError {
    fn description(&self) -> &str {
        "use std::display"
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}
