use crate::protocol::{FileDescriptor, TransferState};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use super::connection::Connection;
use super::dialogs;
pub struct FileTransferThread {
    pub current_transfer: Arc<Mutex<TransferState>>,
    pub file_transfer_dialog: dialogs::up_download_dialog::FileTransferDialog,

    pub join_handle: Option<JoinHandle<Box<Connection>>>,
}

impl FileTransferThread {
    pub fn new(
        mut connection: Box<Connection>,
        protocol_type: crate::protocol::TransferType,
        download: bool,
        files_opt: Option<Vec<FileDescriptor>>,
    ) -> Self {
        let current_transfer = Arc::new(Mutex::new(TransferState::default()));

        let current_transfer2 = current_transfer.clone();

        let join_handle = thread::spawn(move || {
            let mut protocol = protocol_type.create();

            if let Err(err) = if download {
                protocol.initiate_recv(&mut connection, &mut current_transfer2.lock().unwrap())
            } else {
                protocol.initiate_send(
                    &mut connection,
                    files_opt.unwrap(),
                    &mut current_transfer2.lock().unwrap(),
                )
            } {
                log::error!("{err}");
                return connection;
            }

            if let Ok(mut storage_handler) = crate::protocol::DiskStorageHandler::new() {
                let mut is_running = true;
                while is_running {
                    if let Err(err) = connection.update_state() {
                        log::error!("Error updating state on file transfer thread: {err}");
                        break;
                    }
                    match protocol.update(&mut connection, &current_transfer2, &mut storage_handler)
                    {
                        Ok(b) => is_running &= b,
                        Err(err) => {
                            log::error!("Error updating protocol on file transfer thread: {err}");
                            break;
                        }
                    }
                    match current_transfer2.lock() {
                        Ok(ct) => {
                            if ct.request_cancel {
                                if let Err(err) = protocol.cancel(&mut connection) {
                                    log::error!(
                                        "Error sending cancel request on file transfer thread: {err}"
                                    );
                                }
                                break;
                            }
                        }
                        Err(err) => {
                            log::error!(
                                "Error locking current_transfer on file transfer thread: {err}"
                            );
                            break;
                        }
                    }
                }
            }
            connection
        });

        Self {
            current_transfer,
            file_transfer_dialog: dialogs::up_download_dialog::FileTransferDialog::new(),
            join_handle: Some(join_handle),
        }
    }
}
