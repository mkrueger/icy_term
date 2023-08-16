#[cfg(test)]
mod zmodem_test {
    use crate::{
        com::TestChannel,
        protocol::{zmodem::rz::read_subpacket, *},
    };

    fn create_channel() -> TestChannel {
        TestChannel::new()
    }

    #[test]
    fn test_encode_subpckg_crc32() {
        let pck = Zmodem::encode_subpacket_crc32(ZCRCE, b"a\n");
        assert_eq!(vec![0x61, 0x0a, 0x18, 0x68, 0xe5, 0x79, 0xd2, 0x0f], pck);
    }

    #[test]
    fn test_zmodem_simple() {
        return;
        let send: Box<dyn Protocol> = Box::new(Zmodem::new(512));
        let recv: Box<dyn Protocol> = Box::new(Zmodem::new(512));

        let data: Vec<u8> = vec![1u8, 2, 5, 10];
        let com = create_channel();
        let files = vec![FileDescriptor::create_test(
            "foo.bar".to_string(),
            data.clone(),
        )];
        let storage_handler =
            crate::protocol::xymodem::tests::run_protocols(com, files, recv, send);

        assert_eq!(1, storage_handler.file.len());
        for (file_name, content) in &storage_handler.file {
            assert_eq!("foo.bar", file_name);
            assert_eq!(&data, content);
        }
    }

    #[test]
    fn test_zmodem_simple_send() {
        let mut send = Zmodem::new(512);

        let data = vec![1u8, 2, 5, 10];
        let mut com = create_channel();
        let mut transfer_state = TransferState::new();

        send.initiate_send(
            &mut com.sender,
            vec![FileDescriptor::create_test(
                "foo.bar".to_string(),
                data.clone(),
            )],
            &mut transfer_state,
        )
        .expect("error.");
        let mut handler: TestStorageHandler = TestStorageHandler::new();

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
        send.update(&mut com.sender, &mut transfer_state, &mut handler)
            .expect("error.");
        let header = Header::read(&mut com.receiver, &mut can_count)
            .unwrap()
            .unwrap();
        assert_eq!(ZFrameType::RQInit, header.frame_type);
        Header::from_flags(HeaderType::Hex, ZFrameType::RIinit, 0, 0, 0, 0x23)
            .write(&mut com.receiver)
            .unwrap();

        send.update(&mut com.sender, &mut transfer_state, &mut handler)
            .expect("error.");
        let header = Header::read(&mut com.receiver, &mut can_count)
            .unwrap()
            .unwrap();
        assert_eq!(ZFrameType::File, header.frame_type);
        let (block, _, _) = read_subpacket(
            &mut com.receiver,
            1024,
            header.header_type == HeaderType::Bin32,
        )
        .unwrap();
        let file_name = str_from_null_terminated_utf8_unchecked(&block).to_string();
        assert_eq!("foo.bar", file_name);
        Header::from_number(HeaderType::Hex, ZFrameType::RPos, 0)
            .write(&mut com.receiver)
            .unwrap();

        send.update(&mut com.sender, &mut transfer_state, &mut handler)
            .expect("error.");
        send.update(&mut com.sender, &mut transfer_state, &mut handler)
            .expect("error.");
        let header = Header::read(&mut com.receiver, &mut can_count)
            .unwrap()
            .unwrap();
        assert_eq!(ZFrameType::Data, header.frame_type);

        send.update(&mut com.sender, &mut transfer_state, &mut handler)
            .expect("error.");

        match read_subpacket(
            &mut com.receiver,
            1024,
            header.header_type == HeaderType::Bin32,
        ) {
            Ok((block_data, last, _)) => {
                assert!(last);
                assert_eq!(data, block_data);
            }
            Err(err) => {
                panic!("error reading subpacket: {err:?}");
            }
        }

        send.update(&mut com.sender, &mut transfer_state, &mut handler)
            .expect("error.");
        let header = Header::read(&mut com.receiver, &mut can_count)
            .unwrap()
            .unwrap();
        assert_eq!(ZFrameType::Eof, header.frame_type);
        Header::from_flags(HeaderType::Hex, ZFrameType::RIinit, 0, 0, 0, 0x23)
            .write(&mut com.receiver)
            .unwrap();

        send.update(&mut com.sender, &mut transfer_state, &mut handler)
            .expect("error.");
        let header = Header::read(&mut com.receiver, &mut can_count)
            .unwrap()
            .unwrap();
        assert_eq!(ZFrameType::Fin, header.frame_type);
    }
}
