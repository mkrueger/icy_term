use std::{io::{self, ErrorKind}, time::Duration, fmt::Display};

use icy_engine::{get_crc16, get_crc32, update_crc16};

use crate::{com::Com, protocol::{XON, frame_types::{ZACK}}};

use super::{ZPAD, ZDLE, ZBIN, ZBIN32, ZHEX, get_hex, from_hex, frame_types::{self}, append_zdle_encoded, read_zdle_bytes};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum HeaderType {
    Bin,
    Bin32,
    Hex
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum FrameType {
    ZRQINIT = 0,      // Request receive init 
    ZRINIT = 1,       // Receive init
    ZSINIT = 2,       // Send init sequence (optional)
    ZACK = 3,         // ACK to above
    ZFILE = 4,        // File name from sender
    ZSKIP = 5,        // To sender: skip this file
    ZNAK = 6,         // Last packet was garbled
    ZABORT = 7,       // Abort batch transfers
    ZFIN = 8,         // Finish session
    ZRPOS = 9,        // Resume data trans at this position
    ZDATA = 10,       // Data packet(s) follow
    ZEOF = 11,        // End of file
    ZFERR = 12,       // Fatal Read or Write error Detected
    ZCRC = 13,        // Request for file CRC and response
    ZCHALLENGE = 14,  // Receiver's Challenge
    ZCOMPL = 15,      // Request is complete
    ZCAN = 16,        // Other end canned session with CAN*5
    ZFREECNT = 17,    // Request for free bytes on filesystem
    ZCOMMAND = 18,    // Command from sending program
    ZSTDERR = 19	  // Output to standard error, data follows
}

#[derive(PartialEq, Clone, Debug)]
pub struct Header {
    pub header_type: HeaderType,
    pub frame_type: FrameType,
    pub data: [u8; 4]
}

impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        match self.frame_type {
            FrameType::ZRPOS | 
            FrameType::ZEOF | 
            FrameType::ZFREECNT | 
            FrameType::ZDATA =>  write!(f,"[{:?} Header with {:?} number = {}]", self.header_type, self.frame_type, self.number()),
            FrameType::ZCRC | 
            FrameType::ZCHALLENGE =>  write!(f,"[{:?} Header with {:?} number = x{:08X}]", self.header_type, self.frame_type, self.number()),
            _ => write!(f,"[{:?} Header with {:?} frame flags = x{:02X}, x{:02X}, x{:02X}, x{:02X}]", self.header_type, self.frame_type, self.f3(), self.f2(), self.f1(), self.f0())
        }
    }
}

impl Header {
    pub fn empty(header_type: HeaderType, frame_type: FrameType) -> Self
    {
        Self {
            header_type,
            frame_type,
            data: [0, 0, 0, 0]
        }
    }

    pub fn from_flags(header_type: HeaderType, frame_type: FrameType, f3: u8, f2: u8, f1: u8, f0: u8) -> Self
    {
        Self {
            header_type,
            frame_type,
            data: [f3, f2, f1, f0]
        }
    }

    pub fn from_number(header_type: HeaderType, frame_type: FrameType, number: u32) -> Self
    {
        Self {
            header_type,
            frame_type,
            data: u32::to_le_bytes(number)
        }
    }

    pub fn f0(&self) -> u8 {
        self.data[3]
    }

    pub fn f1(&self) -> u8 {
        self.data[2]
    }

    pub fn f2(&self) -> u8 {
        self.data[1]
    }

    pub fn f3(&self) -> u8 {
        self.data[0]
    }

    pub fn number(&self) -> u32 {
        u32::from_le_bytes(self.data)
    }

    pub fn build(&self) -> Vec<u8> {
        let mut res = Vec::new();

        match self.header_type {
            HeaderType::Bin => {
                res.extend_from_slice(&[ZPAD, ZDLE, ZBIN, self.frame_type as u8]);
                append_zdle_encoded(&mut res, &self.data);
                let crc16 = get_crc16(&res[3..]);
                append_zdle_encoded(&mut res, &u16::to_le_bytes(crc16));
            }

            HeaderType::Bin32 => {
                res.extend_from_slice(&[ZPAD, ZDLE, ZBIN32, self.frame_type as u8]);
                append_zdle_encoded(&mut res, &self.data);
                let crc32 = get_crc32(&res[3..]);
                append_zdle_encoded(&mut res, &u32::to_le_bytes(crc32));
            }
            
            HeaderType::Hex => {
                res.extend_from_slice(&[ZPAD, ZPAD, ZDLE, ZHEX]);
                let ft = self.frame_type as u8;
                res.push(get_hex((ft >> 4)& 0xF));
                res.push(get_hex(ft & 0xF));
        
                for f in self.data {
                    res.push(get_hex((f >> 4) & 0xF));
                    res.push(get_hex(f & 0xF));
                }

                let mut crc16 = update_crc16(0,self.frame_type as u8);
                for b in self.data {
                    crc16 = update_crc16(crc16, b);
                }
                res.push(get_hex((crc16 >> 12) as u8 & 0xF));
                res.push(get_hex((crc16 >> 8) as u8 & 0xF));
                res.push(get_hex((crc16 >> 4) as u8 & 0xF));
                res.push(get_hex((crc16 & 0xF) as u8));
                res.push(b'\n'); // only 1 is required, 2 if it starts with \r then windows EOL is expected
                if self.frame_type != FrameType::ZACK && self.frame_type != FrameType::ZFIN {
                    res.push(XON);
                }
            }
        }
        res
    }

    pub fn write(&mut self, com: &mut Box<dyn Com>) -> io::Result<()>
    {
        println!("Send Header: {}", self);
        com.write(&self.build())
    }
    
    pub fn get_frame_type(ftype: u8) -> io::Result<FrameType>
    {
        match ftype {
            frame_types::ZRQINIT => Ok(FrameType::ZRQINIT),
            frame_types::ZRINIT => Ok(FrameType::ZRINIT),
            frame_types::ZSINIT => Ok(FrameType::ZSINIT),
            frame_types::ZACK => Ok(FrameType::ZACK),
            frame_types::ZFILE => Ok(FrameType::ZFILE),
            frame_types::ZSKIP => Ok(FrameType::ZSKIP),
            frame_types::ZNAK => Ok(FrameType::ZNAK),
            frame_types::ZABORT => Ok(FrameType::ZABORT),
            frame_types::ZFIN => Ok(FrameType::ZFIN),
            frame_types::ZRPOS => Ok(FrameType::ZRPOS),
            frame_types::ZDATA => Ok(FrameType::ZDATA),
            frame_types::ZEOF => Ok(FrameType::ZEOF),
            frame_types::ZFERR => Ok(FrameType::ZFERR),
            frame_types::ZCRC => Ok(FrameType::ZCRC),
            frame_types::ZCHALLENGE => Ok(FrameType::ZCHALLENGE),
            frame_types::ZCOMPL => Ok(FrameType::ZCOMPL),
            frame_types::ZCAN => Ok(FrameType::ZCAN),
            frame_types::ZFREECNT => Ok(FrameType::ZFREECNT),
            frame_types::ZCOMMAND => Ok(FrameType::ZCOMMAND),
            frame_types::ZSTDERR => Ok(FrameType::ZSTDERR),
            _ => Err(io::Error::new(ErrorKind::InvalidInput, format!("Invalid frame type {}", ftype)))
        }

    }

    pub fn read(com: &mut Box<dyn Com>, can_count: &mut usize) -> io::Result<Option<Header>> {
        if com.is_data_available()? {
            let zpad = com.read_char(Duration::from_secs(5))?;
            if zpad == 0x18 { // CAN
                *can_count += 1;
            }
            if zpad != ZPAD {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("ZPAD expected got {} (0x{:X}) ", char::from_u32(zpad as u32).unwrap(), zpad)));
            }
            *can_count = 0;
            let mut next = com.read_char(Duration::from_secs(5))?;
            if next == ZPAD {
                next = com.read_char(Duration::from_secs(5))?;
            }
            if next != ZDLE {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "ZDLE expected"));
            }

            let header_type = com.read_char(Duration::from_secs(5))?;
            let header_data_size = match header_type {
                ZBIN => 7,
                ZBIN32 => 9,
                ZHEX => 14,
                _ => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Unknown header type"))
                }
            };
            
            let header_data = read_zdle_bytes(com, header_data_size)?;
            match header_type {
                ZBIN => {
                    let crc16 = get_crc16(&header_data[0..5]);
                    let check_crc16 = u16::from_le_bytes(header_data[5..7].try_into().unwrap());
                    if crc16 != check_crc16 {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "CRC16 mismatch"));
                    }
                    Ok(Some(Header {
                        header_type: HeaderType::Bin,
                        frame_type: Header::get_frame_type(header_data[0])?,
                        data: header_data[1..5].try_into().unwrap(),
                    }))
                }
                ZBIN32 => {
                    let data = &header_data[0..5];
                    let crc32 = get_crc32(&data);
                    let check_crc32 = u32::from_le_bytes(header_data[5..9].try_into().unwrap());
                    if crc32 != check_crc32 {
                        for b in header_data  {
                            print!("{:02X},",b);
                        }
                        println!();
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "CRC32 mismatch"));
                    }
                    Ok(Some(Header {
                        header_type: HeaderType::Bin32,
                        frame_type: Header::get_frame_type(header_data[0])?,
                        data: header_data[1..5].try_into().unwrap(),
                    }))
                }
                ZHEX => {
                    let data = [
                        from_hex(header_data[0])? << 4 | from_hex(header_data[1])?,
                        from_hex(header_data[2])? << 4 | from_hex(header_data[3])?,
                        from_hex(header_data[4])? << 4 | from_hex(header_data[5])?,
                        from_hex(header_data[6])? << 4 | from_hex(header_data[7])?,
                        from_hex(header_data[8])? << 4 | from_hex(header_data[9])?
                    ];

                    let crc16 = get_crc16(&data);

                    let mut check_crc16 = 0 ;
                    for b in &header_data[10..14] {
                        check_crc16 = check_crc16  << 4 | (from_hex(*b)? as u16);
                    }
                    if crc16 != check_crc16 {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "CRC32 mismatch"));
                    }
                    // read rest
                    let eol  = com.read_char(Duration::from_secs(5))?;

                    // don't check the next bytes. Errors there don't impact much
                    if eol == b'\r' {
                        com.read_char(Duration::from_secs(5))?; // \n windows eol
                    }
                    if data[0] != ZACK && data[0] != frame_types::ZFIN {
                        com.read_char(Duration::from_secs(5))?; // read XON
                    }
    
                    Ok(Some(Header {
                        header_type: HeaderType::Hex,
                        frame_type: Header::get_frame_type(data[0])?,
                        data: data[1..5].try_into().unwrap(),
                    }))
                }
                _ => { panic!("should never happen"); }
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{protocol::{*, zmodem::header::{HeaderType, Header}}};

    #[test]
    fn test_from_number() {
        assert_eq!(
            Header::from_number(HeaderType::Hex, FrameType::ZFIN, 2).build(),
            vec![0x2a, 0x2a, 0x18, 0x42, 0x30, 0x38, 0x30, 0x32, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x65, 0x66, 0x34, 0x35, 0x0a]);
    }

    #[test]
    fn test_bin32_header_data() {
        assert_eq!(
            Header::empty(HeaderType::Bin32, FrameType::ZDATA).build(),
                vec![ZPAD, ZDLE, ZBIN32, 0x0A, 0, 0, 0, 0, 0xBC, 0xEF, 0x92, 0x8C]);
    }

    #[test]
    fn test_hex_header_data() {
        assert_eq!(
            Header::empty(HeaderType::Hex, FrameType::ZRPOS).build(),
                vec![ZPAD, ZPAD, ZDLE, ZHEX, b'0', b'9', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'a', b'8', b'7', b'c', b'\n', 0x11]);

            assert_eq!("**\x18B00000000000000\n\x11".to_string(), 
                String::from_utf8(Header::from_flags(HeaderType::Hex, FrameType::ZRQINIT, 0, 0, 0, 0).build()).unwrap());
            assert_eq!("**\x18B0100000000aa51\n\x11".to_string(), 
                String::from_utf8(Header::from_flags(HeaderType::Hex, FrameType::ZRINIT, 0, 0, 0, 0).build()).unwrap());
            assert_eq!("**\x18B02000000004483\n\x11".to_string(), 
                String::from_utf8(Header::from_flags(HeaderType::Hex, FrameType::ZSINIT, 0, 0, 0, 0).build()).unwrap());
            assert_eq!("**\x18B0300000000eed2\n".to_string(), 
                String::from_utf8(Header::from_flags(HeaderType::Hex, FrameType::ZACK, 0, 0, 0, 0).build()).unwrap());
            assert_eq!("**\x18B087e0400003ec2\n".to_string(), 
                String::from_utf8(Header::from_flags(HeaderType::Hex, FrameType::ZFIN, 126, 4, 0, 0).build()).unwrap());
    }
/*
    #[test]
    fn test_bin_header() {
        let mut com = TestChannel::new();
        let header = Header::from_flags(HeaderType::Bin, FrameType::ZDATA, 3, 2, 1, 0);
        let mut i = 0;
        com.sender.write(&header.build()).expect("err");
        let read_header = Header::read(&mut com.receiver, &mut i).unwrap().unwrap();
        assert_eq!(read_header, header);
    }

    #[test]
    fn test_bin32_header() {
        let mut com = TestChannel::new();
        let header = Header::from_flags(HeaderType::Bin32, FrameType::ZDATA, 3, 2, 1, 0);
        com.sender.write(&header.build()).expect("err");
        let mut i = 0;
        let read_header = Header::read(&mut com.receiver, &mut i).unwrap().unwrap();
        assert_eq!(read_header, header);
    }

    #[test]
    fn test_hex_header() {
        let mut com = TestChannel::new();
        let header = Header::from_flags(HeaderType::Hex, FrameType::ZDATA, 3, 2, 1, 0);
        com.sender.write(&header.build()).expect("err");
        let mut i = 0;

        let read_header = Header::read(&mut com.receiver, &mut i).unwrap().unwrap();
        assert_eq!(read_header, header);
    }
 */
}
