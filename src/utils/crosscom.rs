use crate::{
    ui::community::CommunityItem,
    utils::{
        compressionutils::CompressionUtils, extensions::OptionExt,
        network::network_listener::NetworkListener,
    },
};
use message_io::{
    network::{Endpoint, NetEvent, Transport},
    node::{self, NodeEvent, NodeHandler},
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    collections::HashMap,
    sync::{Arc, OnceLock},
};

/// CrossCom outgoing client data.
#[derive(rkyv::Archive, rkyv::Serialize, Default)]
pub struct CrossComClientData {
    username: String,
    data_type: DataType,
}

/// CrossCom incoming server data.
#[derive(rkyv::Archive, rkyv::Deserialize, Clone)]
pub struct CrossComServerData {
    username: Option<String>,
    pub data_type: DataType,
}

/// Cross Communication state.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum CrossComState {
    Connecting,
    Connected,
    Disconnected,
}

/// Different data types.
#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Default, Debug, PartialEq, Eq, Clone,
)]
pub enum DataType {
    #[default]
    None,

    /// Called when requesting authentication from the server, takes the startup channel.
    Auth(String),

    /// Received once authentication has completed.
    AuthSuccess,

    /// Sends Rune code from the client, to the server which then sends it to all party members.
    SendScripts(String),

    /// Gets the current version of dynamic. If `None`, it simply fetches the latest version.
    /// If `Some(String)`, the data is the latest version.
    GetVersion(Option<String>),

    /// Requests the server to update channel to the defined one.
    UpdateChannel(String),

    /// Received once `UpdateChannel` has been successfully processed.
    UpdateChannelSuccess,

    /// Requests the variables from the server.
    RequestVariables,

    /// Requests all of the server variables if specified as `None`.
    ReceiveVariables(HashMap<String, String>),

    UpdateVariables(HashMap<String, String>),

    /// Requests the community content from the server if specified as `None`.
    /// If `Some(Vec<CommunityItem>)`, the `Vec` contains all of the community content.
    BroadcastCommunityContent(Option<Vec<CommunityItem>>),

    /// Checks if a set of serial keys are valid for the given Sellix product.
    /// First parameter is the Product ID.
    /// Second parameter is the Bearer Token.
    CheckIsSerialOK(String, String, Vec<String>),

    /// Received once `CheckIsSerialOK` is been processed. The given response is a bool which
    /// indicates if its valid or not.
    CheckIsSerialOKResponse(bool),

    /// Sent by the server and displayed with a message box in dynamic.
    ServerError(String),

    /// Requests the font bytes from the server.
    RequestFonts,

    /// Sent by the server and contains the font bytes.
    SendFonts(Vec<u8>, Vec<u8>),
}

impl CrossComClientData {
    /// Converts the structure into a vector of bytes.
    #[allow(clippy::wrong_self_convention)]
    pub fn to_vec(self) -> Vec<u8> {
        CompressionUtils::write_compressed(
            rkyv::to_bytes::<_, 256>(&self)
                .unwrap_or_else(|error| {
                    crash!(
                        "[ERROR] Failed parsing structure into CrossComClientData, error: ",
                        error
                    )
                })
                .to_vec(),
        )
    }
}

/// Cross Communication between dynamic instances and a centralized server.
pub struct CrossCom {
    /// Client Username.
    username: &'static str,

    startup_channel: String,

    /// Use the local server?
    use_local_server: bool,

    /// Current CrossCom state.
    state: Cell<CrossComState>,

    /// Currently active channel.
    current_channel: RefCell<String>,

    /// Has the user requested to change channel?
    has_pending_channel_update: Cell<bool>,

    /// Server endpoint.
    server_endpoint: OnceLock<Endpoint>,

    /// Handler needed to send data to the server.
    handler: OnceLock<NodeHandler<Signal>>,

    /// Network Listener instance, needed for sending server messages across the rest of the
    /// client, and for receiving them on other ends.
    network_listener: NetworkListener,

    /// Decompressed data vector.
    decompressed_data: RefCell<Vec<u8>>,
}

thread_safe_structs!(CrossCom);

/// Network Signals.
enum Signal {
    /// Signal used when connecting.
    ConnectSignal,
}

impl CrossCom {
    /// Initializes `CrossCom`.
    pub fn init(username: &'static str, mut channel: String, use_local_server: bool) -> Self {
        channel.truncate(64);

        Self {
            username,
            startup_channel: channel.to_owned(),
            use_local_server,
            state: Cell::new(CrossComState::Disconnected),
            current_channel: RefCell::new(channel.to_owned()),
            has_pending_channel_update: Cell::default(),
            server_endpoint: OnceLock::new(),
            handler: OnceLock::new(),
            network_listener: NetworkListener::new(),
            decompressed_data: RefCell::new(Vec::with_capacity(512)),
        }
    }

    /// Attempts to connect to the server.
    pub fn connect(&self) {
        // Setup server.
        let (handler, listener) = node::split();

        let server_address = if self.use_local_server {
            log!("## Development: Using local server at port 8391!");
            ozencstr!("0.0.0.0:8391")
        } else {
            ozencstr!(include_str!("../../crosscom_ip").replace(['\n', '\r'], ""))
        };

        // Connect via FramedTcp.
        if let Err(error) = handler
            .network()
            .connect(Transport::FramedTcp, server_address)
        {
            crash!(
                "[ERROR] Couldn't connect to server, report the following message: ",
                error
            );
        }

        self.set_state(CrossComState::Connecting);
        listener.for_each(move |event| match event {
            NodeEvent::Network(net_event) => match net_event {
                NetEvent::Connected(endpoint, _) => {
                    self.server_endpoint.get_or_init(|| endpoint);
                    self.handler.get_or_init(|| handler.to_owned());

                    handler.signals().send(Signal::ConnectSignal);
                }
                NetEvent::Accepted(..) => unreachable!(),
                NetEvent::Message(_, data) => {
                    if let Ok(server_data) = unsafe {
                        CompressionUtils::decompress(
                            data,
                            &mut self.decompressed_data.borrow_mut(),
                        );
                        rkyv::from_bytes_unchecked(&self.decompressed_data.borrow())
                    } {
                        self.handle_server_data(server_data);
                    }
                }
                NetEvent::Disconnected(_) => {
                    self.set_state(CrossComState::Disconnected);
                }
            },
            NodeEvent::Signal(signal) => match signal {
                Signal::ConnectSignal => {
                    if self.get_state() == CrossComState::Connected {
                        crash!("[ERROR] CrossCom is already connected!");
                    }

                    self.send_data_type(DataType::Auth(self.get_startup_channel().to_owned()));
                }
            },
        });
    }

    /// Handles all server data types.
    fn handle_server_data(&self, server_data: CrossComServerData) {
        match server_data.data_type {
            DataType::AuthSuccess => self.send_data_type(DataType::GetVersion(None)),
            DataType::GetVersion(ref version) => {
                if version.is_none() {
                    return;
                }

                self.set_state(CrossComState::Connected);
                self.send_to_channel(server_data);
            }
            DataType::SendScripts(ref script) => {
                if server_data.username.is_some() && !script.is_empty() {
                    self.send_to_channel(server_data);
                }
            }
            DataType::UpdateChannelSuccess | DataType::SendFonts(..) => {
                self.send_to_channel(server_data);
            }
            DataType::ReceiveVariables(_) => self.send_to_channel(server_data),
            DataType::BroadcastCommunityContent(ref content) => {
                if content.is_some() {
                    self.send_to_channel(server_data)
                }
            }
            DataType::CheckIsSerialOKResponse(_success) => self.send_to_channel(server_data),
            DataType::ServerError(ref error) => crash!(error),
            _ => {
                crash!("[SECURITY] Received an unknown data type, closing dynamic for your own safety.");
            }
        }
    }

    /// Send a basic data type request.
    pub fn send_data_type(&self, data_type: DataType) {
        match data_type {
            DataType::UpdateChannelSuccess
            | DataType::AuthSuccess
            | DataType::CheckIsSerialOKResponse(..)
            | DataType::ServerError(..)
            | DataType::SendFonts(..)
            | DataType::ReceiveVariables(..) => {
                crash!("[ERROR] Unsupported Data Type!")
            }
            _ => {
                let server_endpoint = self
                    .server_endpoint
                    .get()
                    .unwrap_or_crash(zencstr!("[ERROR] Server Endpoint hasn't been assigned!"));

                let handler = self
                    .handler
                    .get()
                    .unwrap_or_crash(zencstr!("[ERROR] Handler hasn't been assigned!"));

                let data = CrossComClientData {
                    username: self.username.to_owned(),
                    data_type,
                }
                .to_vec();
                handler.network().send(*server_endpoint, &data);
            }
        }
    }

    /// Sends the specified Rune script.
    pub fn send_script(&self, source: &str) {
        self.send_data_type(DataType::SendScripts(source.to_owned()));
        log!("[PARTY] Sent script to channel members!");
    }

    /// Sends the variables, if required to.
    pub fn send_variables(&self, variables: HashMap<String, String>) {
        self.send_data_type(DataType::UpdateVariables(variables));
    }

    /// Sends the specified data type and waits for a server message to be received, then
    /// passes it into `callback`.
    /// `callback` should return true/false for whether or not the message was the correct one or
    /// not.
    fn send_and_wait<F: Fn(DataType) -> bool>(&self, send_data_type: DataType, callback: F) {
        self.send_data_type(send_data_type);

        loop {
            let Some(server_message) = self.get_network_listener().wait_for_message_raw() else {
                continue;
            };

            if callback(server_message.data_type) {
                break;
            }
        }
    }

    /// Gets the server variables.
    #[optimize(size)]
    pub fn get_variables(&self) -> HashMap<String, String> {
        let mut result = OnceCell::new();
        self.send_and_wait(DataType::RequestVariables, |data_type| match data_type {
            DataType::ReceiveVariables(variables) => {
                result.get_or_init(|| variables);
                true
            }
            _ => false,
        });

        std::mem::take(
            result
                .get_mut()
                .unwrap_or_crash(zencstr!("[ERROR] No variables received!")),
        )
    }

    /// Gets the server community content.
    #[optimize(size)]
    pub fn get_community_content(&self) -> Vec<CommunityItem> {
        let mut result = OnceCell::new();
        self.send_and_wait(
            DataType::BroadcastCommunityContent(None),
            |data_type| match data_type {
                DataType::BroadcastCommunityContent(content) => {
                    result.get_or_init(|| {
                        content.unwrap_or_crash(zencstr!(
                            "[ERROR] Server sent content as `None`, this should never happen!"
                        ))
                    });
                    true
                }
                _ => false,
            },
        );

        std::mem::take(
            result
                .get_mut()
                .unwrap_or_crash(zencstr!("[ERROR] No community content received!")),
        )
    }

    /// Requests to get the fonts used.
    #[optimize(size)]
    pub fn get_fonts(&self) -> (Vec<u8>, Vec<u8>) {
        let mut result = OnceCell::new();
        self.send_and_wait(DataType::RequestFonts, |data_type| match data_type {
            DataType::SendFonts(normal, bold) => {
                result.get_or_init(|| (normal, bold));
                true
            }
            _ => false,
        });

        std::mem::take(
            result
                .get_mut()
                .unwrap_or_crash(zencstr!("[ERROR] No fonts received!")),
        )
    }

    /// Sets the current state.
    fn set_state(&self, state: CrossComState) {
        match state {
            CrossComState::Disconnected => {
                if let Some(handler) = self.handler.get() {
                    handler.stop();
                }

                crash!("[SERVER] Disconnected from server, closing dynamic!");
            }
            CrossComState::Connecting => {
                log!("[SERVER] Connecting...");
            }
            CrossComState::Connected => {
                log!("[SERVER] Connected!");
            }
        }

        self.state.set(state);
    }

    /// Gets the current state.
    pub fn get_state(&self) -> CrossComState {
        self.state.get()
    }

    /// Sends a message to the CrossCom channel.
    pub fn send_to_channel(&self, data: CrossComServerData) {
        let Err(error) = self
            .get_network_listener()
            .get_crossbeam_channel()
            .0
            .send(data)
        else {
            return;
        };

        log!("[ERROR] Failed sending channel message, error: ", error);
    }

    /// Returns the startup channel.
    fn get_startup_channel(&self) -> &str {
        &self.startup_channel
    }

    /// Joins a new channel, leaving `channel` as-is since it's the startup value.
    /// No `&self` as it causes deadlocks.
    pub fn join_channel(&self, channel: &str) {
        if self.has_pending_channel_update.get() {
            log!("[ERROR] You are already in the process of joining a channel, be patient!");
            return;
        }

        let current_state = self.get_state();
        if current_state != CrossComState::Connected {
            crash!(
                "[ERROR] Invalid CrossCom state: ",
                format!("{:?}", current_state),
                ", expected CrossComState::Connected!"
            );
        }

        if self.current_channel.borrow().eq_ignore_ascii_case(channel)
            || !channel.starts_with('#')
            || channel.contains(' ')
            || channel.len() < 4
        {
            log!("[ERROR] You are either already in the specified channel, or its invalid!");
            return;
        }

        self.has_pending_channel_update.set(true);

        // Keep the channel string within a certain range of characters before applying it as the
        // new channel.
        let mut channel = channel.to_owned();
        channel.truncate(64);

        *self.current_channel.borrow_mut() = channel;
        self.send_data_type(DataType::UpdateChannel(
            self.current_channel.borrow().to_owned(),
        ));

        if self
            .get_network_listener()
            .wait_for_message(DataType::UpdateChannelSuccess)
            .is_some()
        {
            log!("[PARTY] Joined channel!");
            self.has_pending_channel_update.set(false);
            return;
        }

        log!("[ERROR] Failed joining channel, no server approval received!");
        self.has_pending_channel_update.set(false);
    }

    /// Checks if one of the serials for the given Sellix product, is valid.
    /// This has to be done through CrossCom for security and compatibility reasons.
    /// On Wine and/or Proton, using reqwest may fail.
    pub fn check_is_ex_serial_ok(
        &self,
        product_id: String,
        bearer_token: String,
        serials: Arc<Vec<String>>,
    ) -> bool {
        self.send_data_type(DataType::CheckIsSerialOK(
            product_id,
            bearer_token,
            (*serials).to_owned(),
        ));

        loop {
            let Some(server_message) = self.get_network_listener().wait_for_message_raw() else {
                continue;
            };

            match server_message.data_type {
                DataType::CheckIsSerialOKResponse(success) => {
                    return success;
                }
                _ => {
                    drop(server_message);
                    continue;
                }
            }
        }
    }

    /// Gets the current channel. Do **not** modify it as-is, use `join_channel`!
    pub const fn get_current_channel(&self) -> &RefCell<String> {
        &self.current_channel
    }

    /// Gets the Network Listener, which is useful for getting network data and/or hooking into a
    /// special set of network events.
    pub const fn get_network_listener(&self) -> &NetworkListener {
        &self.network_listener
    }
}
