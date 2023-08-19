#![allow(unsafe_code, clippy::wildcard_imports)]

use std::collections::VecDeque;
use std::sync::mpsc;

#[cfg(not(target_arch = "wasm32"))]
use std::thread;

#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;

use std::time::{Duration, Instant};

use eframe::egui::{self};

use crate::com::Com;
use crate::com::SendData;
use crate::protocol::TestStorageHandler;

use crate::com::Connection;

use super::MainWindow;

const BITS_PER_BYTE: u32 = 8;

impl MainWindow {
    pub fn open_connection() -> Connection {
        //let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel::<SendData>();
        let (tx2, rx2) = mpsc::channel::<SendData>();
        thread::spawn(move || {
            let mut baud_rate = 0;
            let mut handle: Box<dyn Com> = Box::new(crate::com::NullConnection {});

            let mut done = false;

            let mut data_buffer = VecDeque::<u8>::new();
            let mut time = Instant::now();

            while !done {
                if data_buffer.is_empty() {
                    if let Ok(Some(data)) = handle.read_data() {
                        if baud_rate == 0 {
                            if let Err(err) = tx.send(SendData::Data(data)) {
                                log::error!("{err}");
                                done = true;
                            }
                            // ctx.request_repaint();
                        } else {
                            data_buffer.extend(data);
                        }
                    } else {
                        wasm_thread::sleep(Duration::from_millis(25));
                    }
                } else if baud_rate == 0 {
                    if let Err(err) = tx.send(SendData::Data(data_buffer.drain(..).collect())) {
                        log::error!("{err}");
                        done = true;
                    }
                } else {
                    let cur_time = Instant::now();
                    let bytes_per_sec = baud_rate / BITS_PER_BYTE;
                    let elapsed_ms = cur_time.duration_since(time).as_millis() as u32;
                    let bytes_to_send: usize = ((bytes_per_sec * elapsed_ms) / 1000)
                        .min(data_buffer.len() as u32)
                        as usize;

                    if bytes_to_send > 0 {
                        if let Err(err) =
                            tx.send(SendData::Data(data_buffer.drain(..bytes_to_send).collect()))
                        {
                            log::error!("{err}");
                            done = true;
                        }
                        time = cur_time;
                    }
                }

                while let Ok(result) = rx2.try_recv() {
                    match result {
                        SendData::OpenConnection(call_adr, timeout, window_size) => {
                            let mut com: Box<dyn Com> = match call_adr.protocol {
                                crate::addresses::Protocol::Telnet => {
                                    Box::new(crate::com::ComTelnetImpl::new(window_size))
                                }
                                crate::addresses::Protocol::Raw => {
                                    Box::new(crate::com::ComRawImpl::new())
                                }
                                crate::addresses::Protocol::Ssh => panic!(), //Box::new(crate::com::SSHCom::new()),
                            };
                            if let Err(err) = com.connect(&call_adr, timeout) {
                                tx.send(SendData::ConnectionError(err.to_string()));
                            } else {
                                tx.send(SendData::Connected);
                            }
                            handle = com;
                        }
                        SendData::Data(buf) => {
                            if let Err(err) = handle.send(&buf) {
                                log::error!("{err}");
                                done = true;
                            }
                        }
                        SendData::StartTransfer(
                            protocol_type,
                            download,
                            transfer_state,
                            files_opt,
                        ) => {
                            let mut copy_state = transfer_state.lock().unwrap().clone();

                            let mut protocol = protocol_type.create();

                            if let Err(err) = if download {
                                protocol.initiate_recv(&mut handle, &mut copy_state)
                            } else {
                                protocol.initiate_send(
                                    &mut handle,
                                    files_opt.unwrap(),
                                    &mut copy_state,
                                )
                            } {
                                log::error!("{err}");
                                break;
                            }
                            let mut storage_handler: TestStorageHandler = TestStorageHandler::new();

                            loop {
                                let v = protocol.update(
                                    &mut handle,
                                    &mut copy_state,
                                    &mut storage_handler,
                                );
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
                                if let Ok(SendData::CancelTransfer) = rx2.try_recv() {
                                    protocol.cancel(&mut handle).unwrap_or_default();
                                    break;
                                }
                                *transfer_state.lock().unwrap() = copy_state.clone();
                            }
                            *transfer_state.lock().unwrap() = copy_state.clone();
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
                                        file_name = dir.join(&format!("{}.{}", f, i));
                                        i += 1;
                                    }
                                    std::fs::write(file_name, file.1.clone()).unwrap_or_default();
                                }
                            }
                            tx.send(SendData::EndTransfer).unwrap_or_default();
                        }
                        SendData::SetBaudRate(baud) => {
                            baud_rate = baud;
                        }
                        SendData::Disconnect => {
                            done = true;
                        }
                        _ => {}
                    }
                }
            }

            tx.send(SendData::Disconnect);
        });
        Connection::new(rx, tx2)
    }
}

/*
        /*
        if self.open_connection_promise.is_some()
         && self.open_connection_promise.as_ref().unwrap().()
         */
        {
            if let Some(join_handle) = &self.open_connection_promise {
                let handle = &join_handle.join();
                if let Ok(handle) = &handle {
                    match handle {
                        Ok(handle) => {
                            self.open_connection(ctx, handle);
                        }
                        Err(err) => {
                            self.println(&format!("\n\r{err}")).unwrap();
                        }
                    }
                }
            }
        }


*/
