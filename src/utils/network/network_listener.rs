use crate::utils::crosscom::{CrossCom, CrossComServerData, DataType};
use crossbeam_channel::{Receiver, Sender};
use parking_lot::RwLock;
use std::sync::{Arc, LazyLock};

/// Network Listener utility.
pub struct NetworkListener {
    /// Channel that receives server messages from across `crosscom.rs`.
    crossbeam_channel: Arc<LazyLock<(Sender<CrossComServerData>, Receiver<CrossComServerData>)>>,
}

impl NetworkListener {
    /// Initializes an instance of `Self`.
    pub fn new() -> Self {
        Self {
            crossbeam_channel: Arc::new(LazyLock::new(
                crossbeam_channel::unbounded::<CrossComServerData>,
            )),
        }
    }

    /// Waits for the Crossbeam channel to receive an instance of `CrossComServerData`.
    fn internal_wait_for_message_raw(
        crossbeam_channel: Arc<
            LazyLock<(Sender<CrossComServerData>, Receiver<CrossComServerData>)>,
        >,
    ) -> Option<CrossComServerData> {
        crossbeam_channel.1.recv().ok()
    }

    /// Waits for the Crossbeam channel to receive an instance of `CrossComServerData`.
    pub fn wait_for_message_raw(&self) -> Option<CrossComServerData> {
        Self::internal_wait_for_message_raw(Arc::clone(&self.crossbeam_channel))
    }

    /// Waits for the Crossbeam channel to receive an instance of `CrossComServerData`, with a
    /// specific simple data type.
    pub fn wait_for_message(&self, data_type: DataType) -> Option<CrossComServerData> {
        if let Some(message) = self
            .wait_for_message_raw()
            .take_if(|message| message.data_type == data_type)
        {
            return Some(message);
        }

        None
    }

    /// Hooks the `SendScripts` Data Type.
    pub fn hook_on_script_received<F: Fn(String) + Send + 'static>(
        &self,
        crosscom: Arc<RwLock<CrossCom>>,
        callback: F,
    ) {
        // Using AsyncUtils here causes it to crash randomly after the manual mapping
        // implementation.
        let crossbeam_chanel = Arc::clone(&self.crossbeam_channel);
        std::thread::spawn(move || {
            loop {
                let Some(message) =
                    Self::internal_wait_for_message_raw(Arc::clone(&crossbeam_chanel))
                else {
                    continue;
                };

                if let Some(crosscom) = crosscom.try_read() {
                    match message.data_type {
                        DataType::SendScripts(script) => {
                            if script.is_empty() {
                                continue;
                            }

                            callback(script)
                        }
                        _ => {
                            // Not SendScripts, send the message back to Crossbeam.
                            crosscom.send_to_channel(message);
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });
    }

    /// Gets the crossbeam channel.
    pub fn get_crossbeam_channel(
        &self,
    ) -> &LazyLock<(Sender<CrossComServerData>, Receiver<CrossComServerData>)> {
        &self.crossbeam_channel
    }
}
