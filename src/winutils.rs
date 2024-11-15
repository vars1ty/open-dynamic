use crate::{
    globals::{SafeMODULEENTRY32, LOGGED_MESSAGES, MODULES},
    utils::{crosscom::CrossCom, extensions::OptionExt, types::char_ptr},
};
use ahash::AHashMap;
use parking_lot::RwLock;
use rune::Any;
use std::{collections::HashMap, ffi::*, os::windows::prelude::OsStringExt, sync::Arc};
use windows::{
    core::PCSTR,
    System::VirtualKey,
    Win32::{
        Foundation::{HANDLE, MAX_PATH},
        System::{Diagnostics::ToolHelp::MODULEENTRY32, LibraryLoader::*},
    },
};
use windows_sys::Win32::{
    Foundation::POINT,
    Graphics::Gdi::ScreenToClient,
    System::Threading::GetCurrentProcess,
    UI::{Input::KeyboardAndMouse::GetAsyncKeyState, WindowsAndMessaging::*},
};
use wmem::Memory;
use zstring::ZString;

/// Wrapper around `POINT` which implements `Default`.
#[derive(Clone, Copy)]
pub struct POINTWrapper(pub POINT);

impl Default for POINTWrapper {
    fn default() -> Self {
        Self(POINT { x: 0, y: 0 })
    }
}

/// Renderer enum for determing the render target for an unsupported game.
#[derive(Debug, Default)]
pub enum Renderer {
    DirectX9,
    DirectX11,
    DirectX12,
    OpenGL,

    #[default]
    None,
}

/// Extension trait for `VirtualKey`.
pub trait VirtualKeyExt {
    /// Gets the `VirtualKey` from a string.
    /// Only a limited set of keys are included.
    fn from_str(str: &str) -> VirtualKey;
}

impl VirtualKeyExt for VirtualKey {
    fn from_str(str: &str) -> VirtualKey {
        match str {
            "A" => VirtualKey::A,
            "B" => VirtualKey::B,
            "C" => VirtualKey::C,
            "D" => VirtualKey::D,
            "E" => VirtualKey::E,
            "F" => VirtualKey::F,
            "G" => VirtualKey::G,
            "H" => VirtualKey::H,
            "I" => VirtualKey::I,
            "J" => VirtualKey::J,
            "K" => VirtualKey::K,
            "L" => VirtualKey::L,
            "M" => VirtualKey::M,
            "N" => VirtualKey::N,
            "O" => VirtualKey::O,
            "P" => VirtualKey::P,
            "Q" => VirtualKey::Q,
            "R" => VirtualKey::R,
            "S" => VirtualKey::S,
            "T" => VirtualKey::T,
            "U" => VirtualKey::U,
            "V" => VirtualKey::V,
            "W" => VirtualKey::W,
            "X" => VirtualKey::X,
            "Y" => VirtualKey::Y,
            "Z" => VirtualKey::Z,
            "0" => VirtualKey::Number0,
            "1" => VirtualKey::Number1,
            "2" => VirtualKey::Number2,
            "3" => VirtualKey::Number3,
            "4" => VirtualKey::Number4,
            "5" => VirtualKey::Number5,
            "6" => VirtualKey::Number6,
            "7" => VirtualKey::Number7,
            "8" => VirtualKey::Number8,
            "9" => VirtualKey::Number9,
            "F1" => VirtualKey::F1,
            "F2" => VirtualKey::F2,
            "F3" => VirtualKey::F3,
            "F4" => VirtualKey::F4,
            "F5" => VirtualKey::F5,
            "F6" => VirtualKey::F6,
            "F7" => VirtualKey::F7,
            "F8" => VirtualKey::F8,
            "F9" => VirtualKey::F9,
            "F10" => VirtualKey::F10,
            "F11" => VirtualKey::F11,
            "F12" => VirtualKey::F12,
            "Space" => VirtualKey::Space,
            "Control" => VirtualKey::Control,
            "Left" => VirtualKey::Left,
            "Right" => VirtualKey::Right,
            "Up" => VirtualKey::Up,
            "Down" => VirtualKey::Down,
            "Shift" => VirtualKey::Shift,
            _ => VirtualKey::None,
        }
    }
}

/// Address types.
#[derive(PartialEq, Eq, Any)]
pub enum AddressType {
    #[rune(constructor)]
    Static,

    #[rune(constructor)]
    Any,
}

/// Windows utilities.
pub struct WinUtils;

impl WinUtils {
    /// Gets the path to a module.
    pub fn get_module_path(name: ZString) -> ZString {
        unsafe {
            let dll_handle = GetModuleHandleA(PCSTR(name.data.as_ptr())).unwrap_or_else(|error| {
                crash!(
                    "[ERROR] Failed to get a valid handle to the DLL named: ",
                    &name,
                    ", error: ",
                    error
                )
            });

            let mut buffer: [u16; MAX_PATH as _] = [0; MAX_PATH as _];
            let size = GetModuleFileNameW(dll_handle, &mut buffer);
            if size == 0 {
                crash!(
                    "[ERROR] Failed retrieving path to the \"",
                    name,
                    "\" module, size returned by GetModuleFileNameW was 0!"
                );
            }

            ZString::new(
                OsString::from_wide(&buffer[..size as _])
                    .into_string()
                    .unwrap_or_else(|error| {
                        crash!("[ERROR] Failed to safely convert DLL path to a valid OsString, error: ", format!("{error:?}"))
                    }),
            )
        }
    }

    /// Converts a `*const u8` pointer to a `String` if successful.
    /// Not a safe function by design, but not marked as unsafe as it does try and ensure some form
    /// of safety.
    pub fn ptr_to_string(ptr: char_ptr) -> Option<&'static str> {
        Memory::ptr_to_string(&HANDLE(unsafe { GetCurrentProcess() } as isize), ptr)
    }

    /// Gets a module by its non-exact name.
    /// This uses a `contains()` call, rathern than checking if it's exactly equal to `name`.
    pub fn get_module(name: &str) -> MODULEENTRY32 {
        Self::get_modules()
            .iter()
            .find(|(module_name, _)| module_name.contains(name))
            .map(|(_, value)| value.0)
            .unwrap_or_crash(zencstr!(
                "[ERROR] Couldn't find any module named \"",
                name,
                "\"!"
            ))
    }

    /// Gets the base address of a module.
    pub fn get_base_of(name: &str) -> *mut u8 {
        Self::get_module(name).modBaseAddr
    }

    /// Fetches the modules from the current process and returns them.
    /// This is the non-cache variant of `WinUtils::get_modules`.
    pub fn get_modules_no_cache() -> AHashMap<String, SafeMODULEENTRY32> {
        let modules = Memory::get_modules()
            .unwrap_or_else(|error| crash!("[ERROR] Couldn't get process modules, error: ", error));
        let mut hashmap = AHashMap::new();

        // Insert the modules as Name, SafeMODULEENTRY32.
        for module in modules {
            let module_name = String::from_utf8(Memory::convert_module_name(module.szModule))
                .unwrap_or_else(|error| {
                    crash!(
                        "[ERROR] Couldn't get the module name as a valid UTF-8 String, error: ",
                        error
                    )
                });
            hashmap.insert(module_name, SafeMODULEENTRY32(module));
        }

        hashmap
    }

    /// Caches all the process modules if needed, otherwise returns the internal `AHashMap` with
    /// the module name and the entry.
    pub fn get_modules() -> &'static AHashMap<String, SafeMODULEENTRY32> {
        &MODULES
    }

    /// Converts a byte-slice to its hexadecimal String-form.
    pub fn bytes_to_hex_string(bytes: &[u8]) -> String {
        // Allocate a string with the proper size.
        // 2 bytes for each character in hex.
        // 1 byte for the whitespace between each hex character.
        let mut hex = String::with_capacity(bytes.len() * 3);

        for byte in bytes {
            ZString::new(format!("{:02X} ", byte)).use_string(|data| {
                hex += data;
            });
        }

        if hex.ends_with(' ') {
            hex.pop();
        }

        hex
    }

    /// Finds an address by its signature, `0x7F` is for wildcards.
    /// `module` is only relevant when using `AddressType::Static`.
    #[optimize(speed)]
    pub fn find_from_signature(
        address_type: AddressType,
        module: Option<&str>,
        sig: &[u8],
        include_executable: bool,
    ) -> Vec<*const i64> {
        let handle = Memory::open_current_process().unwrap_or_else(|error| {
            crash!("[ERROR] Failed opening current process, error: ", error)
        });
        let mut results =
            Memory::aob_scan(handle, sig, include_executable).unwrap_or_else(|error| {
                crash!(
                    "[ERROR] Scan failed while looking for ",
                    Self::bytes_to_hex_string(sig),
                    ", error: ",
                    error
                )
            });

        results.retain(|res| *res != sig.as_ptr() as _);
        if address_type == AddressType::Any {
            return results;
        }

        results.retain(|res| {
            if let Some(module) = module {
                let base_address = Self::get_base_of(module) as *const i64;
                // If the address isn't within the defined modules space, remove it.
                return *res >= base_address;
            }

            true
        });

        results
    }

    /// Checks if the given key is being held down.
    pub fn is_key_down(key: &str) -> bool {
        unsafe { (GetAsyncKeyState(VirtualKey::from_str(key).0) as i32 & 0x8000) != 0 }
    }

    /// Parses a hexadecimal value to its normal primitive value.
    pub fn hex_to_primitive(hex: &str) -> i64 {
        i64::from_str_radix(&hex[2..], 16).unwrap_or_else(|error| {
            log!(
                "[ERROR] Hex \"",
                hex,
                "\" couldn't be turned into an i64, falling back to 0. Error: ",
                error
            );
            0
        })
    }

    /// Gets the address to a function inside of a module.
    pub fn get_module_symbol_address<S: AsRef<str>>(module: S, symbol: &CStr) -> Option<usize> {
        unsafe {
            GetModuleHandleA(PCSTR(
                CString::new(module.as_ref())
                    .unwrap_or_else(|error| {
                        crash!("[ERROR] Failed constructing C-String, error: ", error)
                    })
                    .as_ref()
                    .as_ptr() as _,
            ))
            .ok()
            .and_then(|handle| GetProcAddress(handle, PCSTR(symbol.as_ptr() as _)))
            .map(|result| result as usize)
        }
    }

    /// If `variables` contains `variable_name`, the value is parsed and returned.
    /// If it doesn't contain the value, the `sig_scan_address` closure is called, offset is saved,
    /// parsed and returned.
    pub fn server_aob_scan<F: FnOnce() -> usize>(
        variable_name: &str,
        base_address: usize,
        variables: Arc<RwLock<HashMap<String, String>>>,
        sig_scan_address: F,
        crosscom: Arc<RwLock<CrossCom>>,
    ) -> *const i64 {
        if base_address == 0 {
            crash!(
                "[ERROR] Passed base address was null for \"",
                variable_name,
                "\"!"
            );
        }

        // A reader is required, because if we just use read() in the if-statement, then it won't
        // cache the addresses.
        let variables_reader = variables.read();

        // entry() would work here, but it would require a consistent write() lock, slowing things
        // down.
        // In this code, a reader is used to try and get the value. If `None`, it switches to using
        // a writer and inserts the value before returning it.
        let offset: usize = if let Some(value) = variables_reader.get(variable_name) {
            value.parse().unwrap_or_else(|error| {
                crash!(
                    "[ERROR] Failed parsing offset for \"",
                    variable_name,
                    "\" as usize, error: ",
                    error
                )
            })
        } else {
            // Drop reader, we don't need it anymore.
            drop(variables_reader);

            let scan_address = sig_scan_address();
            if scan_address == 0 {
                crash!(
                    "[ERROR] sig_scan_address for \"",
                    variable_name,
                    "\" is null!"
                );
            }

            if scan_address <= base_address {
                crash!(
                    "[ERROR] sig_scan_address for \"",
                    variable_name,
                    "\" is less (or eq.)  to the base address. sig_scan_address == ",
                    scan_address
                );
            }

            // Calculate offset and add it as a string.
            let offset = scan_address - base_address;

            // Insert and tell CrossCom to update the variable.
            let mut variables_writer = variables.write();
            variables_writer.insert(variable_name.to_string(), offset.to_string());
            crosscom.read().send_variables(variables_writer.to_owned());

            drop(variables_writer);
            offset
        };

        unsafe { (base_address as *const i64).byte_add(offset) }
    }

    /// Returns the cursor position within the foreground window.
    /// This requires an already-made `POINT` instance, as it will output the data to it.
    pub fn get_cursor_pos_recycle(point: &mut POINT) {
        unsafe {
            let cursor_pos = GetCursorPos(point);
            if cursor_pos == 0 {
                log!("[ERROR] Failed to call GetCursorPos, initial value in point remains.");
                return;
            }

            ScreenToClient(GetForegroundWindow(), point);
        };
    }

    /// Returns the cursor position within the foreground window.
    pub fn get_cursor_pos() -> POINT {
        let mut point = POINT { x: 0, y: 0 };
        Self::get_cursor_pos_recycle(&mut point);
        point
    }

    /// Puts the calling thread to sleep for the specified amount of seconds, then exits the
    /// process.
    #[inline(always)]
    pub fn sleep_and_exit(secs: u64) -> ! {
        std::thread::sleep(std::time::Duration::from_secs(secs));
        std::process::exit(-1)
    }

    /// Displays a message box.
    pub fn display_message_box(caption: &str, text: &str, message_type: u32) {
        let text_cstr = CString::new(text).unwrap_or_else(|error| {
            crash!(
                "[ERROR] Failed creating C-String out of text, error: ",
                error
            )
        });

        let caption_cstr = CString::new(caption).unwrap_or_else(|error| {
            crash!(
                "[ERROR] Failed creating C-String out of caption, error: ",
                error
            )
        });

        unsafe {
            MessageBoxA(
                GetForegroundWindow(),
                text_cstr.as_ptr() as _,
                caption_cstr.as_ptr() as _,
                message_type,
            )
        };
    }

    /// To be moved to general utils: Logs a message to `LOGGED_MESSAGES` and `stdout`.
    /// # Safety
    /// This should be relatively safe due to the usage of `OnceLock` and `Mutex<ZString>`.
    #[optimize(size)]
    pub fn log_message(mut message: ZString, new_line: bool) {
        let Some(mut logged_messages) = LOGGED_MESSAGES.try_lock() else {
            return;
        };

        message.use_string(|message| {
            if logged_messages.data.lines().count() >= 30 {
                logged_messages.data.clear();
            }

            // new_line(true): Print message before pushing a new line to it, as otherwise it ends up
            // with 2 lines.
            if new_line {
                println!("{message}");
            }

            // new_line(true): Only add a new line if none is present.
            if !message.ends_with('\n') && new_line {
                message.push('\n');
            }

            logged_messages.data.push_str(message);

            if new_line {
                return;
            }

            print!("{message}");
        });
    }
}
