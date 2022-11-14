#[cfg(test)]
mod tests {
    use std::vec;

    use crate::{
        com::test_com::TestChannel,
        protocol::{
            str_from_null_terminated_utf8_unchecked, zmodem::rz::read_subpacket, FileDescriptor,
            FrameType, Header, HeaderType, Protocol, Zmodem, ZCRCE,
        },
    };

    #[test]
    fn test_encode_subpckg_crc32() {
        let pck = Zmodem::encode_subpacket_crc32(ZCRCE, b"a\n");
        assert_eq!(vec![0x61, 0x0a, 0x18, 0x68, 0xe5, 0x79, 0xd2, 0x0f], pck);
    }

    fn create_channel() -> TestChannel {
        let res = TestChannel::new();
        //setup_xmodem_cmds(&mut res.sender);
        //setup_xmodem_cmds(&mut res.receiver);
        res
    }

    #[test]
    fn test_zmodem_simple() {
        let mut send = Zmodem::new(512);
        let mut recv = Zmodem::new(512);

        let data = vec![1u8, 2, 5, 10];
        let mut com = create_channel();

        let mut send_state = send
            .initiate_send(
                &mut com.sender,
                vec![FileDescriptor::create_test(
                    "foo.bar".to_string(),
                    data.clone(),
                )],
            )
            .expect("error.");
        let mut revc_state = recv.initiate_recv(&mut com.receiver).expect("error.");
        let mut i = 0;
        while !send_state.is_finished || !revc_state.is_finished {
            i += 1;
            if i > 100 {
                break;
            }
            send.update(&mut com.sender, &mut send_state)
                .expect("error.");
            recv.update(&mut com.receiver, &mut revc_state)
                .expect("error.");
        }

        let rdata = recv.get_received_files();
        assert_eq!(1, rdata.len());
        assert_eq!("foo.bar", rdata[0].file_name.as_str());
        assert_eq!(&data, &rdata[0].get_data().unwrap());
    }

    #[test]
    fn test_zmodem_simple_send() {
        let mut send = Zmodem::new(512);

        let data = vec![1u8, 2, 5, 10];
        let mut com = create_channel();

        let mut send_state = send
            .initiate_send(
                &mut com.sender,
                vec![FileDescriptor::create_test(
                    "foo.bar".to_string(),
                    data.clone(),
                )],
            )
            .expect("error.");
        let mut can_count = 0;
        // sender          receiver
        // ZRQINIT(0)
        //                 ZRINIT
        // ZFILE
        //                 ZRPOS
        // ZDATA data…
        // ZEOF
        //                 ZRINIT
        // ZFIN
        //                 ZFIN
        // OO
        send.update(&mut com.sender, &mut send_state)
            .expect("error.");
        let header = Header::read(&mut com.receiver, &mut can_count)
            .unwrap()
            .unwrap();
        assert_eq!(FrameType::ZRQINIT, header.frame_type);
        Header::from_flags(HeaderType::Hex, FrameType::ZRINIT, 0, 0, 0, 0x23)
            .write(&mut com.receiver)
            .unwrap();

        send.update(&mut com.sender, &mut send_state)
            .expect("error.");
        let header = Header::read(&mut com.receiver, &mut can_count)
            .unwrap()
            .unwrap();
        assert_eq!(FrameType::ZFILE, header.frame_type);
        let (block, _, _) = read_subpacket(
            &mut com.receiver,
            1024,
            header.header_type == HeaderType::Bin32,
        )
        .unwrap();
        let file_name = str_from_null_terminated_utf8_unchecked(&block).to_string();
        assert_eq!("foo.bar", file_name);
        Header::from_number(HeaderType::Hex, FrameType::ZRPOS, 0)
            .write(&mut com.receiver)
            .unwrap();

        send.update(&mut com.sender, &mut send_state)
            .expect("error.");
        send.update(&mut com.sender, &mut send_state)
            .expect("error.");
        let header = Header::read(&mut com.receiver, &mut can_count)
            .unwrap()
            .unwrap();
        assert_eq!(FrameType::ZDATA, header.frame_type);
        let (block, last, _) = read_subpacket(
            &mut com.receiver,
            1024,
            header.header_type == HeaderType::Bin32,
        )
        .unwrap();
        assert_eq!(true, last);
        assert_eq!(data, block);

        send.update(&mut com.sender, &mut send_state)
            .expect("error.");
        let header = Header::read(&mut com.receiver, &mut can_count)
            .unwrap()
            .unwrap();
        assert_eq!(FrameType::ZEOF, header.frame_type);
        Header::from_flags(HeaderType::Hex, FrameType::ZRINIT, 0, 0, 0, 0x23)
            .write(&mut com.receiver)
            .unwrap();

        send.update(&mut com.sender, &mut send_state)
            .expect("error.");
        let header = Header::read(&mut com.receiver, &mut can_count)
            .unwrap()
            .unwrap();
        assert_eq!(FrameType::ZFIN, header.frame_type);
    }
}
