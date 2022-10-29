
#[cfg(test)]
mod tests {
    use std::vec;

    use crate::{protocol::{Zmodem, ZCRCE, Protocol, FileDescriptor}, com::TestChannel};

    #[test]
    fn test_encode_subpckg_crc32() {
        let pck = Zmodem::encode_subpacket_crc32(ZCRCE, b"a\n");
        assert_eq!(vec![0x61, 0x0a, 0x18, 0x68, 0xe5, 0x79, 0xd2, 0x0f], pck);
    }

    fn create_channel() -> TestChannel {
        let  res = TestChannel::new();
       // setup_xmodem_cmds(&mut res.sender);
       // setup_xmodem_cmds(&mut res.receiver);
        res
    }
    
    #[test]
    fn test_zmodem_simple() {
        let mut send = Zmodem::new(512);
        let mut recv = Zmodem::new(512);
        
        let data = vec![1u8, 2, 5, 10];
        let mut com = create_channel();

        send.initiate_send(&mut com.sender, vec![FileDescriptor::create_test("foo.bar".to_string(), data.clone())]).expect("error.");
        recv.initiate_recv(&mut com.receiver).expect("error.");
        let mut i = 0;
        while send.is_active() || recv.is_active()  {
            i += 1;
            if i > 100 { break; }
            println!("sender:");
            send.update(&mut com.sender).expect("error.");
            println!("receiver:");
            recv.update(&mut com.receiver).expect("error.");
        }

        let rdata = recv.get_received_files();
        assert_eq!(1, rdata.len());
        assert_eq!("foo.bar", rdata[0].file_name.as_str());
        assert_eq!(&data, &rdata[0].get_data().unwrap());
    }
}