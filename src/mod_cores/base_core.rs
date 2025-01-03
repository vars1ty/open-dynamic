use crate::utils::{
    api::API,
    config::Config,
    crosscom::{CrossCom, CrossComState},
    eguiutils::ImGuiUtils,
    extensions::OptionExt,
    prompter::Prompter,
    runedetour::RDetour,
    scripting::{arctic::Arctic, script_core::ScriptCore},
    stringutils::StringUtils,
    ui::customwindows::CustomWindowsUtils,
};
use parking_lot::RwLock;
use std::sync::{Arc, LazyLock, OnceLock};

/// A base core structure which holds a handle to the current process, and an instance to `Config`.
pub struct BaseCore {
    /// Cached config instance.
    config: &'static Config,

    /// CrossCom instance.
    crosscom: Arc<RwLock<CrossCom>>,

    /// ScriptCore instance.
    script_core: LazyLock<&'static ScriptCore>,

    /// Custom Window utilities instance.
    custom_window_utils: LazyLock<&'static CustomWindowsUtils>,

    /// Arctic Gateway core.
    arctic_core: OnceLock<Arctic>,

    /// ImGuiUtils instance.
    imgui_utils: Arc<RwLock<ImGuiUtils>>,
}

thread_safe_structs!(BaseCore);

impl BaseCore {
    /// Initializes everything needed.
    pub fn init() -> Self {
        let config: &'static Config = Box::leak(Box::default());
        let use_local_server = config.get_use_local_server();
        let startup_channel = config.get_startup_channel();
        RDetour::register_all_detours();

        Self {
            config,
            crosscom: {
                // Create username and channel as static strings.
                let username: &'static str = StringUtils::get_random().leak();
                let channel = startup_channel.unwrap_or_else(|| {
                    format!(
                        "#{}0{}",
                        StringUtils::get_random(),
                        std::ptr::addr_of!(username) as i32
                    )
                });

                // Validate version as soon as we are connected.
                API::validate_version(Self::connect_crosscom(username, channel, use_local_server))
            },
            script_core: LazyLock::new(|| Box::leak(Box::new(ScriptCore::init()))),
            custom_window_utils: LazyLock::new(|| Box::leak(Box::default())),
            arctic_core: OnceLock::new(),
            imgui_utils: Arc::new(RwLock::new(ImGuiUtils::new())),
        }
    }

    /// Attempts to connect to CrossCom's server.
    fn connect_crosscom(
        username: &'static str,
        channel: String,
        use_local_server: bool,
    ) -> Arc<RwLock<CrossCom>> {
        let instance = Arc::new(RwLock::new(CrossCom::init(
            username,
            channel.to_owned(),
            use_local_server,
        )));
        let instance_clone = Arc::clone(&instance);

        std::thread::spawn(move || {
            let reader = instance_clone.try_read().unwrap_or_crash(zencstr!(
                "[ERROR] Failed reading CrossCom, cannot start connecting!"
            ));

            reader.connect();
        });

        let reader = instance.try_read().unwrap_or_crash(zencstr!(
            "[ERROR] Failed reading CrossCom, cannot check state!"
        ));

        let mut elapsed = 0.0;
        loop {
            let is_connected = reader.get_state() == CrossComState::Connected;
            if is_connected {
                // Exit block, we are connected.
                break Arc::clone(&instance);
            }

            std::thread::sleep(std::time::Duration::from_millis(500));
            elapsed += 0.5;

            if elapsed == 5.0 && !is_connected {
                log!("[NOTICE] This is taking longer than expected, ensure you have a stable connection!");
                log!("[NOTICE] Also ensure that Windows Defender (or any other anti-virus) is not interfering.");
            }

            // If it has been 10 seconds and we aren't connected, ask the user if they want to try
            // again or give up.
            if elapsed == 10.0 && !is_connected {
                drop(reader);
                drop(instance);

                let mut prompt = Prompter::new("[PROMPT] Write 'r' to try and re-connect. Write any other response to close dynamic.", vec!["R", "r"]);
                if prompt.prompt().is_some() {
                    log!("[PROMPT] Reconnecting...");
                    drop(prompt);

                    break Self::connect_crosscom(username, channel, use_local_server);
                }

                crash!("[ERROR] Failed connecting to the server, perhaps your serial is incorrect, or the server is down?");
            }
        }
    }

    /// Hooks the `SendScripts` event and executes the source once received.
    pub fn link_script_received(&self, self_arc: Arc<RwLock<Self>>) {
        let crosscom = self.get_crosscom();
        let Some(crosscom) = crosscom.try_read() else {
            log!("[ERROR] Can't link script events to a callback, CrossCom is locked!");
            return;
        };

        let script_core = self.get_script_core();
        crosscom.get_network_listener().hook_on_script_received(
            self.get_crosscom(),
            move |script| {
                let self_arc = Arc::clone(&self_arc);
                script_core.execute(script, self_arc, false, false);
            },
        );
    }

    /// Returns the cached config instance.
    pub const fn get_config(&self) -> &'static Config {
        self.config
    }

    /// Returns the CrossCom instance.
    pub fn get_crosscom(&self) -> Arc<RwLock<CrossCom>> {
        Arc::clone(&self.crosscom)
    }

    /// Gets the `ScriptCore` instance.
    pub fn get_script_core(&self) -> &'static ScriptCore {
        &self.script_core
    }

    /// Returns an instance of `CustomWindowsUtils`.
    pub fn get_custom_window_utils(&self) -> &'static CustomWindowsUtils {
        &self.custom_window_utils
    }

    /// Gets the Arctic Gateway core.
    pub const fn get_arctic_core(&self) -> &OnceLock<Arctic> {
        &self.arctic_core
    }

    /// Gets the `ImGuiUtils` instance.
    pub fn get_imgui_utils(&self) -> Arc<RwLock<ImGuiUtils>> {
        Arc::clone(&self.imgui_utils)
    }
}
