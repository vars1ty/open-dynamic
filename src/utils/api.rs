use crate::utils::{
    crosscom::{CrossCom, DataType},
    extensions::OptionExt,
};
use parking_lot::RwLock;
use std::sync::Arc;
use zstring::ZString;

/// Minimal API implementation.
#[allow(clippy::upper_case_acronyms)]
pub struct API;

impl API {
    /// Validates the version of the client.
    /// If a check fails, the program crashes.
    pub fn validate_version(crosscom: Arc<RwLock<CrossCom>>) -> Arc<RwLock<CrossCom>> {
        log!("Validating version...");

        // Lock thread until a valid response has been received.
        let version = loop {
            if let Some(message) = crosscom
                .read()
                .get_network_listener()
                .wait_for_message_raw()
            {
                match message.data_type {
                    DataType::GetVersion(version) => {
                        let version = ZString::new(
                            version
                                .unwrap_or_crash(zencstr!("[ERROR] Server sent missing version!")),
                        );
                        break version;
                    }
                    _ => continue,
                }
            }
        };

        // Update this once the version changes, since using env!() doesn't inline the version for
        // it to be encrypted.
        if zencstr!("6.9.0-release") != zencstr!(version.data) {
            crash!("[ERROR] Invalid version, switch to ", version)
        }

        drop(version);
        log!("Version validated!");
        crosscom
    }
}
