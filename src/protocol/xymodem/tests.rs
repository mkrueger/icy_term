#[cfg(test)]
use crate::protocol::Protocol;
#[cfg(test)]
use crate::{com::TestChannel, protocol::TransferState};

#[cfg(test)]
pub fn run_protocols(
    mut com: TestChannel,
    files: Vec<crate::protocol::FileDescriptor>,
    mut recv: Box<dyn Protocol>,
    mut send: Box<dyn Protocol>,
) -> crate::protocol::TestStorageHandler {
    use std::thread;

    use crate::protocol::TestStorageHandler;

    let handle1 = std::thread::spawn(move || {
        println!("start send thread.");
        let mut storage_handler: TestStorageHandler = TestStorageHandler::new();
        let mut transfer_state: TransferState = TransferState::default();
        send.initiate_send(&mut com.sender, files, &mut transfer_state)
            .expect("error.");
        while !transfer_state.is_finished {
            send.update(&mut com.sender, &mut transfer_state, &mut storage_handler)
                .expect("error.");
        }
        println!("end send thread.");
    });

    let handle2 = thread::spawn(move || {
        println!("start recv thread.");
        let mut storage_handler: TestStorageHandler = TestStorageHandler::new();
        let mut transfer_state: TransferState = TransferState::default();
        recv.initiate_recv(&mut com.receiver, &mut transfer_state)
            .expect("error.");

        while !transfer_state.is_finished {
            recv.update(&mut com.receiver, &mut transfer_state, &mut storage_handler)
                .expect("error.");
        }
        println!("end recv thread.");
        storage_handler
    });

    handle1.join().unwrap();
    handle2.join().unwrap()
}

#[cfg(test)]
mod xy_modem_tests {
    use crate::{
        com::TestChannel,
        protocol::{FileDescriptor, Protocol, XYModemVariant, XYmodem},
    };

    fn create_channel() -> TestChannel {
        let mut cmd_table = std::collections::HashMap::new();
        cmd_table.insert(b'C', "C".to_string());
        cmd_table.insert(b'G', "G".to_string());
        cmd_table.insert(0x04, "EOT".to_string());
        cmd_table.insert(0x06, "ACK".to_string());
        cmd_table.insert(0x15, "NAK".to_string());
        cmd_table.insert(0x18, "CAN".to_string());
        TestChannel::from_cmd_table(cmd_table, false)
    }

    #[test]
    fn test_xmodem_simple() {
        let send: Box<dyn Protocol> =
            Box::new(crate::protocol::XYmodem::new(XYModemVariant::XModem));
        let recv: Box<dyn Protocol> = Box::new(XYmodem::new(XYModemVariant::XModem));

        let data = vec![1u8, 2, 5, 10];
        let com = create_channel();
        let files = vec![FileDescriptor::create_test(
            "foo.bar".to_string(),
            data.clone(),
        )];

        let storage_handler =
            crate::protocol::xymodem::tests::run_protocols(com, files, recv, send);
        let recv_data: Vec<Vec<u8>> = storage_handler.file.values().cloned().collect();

        assert_eq!(1, recv_data.len());
        let send_data = &recv_data[0];
        assert_eq!(&data, send_data);
    }

    /*
    #[test]
    fn test_xmodem1k_simple() {
        let mut send = XYmodem::new(XYModemVariant::XModem1k);
        let mut recv = XYmodem::new(XYModemVariant::XModem1k);

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
        let mut recv_state = recv.initiate_recv(&mut com.receiver).expect("error.");
        let mut i = 0;
        while !send_state.is_finished || !recv_state.is_finished {
            i += 1;
            if i > 10 {
                break;
            }
            send.update(&mut com.sender, &mut send_state)
                .expect("error.");
            recv.update(&mut com.receiver, &mut recv_state)
                .expect("error.");
        }

        let rdata = recv.get_received_files();
        assert_eq!(1, rdata.len());
        let sdata = &rdata[0].get_data().unwrap();
        assert_eq!(&data, sdata);
    }

    #[test]
    fn test_xmodem1k_g_simple() {
        let mut send = XYmodem::new(XYModemVariant::XModem1kG);
        let mut recv = XYmodem::new(XYModemVariant::XModem1kG);

        let mut data = Vec::new();
        for i in 0..10 * 1024 {
            data.push(i as u8);
        }

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
        let mut recv_state = recv.initiate_recv(&mut com.receiver).expect("error.");
        let mut i = 0;
        while !send_state.is_finished || !recv_state.is_finished {
            i += 1;
            if i > 100 {
                break;
            }
            send.update(&mut com.sender, &mut send_state)
                .expect("error.");
            recv.update(&mut com.receiver, &mut recv_state)
                .expect("error.");
        }

        let rdata = recv.get_received_files();
        assert_eq!(1, rdata.len());
        let sdata = &rdata[0].get_data().unwrap();
        assert_eq!(&data, sdata);
    }

    #[test]
    fn test_xmodem_longer_file() {
        for test_len in [128, 255, 256, 2048, 4097] {
            let mut send = XYmodem::new(XYModemVariant::XModem);
            let mut recv = XYmodem::new(XYModemVariant::XModem);

            let mut data = Vec::new();
            for i in 0..test_len {
                data.push(i as u8);
            }

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
            let mut recv_state = recv.initiate_recv(&mut com.receiver).expect("error.");
            let mut i = 0;
            while !send_state.is_finished || !recv_state.is_finished {
                i += 1;
                if i > 100 {
                    break;
                }
                send.update(&mut com.sender, &mut send_state)
                    .expect("error.");
                recv.update(&mut com.receiver, &mut recv_state)
                    .expect("error.");
            }

            let rdata = recv.get_received_files();
            assert_eq!(1, rdata.len());
            let sdata = &rdata[0].get_data().unwrap();
            assert_eq!(&data, sdata);
        }
    }


    #[test]
    fn test_ymodem_simple() {
        let mut send = XYmodem::new(XYModemVariant::YModem);
        let mut recv = XYmodem::new(XYModemVariant::YModem);

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
        let mut recv_state = recv.initiate_recv(&mut com.receiver).expect("error.");
        let mut i = 0;
        while !send_state.is_finished || !recv_state.is_finished {
            i += 1;
            if i > 100 {
                break;
            }
            send.update(&mut com.sender, &mut send_state)
                .expect("error.");
            recv.update(&mut com.receiver, &mut recv_state)
                .expect("error.");
        }

        let rdata = recv.get_received_files();
        assert_eq!(1, rdata.len());
        assert_eq!(&data, &rdata[0].get_data().unwrap());
    }

    #[test]
    fn test_ymodem_g_simple() {
        let mut send = XYmodem::new(XYModemVariant::YModemG);
        let mut recv = XYmodem::new(XYModemVariant::YModemG);

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
        let mut recv_state = recv.initiate_recv(&mut com.receiver).expect("error.");
        let mut i = 0;
        while !send_state.is_finished || !recv_state.is_finished {
            i += 1;
            if i > 100 {
                break;
            }
            send.update(&mut com.sender, &mut send_state)
                .expect("error.");
            recv.update(&mut com.receiver, &mut recv_state)
                .expect("error.");
        }

        let rdata = recv.get_received_files();
        assert_eq!(1, rdata.len());
        assert_eq!(&data, &rdata[0].get_data().unwrap());
    }

    #[test]
    fn test_ymodem_batch() {
        let mut send = XYmodem::new(XYModemVariant::YModem);
        let mut recv = XYmodem::new(XYModemVariant::YModem);

        let data1 = vec![1u8, 2, 5, 10];
        let data2 = vec![1u8, 42, 18, 19];
        let mut com = create_channel();
        let mut send_state = send
            .initiate_send(
                &mut com.sender,
                vec![
                    FileDescriptor::create_test("foo.bar".to_string(), data1.clone()),
                    FileDescriptor::create_test("baz".to_string(), data2.clone()),
                ],
            )
            .expect("error.");

        let mut recv_state = recv.initiate_recv(&mut com.receiver).expect("error.");
        let mut i = 0;
        while !send_state.is_finished || !recv_state.is_finished {
            i += 1;
            if i > 100 {
                break;
            }
            send.update(&mut com.sender, &mut send_state)
                .expect("error.");
            recv.update(&mut com.receiver, &mut recv_state)
                .expect("error.");
        }

        let rdata = recv.get_received_files();
        assert_eq!(2, rdata.len());

        assert_eq!(&data1, &rdata[0].get_data().unwrap());
        assert_eq!(data1.len(), rdata[0].size);

        assert_eq!(&data2, &rdata[1].get_data().unwrap());
        assert_eq!(data2.len(), rdata[1].size);
    }

    */
}
