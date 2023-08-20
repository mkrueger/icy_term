#![allow(unsafe_code, clippy::wildcard_imports)]

use std::collections::VecDeque;
use std::sync::mpsc::{self};

#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;
use web_time::Instant;

use crate::com::{Com, TermComResult};
use crate::protocol::TestStorageHandler;

use super::connection::{Connection, OpenConnectionData, SendData};
use super::MainWindow;

const BITS_PER_BYTE: u32 = 8;

pub struct ConnectionThreadData {
    tx: mpsc::Sender<SendData>,
    rx: mpsc::Receiver<SendData>,
    com: Box<dyn Com>,
    thread_is_running: bool,
    is_connected: bool,

    // used for baud rate emulation
    data_buffer: VecDeque<u8>,
    baud_rate: u32,
    last_send_time: Instant,
}

impl ConnectionThreadData {
    fn disconnect(&mut self) {
        self.is_connected = false;
        self.com = Box::new(crate::com::NullConnection {});
        self.baud_rate = 0;
        self.data_buffer.clear();
    }

    fn read_data(&mut self) -> bool {
        if self.data_buffer.is_empty() {
            if let Ok(Some(data)) = self.com.read_data() {
                if self.baud_rate == 0 {
                    if let Err(err) = self.tx.send(SendData::Data(data)) {
                        log::error!("{err}");
                        self.thread_is_running &= self.tx.send(SendData::Disconnect).is_ok();
                    }
                    // ctx.request_repaint();
                } else {
                    self.data_buffer.extend(data);
                }
            } else {
                return false;
            }
        } else if self.baud_rate == 0 {
            if let Err(err) = self
                .tx
                .send(SendData::Data(self.data_buffer.drain(..).collect()))
            {
                log::error!("{err}");
                self.thread_is_running &= self.tx.send(SendData::Disconnect).is_ok();
                self.disconnect();
            }
        } else {
            let cur_time = Instant::now();
            let bytes_per_sec = self.baud_rate / BITS_PER_BYTE;
            let elapsed_ms = cur_time.duration_since(self.last_send_time).as_millis() as u32;
            let bytes_to_send: usize =
                ((bytes_per_sec * elapsed_ms) / 1000).min(self.data_buffer.len() as u32) as usize;

            if bytes_to_send > 0 {
                if let Err(err) = self.tx.send(SendData::Data(
                    self.data_buffer.drain(..bytes_to_send).collect(),
                )) {
                    log::error!("{err}");
                    self.thread_is_running &= self.tx.send(SendData::Disconnect).is_ok();
                }
                self.last_send_time = cur_time;
            }
        }
        true
    }

    fn try_connect(&mut self, connection_data: &OpenConnectionData) -> TermComResult<()> {
        self.com = match connection_data.protocol {
            crate::addresses::Protocol::Telnet => {
                Box::new(crate::com::ComTelnetImpl::connect(connection_data)?)
            }
            crate::addresses::Protocol::Raw => {
                Box::new(crate::com::ComRawImpl::connect(connection_data)?)
            }
            #[cfg(not(target_arch = "wasm32"))]
            crate::addresses::Protocol::Ssh => {
                Box::new(crate::com::ssh::SSHComImpl::connect(connection_data)?)
            }

            #[cfg(target_arch = "wasm32")]
            crate::addresses::Protocol::Ssh => Box::new(crate::com::NullConnection {}),
        };
        Ok(())
    }

    pub fn handle_receive(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            match result {
                SendData::OpenConnection(connection_data) => {
                    match self.try_connect(&connection_data) {
                        Ok(()) => {
                            self.thread_is_running &= self.tx.send(SendData::Connected).is_ok();
                            self.is_connected = true;
                        }
                        Err(err) => {
                            self.thread_is_running &= self
                                .tx
                                .send(SendData::ConnectionError(err.to_string()))
                                .is_ok();
                            self.disconnect();
                        }
                    }
                }
                SendData::Data(buf) => {
                    if let Err(err) = self.com.send(&buf) {
                        log::error!("{err}");
                        let _ = self.tx.send(SendData::Disconnect);
                        self.disconnect();
                    }
                }
                SendData::StartTransfer(protocol_type, download, transfer_state, files_opt) => {
                    let mut copy_state = transfer_state.lock().unwrap().clone();
                    let mut protocol = protocol_type.create();
                    if let Err(err) = if download {
                        protocol.initiate_recv(&mut self.com, &mut copy_state)
                    } else {
                        protocol.initiate_send(&mut self.com, files_opt.unwrap(), &mut copy_state)
                    } {
                        log::error!("{err}");
                        break;
                    }
                    let mut storage_handler: TestStorageHandler = TestStorageHandler::new();

                    loop {
                        let v =
                            protocol.update(&mut self.com, &mut copy_state, &mut storage_handler);
                        match v {
                            Ok(running) => {
                                if !running {
                                    break;
                                }
                            }
                            Err(err) => {
                                log::error!("Error, aborting protocol: {err}");
                                copy_state.is_finished = true;
                                break;
                            }
                        }
                        if let Ok(SendData::CancelTransfer) = self.rx.try_recv() {
                            protocol.cancel(&mut self.com).unwrap_or_default();
                            break;
                        }
                        *transfer_state.lock().unwrap() = copy_state.clone();
                    }
                    *transfer_state.lock().unwrap() = copy_state.clone();

                    // TODO: Implement file storage handler, the test storage handler was ment to use in tests :)
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(user_dirs) = directories::UserDirs::new() {
                        let dir = user_dirs.download_dir().unwrap();

                        for file in &storage_handler.file {
                            let f = if file.0.is_empty() {
                                "new_file".to_string()
                            } else {
                                file.0.clone()
                            };

                            let mut file_name = dir.join(file.0);
                            let mut i = 1;
                            while file_name.exists() {
                                file_name = dir.join(&format!("{f}.{i}"));
                                i += 1;
                            }
                            std::fs::write(file_name, file.1.clone()).unwrap_or_default();
                        }
                    }
                    self.thread_is_running &= self.tx.send(SendData::EndTransfer).is_ok();
                }
                SendData::SetBaudRate(baud) => {
                    self.baud_rate = baud;
                }
                SendData::Disconnect => {
                    self.disconnect();
                }
                _ => {}
            }
        }
    }

    fn new(tx: mpsc::Sender<SendData>, rx: mpsc::Receiver<SendData>) -> Self {
        Self {
            tx,
            rx,
            baud_rate: 0,
            com: Box::new(crate::com::NullConnection {}),
            data_buffer: VecDeque::<u8>::new(),
            last_send_time: Instant::now(),
            thread_is_running: true,
            is_connected: false,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn poll(&mut self) {
        if self.is_connected {
            self.read_data();
        }
        self.handle_receive();
    }
}

impl MainWindow {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_com_thread() -> Connection {
        use web_time::Duration;

        let (tx, rx) = mpsc::channel::<SendData>();
        let (tx2, rx2) = mpsc::channel::<SendData>();
        std::thread::spawn(move || {
            let mut data: ConnectionThreadData = ConnectionThreadData::new(tx, rx2);
            while data.thread_is_running {
                if data.is_connected {
                    if !data.read_data() {
                        std::thread::sleep(Duration::from_millis(25));
                    }
                } else {
                    std::thread::sleep(Duration::from_millis(100));
                }
                data.handle_receive();
            }
            log::error!(
                "communication thread closed because it lost connection with the ui thread."
            );
        });
        Connection::new(rx, tx2)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn start_poll_thead() -> (Connection, ConnectionThreadData) {
        let (tx, rx) = mpsc::channel::<SendData>();
        let (tx2, rx2) = mpsc::channel::<SendData>();
        (Connection::new(rx, tx2), ConnectionThreadData::new(tx, rx2))
    }
}
