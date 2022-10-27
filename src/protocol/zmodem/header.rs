use std::{io, time::Duration};

use crate::{crc16, com::Com, protocol::{XON, frame_types::ZACK}};

use super::{ZPAD, ZDLE, ZBIN, ZBIN32, ZHEX, get_hex, from_hex};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum HeaderType {
    Bin,
    Bin32,
    Hex
}

#[derive(PartialEq, Clone, Debug)]
pub struct Header {
    pub header_type: HeaderType,
    pub frame_type: u8,
    pub data: [u8; 4]
}

impl Header {

    pub fn empty(header_type: HeaderType, frame_type: u8) -> Self
    {
        Self {
            header_type,
            frame_type,
            data: [0, 0, 0, 0]
        }
    }

    pub fn from_flags(header_type: HeaderType, frame_type: u8, f3: u8, f2: u8, f1: u8, f0: u8) -> Self
    {
        Self {
            header_type,
            frame_type,
            data: [f3, f2, f1, f0]
        }
    }

    pub fn from_number(header_type: HeaderType, frame_type: u8, number: u32) -> Self
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

    pub fn _f1(&self) -> u8 {
        self.data[2]
    }

    pub fn _f2(&self) -> u8 {
        self.data[1]
    }

    pub fn _f3(&self) -> u8 {
        self.data[0]
    }

    pub fn number(&self) -> u32 {
        u32::from_le_bytes(self.data)
    }

    pub fn build(&self) -> Vec<u8> {
        let mut res = Vec::new();

        match self.header_type {
            HeaderType::Bin => {
                res.extend_from_slice(&[ZPAD, ZDLE, ZBIN, self.frame_type]);
                res.extend_from_slice(&self.data);
                let crc16 = crc16::get_crc16(&res[3..]);
                res.extend_from_slice(&u16::to_le_bytes(crc16));
            }

            HeaderType::Bin32 => {
                res.extend_from_slice(&[ZPAD, ZDLE, ZBIN32, self.frame_type]);
                res.extend_from_slice(&self.data);
                let crc32 = crc16::get_crc32(&res[3..]);
                res.extend_from_slice(&u32::to_le_bytes(crc32));
            }
            
            HeaderType::Hex => {
                res.extend_from_slice(&[ZPAD, ZPAD, ZDLE, ZHEX]);
                res.push(get_hex((self.frame_type >> 4)& 0xF));
                res.push(get_hex(self.frame_type & 0xF));
                println!("{}", get_hex(self.frame_type & 0xF));
        
                for f in self.data {
                    res.push(get_hex((f >> 4) & 0xF));
                    res.push(get_hex(f & 0xF));
                }

                let mut crc16 = crc16::update_crc16(0,self.frame_type);
                for b in self.data {
                    crc16 = crc16::update_crc16(crc16, b);
                }
                res.push(get_hex((crc16 >> 12) as u8 & 0xF));
                res.push(get_hex((crc16 >> 8) as u8 & 0xF));
                res.push(get_hex((crc16 >> 4) as u8 & 0xF));
                res.push(get_hex((crc16 & 0xF) as u8));
                res.push(b'\n'); // only 1 is required, 2 if it starts with \r then windows EOL is expected
                if self.frame_type != ZACK && self.frame_type != ZBIN {
                    res.push(XON);
                }
            }
        }
        res
    }

    pub fn write<T: Com>(&mut self, com: &mut T) -> io::Result<usize>
    {
        com.write(&self.build())
    }

    pub fn read<T: Com>(com: &mut T) -> io::Result<Option<Header>> {
        if com.is_data_available()? {
            let zpad = com.read_char(Duration::from_secs(5))?;
            if zpad != ZPAD {
                com.discard_buffer()?;
                return Err(io::Error::new(io::ErrorKind::InvalidData, "ZPAD expected"));
            }
            let mut next = com.read_char(Duration::from_secs(5))?;
            if next == ZPAD {
                next = com.read_char(Duration::from_secs(5))?;
            }
            if next != ZDLE {
                com.discard_buffer()?;
                return Err(io::Error::new(io::ErrorKind::InvalidData, "ZDLE expected"));
            }

            let header_type = com.read_char(Duration::from_secs(5))?;
            let header_data_size = match header_type {
                ZBIN => 7,
                ZBIN32 => 9,
                ZHEX => 14,
                _ => {
                    com.discard_buffer()?;
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Unknown header type"))
                }
            };
            let header_data = com.read_exact(Duration::from_secs(5), header_data_size)?;
            match header_type {
                ZBIN => {
                    let crc16 = crc16::get_crc16(&header_data[0..5]);
                    let check_crc16 = u16::from_le_bytes(header_data[5..7].try_into().unwrap());
                    if crc16 != check_crc16 {
                        com.discard_buffer()?;
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "CRC16 mismatch"));
                    }
                    Ok(Some(Header {
                        header_type: HeaderType::Bin,
                        frame_type: header_data[0],
                        data: header_data[1..5].try_into().unwrap(),
                    }))
                }
                ZBIN32 => {
                    let data = &header_data[0..5];
                    let crc32 = crc16::get_crc32(&data);
                    let check_crc32 = u32::from_le_bytes(header_data[5..9].try_into().unwrap());
                    if crc32 != check_crc32 {
                        com.discard_buffer()?;
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "CRC32 mismatch"));
                    }
                    Ok(Some(Header {
                        header_type: HeaderType::Bin32,
                        frame_type: header_data[0],
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

                    let crc16 = crc16::get_crc16(&data);

                    let mut check_crc16 = 0 ;
                    for b in &header_data[10..14] {
                        check_crc16 = check_crc16  << 4 | (from_hex(*b)? as u16);
                    }
                    if crc16 != check_crc16 {
                        com.discard_buffer()?;
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "CRC32 mismatch"));
                    }
                    // read rest
                    let eol  = com.read_char(Duration::from_secs(5))?;

                    // don't check the next bytes. Errors there don't impact much
                    if eol == b'\r' {
                        com.read_char(Duration::from_secs(5))?; // \n windows eol
                    }
                    if data[0] != ZACK && data[0] != ZBIN {
                        com.read_char(Duration::from_secs(5))?; // read XON
                    }
    
                    Ok(Some(Header {
                        header_type: HeaderType::Hex,
                        frame_type: data[0],
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
    use zmodem::frame_types::{ZDATA, ZRPOS};

    use crate::{protocol::{*, zmodem::header::{HeaderType, Header}}, com::{TestChannel}};

    #[test]
    fn test_bin_header_data() {
        assert_eq!(
            Header::empty(HeaderType::Bin, 0).build(),
                vec![ZPAD, ZDLE, ZBIN, 0, 0, 0, 0, 0, 0, 0]);

        assert_eq!(
            Header::from_flags(HeaderType::Bin, 0, 1, 1, 1, 1).build(),
                vec![ZPAD, ZDLE, ZBIN, 0, 1, 1, 1, 1, 148, 98]);
    }

    #[test]
    fn test_bin32_header_data() {
        assert_eq!(
            Header::empty(HeaderType::Bin32, ZDATA).build(),
                vec![ZPAD, ZDLE, ZBIN32, 0x0A, 0, 0, 0, 0, 0xBC, 0xEF, 0x92, 0x8C]);
    }

    #[test]
    fn test_hex_header_data() {
        assert_eq!(
            Header::empty(HeaderType::Hex, ZRPOS).build(),
                vec![ZPAD, ZPAD, ZDLE, ZHEX, b'0', b'9', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'a', b'8', b'7', b'c', b'\n', 0x11]);
        assert_eq!(
            Header::from_flags(HeaderType::Hex, 0, 1, 1, 1, 1).build(),
                vec![ZPAD, ZPAD, ZDLE, ZHEX,
                b'0', b'0',
                b'0', b'1',
                b'0', b'1',
                b'0', b'1',
                b'0', b'1',
                54, 50, 57, 52,
                b'\n', XON]);
    }

    #[test]
    fn test_bin_header() {
        let mut com = TestChannel::new();
        let header = Header::from_flags(HeaderType::Bin, 43, 3, 2, 1, 0);
        com.sender.write(&header.build()).expect("err");
        let read_header = Header::read(&mut com.receiver).unwrap().unwrap();
        assert_eq!(read_header, header);
    }

    #[test]
    fn test_bin32_header() {
        let mut com = TestChannel::new();
        let header = Header::from_flags(HeaderType::Bin32, 43, 3, 2, 1, 0);
        com.sender.write(&header.build()).expect("err");
        let read_header = Header::read(&mut com.receiver).unwrap().unwrap();
        assert_eq!(read_header, header);
    }

    #[test]
    fn test_hex_header() {
        let mut com = TestChannel::new();
        let header = Header::from_flags(HeaderType::Hex, 43, 3, 2, 1, 0);
        com.sender.write(&header.build()).expect("err");

        let read_header = Header::read(&mut com.receiver).unwrap().unwrap();
        assert_eq!(read_header, header);
    }

}
