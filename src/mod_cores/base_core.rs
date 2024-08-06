use crate::utils::{
    api::API,
    config::Config,
    crosscom::{CrossCom, CrossComState},
    extensions::OptionExt,
    prompter::Prompter,
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
    script_core: ScriptCore,

    /// Custom Window utilities instance.
    custom_window_utils: LazyLock<&'static CustomWindowsUtils>,

    /// Arctic Gateway core.
    arctic_core: OnceLock<Arctic>,
}

thread_safe_structs!(BaseCore);

impl BaseCore {
    /// Initializes everything needed.
    pub fn init() -> Self {
        let config: &'static Config = Box::leak(Box::default());
        let use_local_server = config.get_use_local_server();
        let main_serial = config
            .get_product_serials()
            .first()
            .unwrap_or_crash(zencstr!("[ERROR] Missing primary NDNX/INTERNAL serial!"));
        Self {
            config,
            crosscom: {
                // Create username and channel as static strings.
                let username: &'static str = StringUtils::get_random().leak();
                let channel: &'static str = format!(
                    "#{}0{}",
                    StringUtils::get_random(),
                    std::ptr::addr_of!(username) as i32
                )
                .leak();

                // Validate version as soon as we are connected.
                API::validate_version(Self::connect_crosscom(
                    username,
                    channel,
                    use_local_server,
                    main_serial,
                ))
            },
            script_core: ScriptCore::init(),
            custom_window_utils: LazyLock::new(|| Box::leak(Box::default())),
            arctic_core: OnceLock::new(),
        }
    }

    /// Attempts to connect to CrossCom's server.
    fn connect_crosscom(
        username: &'static str,
        channel: &'static str,
        use_local_server: bool,
        main_serial: &'static String,
    ) -> Arc<RwLock<CrossCom>> {
        // Initialize instance.
        let instance = Arc::new(RwLock::new(CrossCom::init(
            username,
            channel,
            use_local_server,
        )));
        let instance_clone = Arc::clone(&instance);

        std::thread::spawn(move || {
            let reader = instance_clone.try_read().unwrap_or_crash(zencstr!(
                "[ERROR] Failed reading CrossCom, cannot start connecting!"
            ));

            reader.connect(main_serial);
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
                log!("[NOTICE] If you are on a VPN, only use Mullvad, PerfectPrivacy or IVPN. All others should remain off.");
            }

            // If it has been 10 seconds and we aren't connected, ask the user if they want to try
            // again or give up.
            if elapsed == 10.0 && !is_connected {
                let mut prompt = Prompter::new("[PROMPT] Write 'r' to try and re-connect. Write any other response to close dynamic.", smallvec!["R", "r"]);
                if prompt.prompt().is_some() {
                    log!("Freeing old resources...");
                    drop(reader);
                    drop(instance);

                    log!("Trying to connect again...");
                    break Self::connect_crosscom(username, channel, use_local_server, main_serial);
                }

                crash!("[ERROR] Failed connecting to the server, perhaps your serial is incorrect, or the server is down?");
            }
        }
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
    pub const fn get_script_core(&self) -> &ScriptCore {
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
}
