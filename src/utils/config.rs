use crate::{
    utils::extensions::OptionExt,
    winutils::{Renderer, WinUtils},
};
use serde_jsonc::Value;
use std::{
    cell::OnceCell,
    fmt::Display,
    fs::read_to_string,
    io::{Result, Write},
    sync::Arc,
};

/// Simple JSON config.
pub struct Config {
    /// Cached config.
    cached_config: OnceCell<Value>,

    /// Directory path.
    path: &'static str,

    /// Custom product serials.
    serials: Arc<Vec<String>>,
}

thread_safe_structs!(Config);

impl Default for Config {
    /// Returns a default pre-configured instance of `Config`, should only be used once and be
    /// cached!
    fn default() -> Self {
        let dir_path = WinUtils::get_module_path(zencstr!("dynamic.dll\0"))
            .data
            .replace(&zencstr!("dynamic.dll").data, "");
        let path = zencstr!(&dir_path, "config.jsonc");
        let config_content = read_to_string(&path.data).unwrap_or_else(|error| {
            log!(
                "[ERROR] Couldn't read config.jsonc, error: ",
                error,
                "\n[INFO] This is entirely your own fault, and not dynamics. Learn JSON!",
                "\n[INFO] Using default OpenGL config."
            );
            include_str!("../../resources/config.jsonc").to_owned()
        });
        drop(path);

        let cached_config: OnceCell<Value> = OnceCell::new();
        let cached_config_ref = cached_config.get_or_init(|| {
            serde_jsonc::from_str(&config_content).unwrap_or_else(|error| {
                crash!(
                    "[ERROR] Failed parsing config.jsonc, error: ",
                    error,
                    "\n[INFO] This is entirely your own fault, and not dynamics. Learn JSON!"
                )
            })
        });

        let default_cfg_serials = vec![serde_jsonc::to_value(ozencstr!("FREE-ACCESS"))
            .unwrap_or_else(|error| {
                crash!(
                    "[ERROR] Failed creating default_cfg_serials, error: ",
                    error
                )
            })];

        let mut cfg_serials = cached_config_ref[&zencstr!("serials").data]
            .as_array()
            .unwrap_or_else(|| {
                log!("[WARN] Missing serials string-array, using an empty array!");
                &default_cfg_serials
            })
            .to_vec()
            .iter()
            .map(|serial| {
                serial
                    .as_str()
                    .unwrap_or_crash(zencstr!("[ERROR] ", serial, " is not a valid string!"))
                    .to_owned()
            })
            .collect::<Vec<String>>();

        if cfg_serials.is_empty() {
            log!("[WARN] No serials present, using free version!");
            cfg_serials.push(ozencstr!("FREE-ACCESS"));
        }

        Self {
            cached_config,
            path: dir_path.leak(),
            serials: Arc::new(cfg_serials),
        }
    }
}

impl Config {
    /// Returns a reference to the config.
    fn get(&self) -> &Value {
        self.cached_config.get().unwrap_or_crash(zencstr!(
            "[ERROR] Config hasn't been parsed, instance created improperly!"
        ))
    }

    /// Should the console be freed?
    pub fn get_free_console(&self) -> bool {
        self.get()[&zencstr!("free_console").data]
            .as_bool()
            .unwrap_or_default()
    }

    /// Should 0.0.0.0 be used over the public server ip?
    pub fn get_use_local_server(&self) -> bool {
        self.get()[&zencstr!("use_local_server").data]
            .as_bool()
            .unwrap_or_default()
    }

    /// Takes `name` and appends it to the back of `self.path`, returning the full path.
    fn get_full_path_for(&self, name: &str) -> Option<String> {
        if name.is_empty() {
            log!("[ERROR] File name cannot be empty!");
            return None;
        }

        // Pre-allocate a string with the length of path and name, plus 2 in case there's any
        // forward-slashes that need to be replaced with backward-slashes.
        // Why? Because some Windows setups are fucking braindead and can't cope with
        // forward-slashes.
        let mut path = String::with_capacity(self.path.len() + name.len() + 2);
        path.push_str(self.path);

        if path.contains('/') {
            path.push_str(&name.replace('/', "\\"));
        } else {
            path.push_str(name);
        }

        Some(path)
    }

    /// Saves content to a file, overriding any old file(s) with the same name.
    pub fn save_to_file(&self, name: &str, content: &str) -> bool {
        let Some(path) = self.get_full_path_for(name) else {
            return false;
        };

        let file = std::fs::File::create(path);
        if let Ok(mut file) = file {
            let write = file.write_all(content.as_bytes());
            if let Err(error) = write {
                log!("[ERROR] Failed writing to file, error: ", error);
                return false;
            }

            return true;
        }

        let error = file.unwrap_err();
        log!("[ERROR] Failed creating file, error: ", error);
        false
    }

    /// Gets the content of the given file.
    pub fn get_file_content(&self, name: &str, output_string: &mut String) -> bool {
        let Some(path) = self.get_full_path_for(name) else {
            return false;
        };

        let read = std::fs::read_to_string(path);
        if let Ok(content) = read {
            *output_string = content;
            return true;
        }

        let error = read.unwrap_err();
        log!("[ERROR] Failed reading file, error: ", error);
        false
    }

    /// Attempts to read the relative file and return the content as bytes.
    pub fn get_file_content_bytes<S: AsRef<str> + Display>(&self, name: S) -> Result<Vec<u8>> {
        std::fs::read(
            self.get_full_path_for(name.as_ref())
                .unwrap_or_crash(zencstr!("[ERROR] File name cannot be empty!")),
        )
    }

    /// Gets the path to the DLL directory.
    pub const fn get_path(&self) -> &'static str {
        self.path
    }

    /// Gets the user-defined product serials, if any.
    pub fn get_product_serials(&self) -> Arc<Vec<String>> {
        Arc::clone(&self.serials)
    }

    /// Gets the renderer target to be used for unsupported games.
    pub fn get_renderer_target(&self) -> Renderer {
        let defined = self.get()[&zencstr!("renderer_target").data]
            .as_str()
            .unwrap_or_else(|| {
                log!("[WARN] No renderer_target defined, using None.");
                "None"
            });

        match defined {
            "DirectX9" => Renderer::DirectX9,
            "DirectX11" => Renderer::DirectX11,
            "DirectX12" => Renderer::DirectX12,
            "OpenGL" => Renderer::OpenGL,
            "None" => Renderer::None,
            _=> crash!("[ERROR] Unknown renderer target. Available options are: DirectX9, DirectX11, DirectX12, OpenGL and None.")
        }
    }

    /// Gets the main font size.
    pub fn get_main_font_size(&self) -> f32 {
        self.get()[&zencstr!("main_font_size").data]
            .as_u64()
            .unwrap_or(18) as f32
    }

    /// Gets the header font size.
    pub fn get_header_font_size(&self) -> f32 {
        self.get()[&zencstr!("header_font_size").data]
            .as_u64()
            .unwrap_or(26) as f32
    }

    /// Gets the list of custom fonts to be added onto the UI.
    pub fn get_fonts(&self) -> Option<Vec<(&String, f32)>> {
        let mut fonts = None;
        for (relative_font_path, font_size) in self
            .get()
            .get(&zencstr!("fonts").data)?
            .as_object()
            .unwrap_or_crash(zencstr!(
                "[ERROR] \"fonts\" couldn't be turned into an object!"
            ))
        {
            let Some(font_size) = font_size.as_u64() else {
                log!(
                    "[ERROR] Font Size of font at ",
                    relative_font_path,
                    " isn't a valid u64 and will therefore not be added!"
                );
                continue;
            };

            let Some(fonts) = fonts.as_mut() else {
                fonts = Some(vec![(relative_font_path, font_size as f32)]);
                continue;
            };

            fonts.push((relative_font_path, font_size as f32));
        }

        fonts
    }

    /// Gets the startup Rune scripts to execute, if any.
    pub fn get_startup_rune_scripts(&self) -> Option<Vec<String>> {
        Some(
            self.get()[&zencstr!("startup_rune_scripts").data]
                .as_array()?
                .iter()
                .map(|entry| {
                    entry
                        .as_str()
                        .unwrap_or_crash(zencstr!(
                            "[ERROR] Startup Rune script \"",
                            entry,
                            "\" is not a valid string!"
                        ))
                        .to_string()
                })
                .collect(),
        )
    }

    /// If `true`, Rune will use a new thread to execute the `main` function.
    /// If not, it's executed on the main thread.
    pub fn get_use_new_rune_thread(&self) -> bool {
        self.get()[&zencstr!("use_new_rune_thread").data]
            .as_bool()
            .unwrap_or(true)
    }
}
