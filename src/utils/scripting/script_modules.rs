use super::{
    fncaller::FNCaller,
    script_core::{ScriptCore, ValueWrapper},
};
use crate::{
    globals::*,
    mod_cores::base_core::BaseCore,
    utils::{
        crosscom::CrossCom,
        dynwidget::WidgetType,
        extensions::{F32Ext, OptionExt},
        runedetour::RDetour,
        scripting::rune_ext_structs::RuneDoubleResultPrimitive,
        stringutils::StringUtils,
        ui::customwindows::CustomWindowsUtils,
    },
    winutils::{AddressType, WinUtils},
};
use dashmap::DashMap;
use indexmap::IndexMap;
use parking_lot::RwLock;
use rune::{alloc::clone::TryClone, runtime::Function, ContextError, Module, Value};
use std::{
    ffi::CString,
    fmt::{Debug, Display},
    rc::Rc,
    str::FromStr,
    sync::{atomic::Ordering, Arc},
};
use windows::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Threading::GetCurrentProcess;
use wmem::Memory;

/// System modules, like Memory operations and such.
pub struct SystemModules;

impl SystemModules {
    /// Builds this module.
    #[optimize(size)]
    pub fn build(
        base_core: Arc<RwLock<BaseCore>>,
        crosscom: Arc<RwLock<CrossCom>>,
        serials: Arc<Vec<String>>,
        global_script_variables: Arc<DashMap<String, ValueWrapper>>,
        thread_keys: Arc<DashMap<String, bool>>,
    ) -> Result<Vec<Module>, ContextError> {
        let mut module = Module::new();
        let mut dynamic_module = Module::with_crate(&zencstr!("dynamic").data)?;
        let mut compiler_module = Module::with_crate(&zencstr!("Compiler").data)?;
        let mut task_module = Module::with_crate(&zencstr!("Task").data)?;
        let mut parse_module = Module::with_crate(&zencstr!("Parse").data)?;
        let mut math_module = Module::with_crate(&zencstr!("Math").data)?;
        let mut windows_module = Module::with_crate(&zencstr!("Windows").data)?;
        let mut memory_module = Module::with_crate(&zencstr!("Memory").data)?;
        let mut sellix_module = Module::with_crate(&zencstr!("Sellix").data)?;
        let mut config_module = Module::with_crate(&zencstr!("Config").data)?;
        let mut arctic_module = Module::with_crate(&zencstr!("Arctic").data)?;
        let mut std_module = Module::with_crate(&zencstr!("std").data)?;

        module.ty::<RuneDoubleResultPrimitive>()?;
        module.ty::<AddressType>()?;

        dynamic_module
            .function("log", |data: String| {
                log!(data);
            })
            .build()?;

        let thread_keys_clone = Arc::clone(&thread_keys);
        dynamic_module
            .function("create_thread_key", move |identifier| {
                Self::create_thread_key(identifier, Arc::clone(&thread_keys_clone))
            })
            .build()?;

        let thread_keys_clone = Arc::clone(&thread_keys);
        dynamic_module
            .function("get_thread_key", move |identifier| {
                Self::get_thread_key(identifier, Arc::clone(&thread_keys_clone))
            })
            .build()?;

        let thread_keys_clone = Arc::clone(&thread_keys);
        dynamic_module
            .function("set_thread_key_value", move |identifier, enabled| {
                Self::set_thread_key_value(identifier, enabled, Arc::clone(&thread_keys_clone))
            })
            .build()?;
        dynamic_module
            .function("is_key_down", WinUtils::is_key_down)
            .build()?;
        dynamic_module
            .function("get_delta_time", || DELTA_TIME.load(Ordering::SeqCst))
            .build()?;
        compiler_module
            .function("run_multi_threaded", Self::run_multi_threaded)
            .build()?;
        task_module
            .function("sleep_secs", Self::sleep_secs)
            .build()?;
        task_module.function("sleep_ms", Self::sleep_ms).build()?;
        parse_module.function("i8", Self::r#as::<i8>).build()?;
        parse_module.function("u8", Self::r#as::<u8>).build()?;
        parse_module.function("i16", Self::r#as::<i16>).build()?;
        parse_module.function("u16", Self::r#as::<u16>).build()?;
        parse_module.function("i32", Self::r#as::<i32>).build()?;
        parse_module.function("u32", Self::r#as::<u32>).build()?;
        parse_module.function("i64", Self::r#as::<i64>).build()?;
        parse_module.function("u64", Self::r#as::<u64>).build()?;
        parse_module.function("f32", Self::r#as::<f32>).build()?;
        parse_module.function("f64", Self::r#as::<f64>).build()?;
        parse_module.function("bool", Self::r#as::<bool>).build()?;
        parse_module
            .function("hex_to_primitive", WinUtils::hex_to_primitive)
            .build()?;
        math_module
            .function("sin", |value: f32| value.sin())
            .build()?;
        math_module
            .function("cos", |value: f32| value.cos())
            .build()?;
        math_module
            .function("pi", || std::f32::consts::PI)
            .build()?;
        math_module
            .function("to_radians", |value: f32| value.to_radians())
            .build()?;
        windows_module
            .function("get_cursor_xy", Self::get_cursor_xy)
            .build()?;
        windows_module
            .function("show_alert", |caption: String, text: String| {
                WinUtils::display_message_box(&caption, &text, 0x00000010)
            })
            .build()?;
        windows_module
            .function("get_base_of_module", |module_name: String| {
                WinUtils::get_base_of(&module_name) as i64
            })
            .build()?;
        windows_module
            .function(
                "get_address_of_symbol",
                |module_name: String, symbol: String| {
                    WinUtils::get_module_symbol_address(
                        module_name,
                        &CString::new(symbol).unwrap_or_else(|error| {
                            crash!(
                                "[ERROR] Failed converting symbol to a C-String, error: ",
                                error
                            )
                        }),
                    )
                    .map(|value| value as i64)
                },
            )
            .build()?;
        memory_module.function("write", Self::write).build()?;
        memory_module
            .function("read", Self::read_primitive)
            .build()?;
        memory_module.function("scan", Self::pattern_scan).build()?;
        memory_module
            .function("read_string", Self::read_string)
            .build()?;

        memory_module
            .function("fn_call", FNCaller::call_auto)
            .build()?;
        memory_module
            .function("fn_call_raw", FNCaller::call_auto_raw)
            .build()?;
        memory_module
            .function("hook_function", RDetour::install_detour_auto)
            .build()?;
        memory_module
            .function("drop_hook", RDetour::drop_rdetour_at)
            .build()?;
        memory_module
            .function("free_cstring", |ptr: i64| {
                if ptr == 0 {
                    log!("[ERROR] free_cstring called with nullptr, cancelling.");
                    return;
                }

                drop(unsafe { CString::from_raw(ptr as _) });
            })
            .build()?;

        math_module
            .function("lerp", |value: f32, to: f32, time: f32| {
                value.lerp(to, time)
            })
            .build()?;
        math_module.function("ptr_add", Self::ptr_add).build()?;
        math_module.function("ptr_sub", Self::ptr_sub).build()?;

        let serials_clone = Arc::clone(&serials);
        sellix_module
            .function(
                "is_paying_for_product",
                move |product_id: String, bearer_token: String| {
                    crosscom.read().check_is_ex_serial_ok(
                        product_id,
                        bearer_token,
                        Arc::clone(&serials_clone),
                    )
                },
            )
            .build()?;
        config_module
            .function("has_serial", move |serial: String| {
                serials.contains(&serial)
            })
            .build()?;

        let base_core_clone = Arc::clone(&base_core);
        arctic_module
            .function("inject_gateway", move |dll_name| {
                base_core_clone
                    .read()
                    .get_arctic_core()
                    .get()
                    .unwrap_or_crash(zencstr!(
                        "[ERROR] Unitialized Arctic instance inside of Script Engine!"
                    ))
                    .inject_plugin(dll_name)
            })
            .build()?;

        let base_core_clone = Arc::clone(&base_core);
        arctic_module
            .function("is_gateway_active", move |identifier| {
                base_core_clone
                    .read()
                    .get_arctic_core()
                    .get()
                    .unwrap_or_crash(zencstr!(
                        "[ERROR] Unitialized Arctic instance inside of Script Engine!"
                    ))
                    .is_gateway_active(identifier)
            })
            .build()?;

        std_module
            .function("get_lines_from_string", |input: String| {
                input
                    .lines()
                    .map(|line| line.to_owned())
                    .collect::<Vec<_>>()
            })
            .build()?;

        std_module
            .function("read_file", std::fs::read_to_string::<String>)
            .build()?;

        std_module
            .function("value_as_ptr", |value: Value| {
                ScriptCore::value_as_ptr(&value).map(|value| value as i64)
            })
            .build()?;

        std_module
            .function("get_random_string", StringUtils::get_random)
            .build()?;

        std_module.function("ftoi", |f: f32| f as i32).build()?;
        std_module.function("dtoi", |f: f64| f as i32).build()?;

        std_module.function("itos", |i: i32| i as i8).build()?;
        std_module.function("itol", |i: i32| i as i64).build()?;
        std_module.function("itof", |i: i32| i as f32).build()?;
        std_module.function("itod", |i: i32| i as f64).build()?;

        std_module.function("ltos", |l: i64| l as i8).build()?;
        std_module.function("ltoi", |l: i64| l as i32).build()?;
        std_module.function("ltof", |l: i64| l as f32).build()?;
        std_module.function("ltod", |l: i64| l as f64).build()?;

        let global_script_variables_clone = Arc::clone(&global_script_variables);
        std_module
            .function("define_global", move |variable_name, value| {
                Self::define_global(
                    variable_name,
                    value,
                    Arc::clone(&global_script_variables_clone),
                )
            })
            .build()?;

        let global_script_variables_clone = Arc::clone(&global_script_variables);
        std_module
            .function("get_global", move |variable_name| {
                Self::get_global(variable_name, Arc::clone(&global_script_variables_clone))
            })
            .build()?;

        Ok(vec![
            module,
            dynamic_module,
            compiler_module,
            task_module,
            parse_module,
            math_module,
            windows_module,
            memory_module,
            sellix_module,
            config_module,
            arctic_module,
            std_module,
        ])
    }

    /// Defines a new global variable if not present, otherwise updates the existing variable.
    fn define_global(
        variable_name: String,
        value: Value,
        global_script_variables: Arc<DashMap<String, ValueWrapper>>,
    ) {
        global_script_variables.insert(variable_name, ValueWrapper(value));
    }

    /// Gets a clone of the value from the identified global variable.
    fn get_global(
        variable_name: String,
        global_script_variables: Arc<DashMap<String, ValueWrapper>>,
    ) -> Option<Value> {
        global_script_variables.get(&variable_name).map(|value| {
            value
                .0
                .try_clone() // Stupid, but either that or &'static due to lifetime issues.
                .unwrap_or_else(|error| crash!("[ERROR] Failed cloning value, error: ", error))
        })
    }

    /// Runs a defined function on a new thread. This is especially useful when the user doesn't
    /// want to block the main thread, or the already newly-created thread from the special
    /// compiler option.
    fn run_multi_threaded(function: Function, arg1: Option<Value>) {
        let arg1 = arg1.map(ValueWrapper);
        let function = function.into_sync().into_result().unwrap_or_else(|error| {
            crash!(
                "[ERROR] Failed turning Function into SyncFunction, error: ",
                error
            )
        });

        std::thread::spawn(move || {
            let Err(error) = function
                .call::<_, ()>((arg1.map(|value| value.0),))
                .into_result()
            else {
                return;
            };

            log!(
                "[ERROR] Failed calling function on a new thread, error: ",
                error
            );
        });
    }

    /// Attempts to read a string at `address`.
    pub fn read_string(address: i64) -> &'static str {
        WinUtils::ptr_to_string(address as _).unwrap_or_default()
    }

    /// Adds `add` to the pointer, then returns the new value.
    fn ptr_add(address: i64, add: i64) -> i64 {
        address + add
    }

    /// Subtracts `sub` from the pointer, then returns the new value.
    fn ptr_sub(address: i64, sub: i64) -> i64 {
        address + sub
    }

    /// Scans for a pattern in memory.
    fn pattern_scan(hex_string: String, address_type: AddressType) -> Vec<i64> {
        let ptr = hex_string.as_ptr();
        WinUtils::find_from_signature(
            address_type,
            None,
            &StringUtils::hex_string_to_bytes(hex_string)
                .unwrap_or_crash(zencstr!("[ERROR] Failed converting hex string into bytes!")),
            true,
        )
        .iter()
        .map(|address| *address as i64)
        .filter(|address| *address != 0 && *address != ptr as i64)
        .collect()
    }

    /// Writes to the specified memory address.
    /// Supports these types:
    /// - Integers
    /// - Unsigned Integers
    /// - Decimals, only f32 for now
    /// - Strings with automatic termination
    /// - Byte-arrays via a special string: `"b[00 00 00 00]"` - The byte array being embedded
    ///   within `b[...]`.
    fn write(address: i64, data: Value) {
        if address == 0 {
            log!("[ERROR] Address passed into Memory::write was null!");
            return;
        }

        let current_process_handle = HANDLE(unsafe { GetCurrentProcess() } as isize);

        // Read one byte from the address just to see if it errors and potentially hinder crashes.
        if let Err(error) =
            Memory::read::<u8>(&current_process_handle, address as *const i64, Some(1))
        {
            log!(
                "[ERROR] Memory at address ",
                format!("{:?}", address as *const i64),
                " is not valid, error: ",
                error
            );
            return;
        }

        let on_error = |error: windows::core::Error| {
            log!(
                "[ERROR] Failed writing to memory address at ",
                format!("{:?}", address as *const i64),
                ", error: ",
                format!("{error:?}")
            );
        };

        if let Ok(data_i64) = data.as_integer().into_result() {
            if let Err(error) = Memory::write(
                &current_process_handle,
                address as _,
                &(data_i64 as i32),
                None,
            ) {
                on_error(error);
            }

            return;
        }

        if let Ok(data_usize) = data.as_usize().into_result() {
            if let Err(error) =
                Memory::write(&current_process_handle, address as _, &data_usize, None)
            {
                on_error(error);
            }

            return;
        }

        if let Ok(data_f64) = data.as_float().into_result() {
            if let Err(error) = Memory::write(
                &current_process_handle,
                address as _,
                &(data_f64 as f32),
                None,
            ) {
                on_error(error);
            }

            return;
        }

        let Ok(data_string) = data.to_owned().into_string().into_result() else {
            return;
        };

        let Ok(data_string) = data_string.borrow_ref() else {
            log!("[ERROR] Invalid type to be written!");
            log!("[INFO] You may only use primitive values, strings and byte-strings!");
            return;
        };

        let mut bytes = data_string.as_bytes().to_vec();
        if !bytes.ends_with(b"\0") {
            bytes.push(b'\0');
        }

        // If the string starts with 'b[' and ends with ']', it's classified as a
        // byte-string. Content goes inside 'b[]'.
        zencstr!("b[").use_string(|start_pfx| {
            if data_string.starts_with(&*start_pfx) && data_string.ends_with(']') {
                if let Some(data_bytes) = StringUtils::hex_string_to_bytes(
                    data_string.replace(&*start_pfx, "").replace([']', ' '], ""),
                ) {
                    bytes = data_bytes;
                }
            }
        });

        if let Err(error) = Memory::write(
            &current_process_handle,
            address as _,
            &bytes,
            Some(bytes.len()),
        ) {
            on_error(error);
        }
    }

    /// Reads a primitive from `address`.
    fn read_primitive(address: i64) -> RuneDoubleResultPrimitive {
        if address == 0 {
            log!("[ERROR] read_primitive called with a nullptr, returning RuneDoubleResultPrimitive::default()!");
            return RuneDoubleResultPrimitive::default();
        }

        unsafe {
            let read_i64: i64 = std::ptr::read(address as _);
            let read_f32: f32 = std::ptr::read(address as _);
            let read_f64: f64 = std::ptr::read(address as _);
            RuneDoubleResultPrimitive::new(
                read_i64 as i8,
                read_i64 as i32,
                read_i64,
                read_f32,
                read_f64,
            )
        }
    }

    /// Gets the X and Y-Coordinate of the cursor.
    fn get_cursor_xy() -> Vec<RuneDoubleResultPrimitive> {
        let mut vec = Vec::with_capacity(2);
        let cursor_pos = WinUtils::get_cursor_pos();
        let (x, y) = (cursor_pos.x, cursor_pos.y);
        vec.push(RuneDoubleResultPrimitive::new(
            x as i8, x, x as i64, x as f32, x as f64,
        ));
        vec.push(RuneDoubleResultPrimitive::new(
            y as i8, y, y as i64, y as f32, y as f64,
        ));
        vec
    }

    /// Attempts to parse the given data as a number.
    fn r#as<T: FromStr + Debug + Default>(data: &str) -> T
    where
        <T as FromStr>::Err: Display,
    {
        data.parse().unwrap_or_else(|error| {
            log!(
                "[ERROR] Failed parsing \"",
                data,
                "\", returning ",
                std::any::type_name::<T>(),
                "::default(). Error: ",
                error
            );
            T::default()
        })
    }

    /// Puts the calling task to sleep for a few seconds.
    fn sleep_secs(seconds: u64) {
        std::thread::sleep(std::time::Duration::from_secs(seconds));
    }

    /// Puts the calling thread to sleep for a few milliseconds.
    fn sleep_ms(ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    /// Creates a new Thread key.
    /// Thread keys are unique keys with a `bool` value which should be checked in
    /// never-ending/long-running loops, as it's used to stop their execution.
    /// Without this, they won't stop until the program restarts.
    pub fn create_thread_key(key: String, thread_keys: Arc<DashMap<String, bool>>) {
        thread_keys.insert(key, true);
    }

    /// Gets the value of a Thread key.
    pub fn get_thread_key(key: String, thread_keys: Arc<DashMap<String, bool>>) -> bool {
        *thread_keys.get(&key).unwrap_or_crash(zencstr!(
            "[ERROR] Thread key ",
            key,
            " has not been defined!"
        ))
    }

    /// Sets the value of a Thread key.
    pub fn set_thread_key_value(
        key: String,
        enabled: bool,
        thread_keys: Arc<DashMap<String, bool>>,
    ) {
        thread_keys
            .entry(key)
            .and_modify(|value| *value = enabled)
            .or_insert(enabled);
    }
}

/// ImGui Modules.
pub struct UIModules;

impl UIModules {
    /// Builds this module.
    #[optimize(size)]
    pub fn build(custom_window_utils: &'static CustomWindowsUtils) -> Result<Module, ContextError> {
        let mut module = Module::with_crate(&zencstr!("ui").data)?; // <-- TODO: Rename to `UI`.

        module
            .function("add_window", |name| custom_window_utils.add_window(name))
            .build()?;

        module
            .function("remove_window", |name| {
                custom_window_utils.remove_window(name)
            })
            .build()?;

        module
            .function("rename_window", |from_name, to_name| {
                custom_window_utils.rename_window(from_name, to_name)
            })
            .build()?;

        module
            .function("focus_window", |name| {
                custom_window_utils.set_current_window_to(name)
            })
            .build()?;

        module
            .function("add_label", |identifier, content| {
                custom_window_utils.add_widget(identifier, WidgetType::Label(content, 0))
            })
            .build()?;

        module
            .function("add_bold_label", |identifier, content| {
                custom_window_utils.add_widget(identifier, WidgetType::Label(content, 2));
            })
            .build()?;

        module
            .function(
                "add_custom_font_label",
                |identifier, content, relative_font_path| {
                    custom_window_utils.add_widget(
                        identifier,
                        WidgetType::LabelCustomFont(content, Arc::new(relative_font_path)),
                    )
                },
            )
            .build()?;

        module
            .function("add_button", |identifier, text, function, opt_param| {
                custom_window_utils.add_widget(
                    identifier,
                    WidgetType::Button(text, Rc::new(function), opt_param),
                )
            })
            .build()?;

        module
            .function("add_legacy_button", |identifier, text, rune_code| {
                log!("[WARN] Legacy buttons will be removed in the future!");
                custom_window_utils
                    .add_widget(identifier, WidgetType::LegacyButton(text, rune_code))
            })
            .build()?;

        module
            .function("add_separator", |identifier| {
                custom_window_utils.add_widget(identifier, WidgetType::Separator)
            })
            .build()?;

        module
            .function("add_spacing", |identifier, x, y| {
                custom_window_utils.add_widget(identifier, WidgetType::Spacing(x, y))
            })
            .build()?;

        module
            .function(
                "add_f32_slider",
                |identifier, text, (min, max), function, opt_param| {
                    custom_window_utils.add_widget(
                        identifier,
                        WidgetType::F32Slider(text, min, max, min, Rc::new(function), opt_param),
                    )
                },
            )
            .build()?;

        module
            .function(
                "add_i32_slider",
                |identifier, text, (min, max), function, opt_param| {
                    custom_window_utils.add_widget(
                        identifier,
                        WidgetType::I32Slider(text, min, max, min, Rc::new(function), opt_param),
                    )
                },
            )
            .build()?;

        module
            .function("get_f32_slider_value", |identifier| {
                custom_window_utils.get_f32_slider_value(identifier)
            })
            .build()?;

        module
            .function("get_i32_slider_value", |identifier| {
                custom_window_utils.get_i32_slider_value(identifier)
            })
            .build()?;

        module
            .function("remove_widget", |identifier| {
                custom_window_utils.remove_widget(identifier)
            })
            .build()?;

        module
            .function("remove_all_widgets", || {
                custom_window_utils.remove_all_widgets()
            })
            .build()?;

        module
            .function("set_next_item_width", |identifier, width| {
                custom_window_utils.add_widget(identifier, WidgetType::NextWidgetWidth(width))
            })
            .build()?;

        module
            .function("set_next_item_same_line", |identifier| {
                custom_window_utils.add_widget(identifier, WidgetType::SameLine)
            })
            .build()?;

        module
            .function(
                "add_image",
                |identifier, image_path, width, height, rune_code| {
                    custom_window_utils.add_widget(
                        identifier,
                        WidgetType::Image(image_path, width, height, false, false, rune_code),
                    )
                },
            )
            .build()?;

        module
            .function(
                "add_image_overlay",
                |identifier, image_path, width, height| {
                    custom_window_utils.add_widget(
                        identifier,
                        WidgetType::Image(image_path, width, height, true, false, "".to_owned()),
                    )
                },
            )
            .build()?;

        module
            .function(
                "add_image_background",
                |identifier, image_path, width, height| {
                    custom_window_utils.add_widget(
                        identifier,
                        WidgetType::Image(image_path, width, height, false, true, "".to_owned()),
                    )
                },
            )
            .build()?;

        module
            .function(
                "replace_image",
                |identifier, new_image_path, width, height| {
                    custom_window_utils.replace_image(identifier, new_image_path, [width, height])
                },
            )
            .build()?;

        module
            .function("set_size_constraints", |min_x, min_y, max_x, max_y| {
                custom_window_utils.set_active_window_size_constraints([min_x, min_y, max_x, max_y])
            })
            .build()?;

        module
            .function("clear_cached_images", || {
                custom_window_utils.clear_cached_images()
            })
            .build()?;

        module
            .function("hide_widgets", |identifiers| {
                custom_window_utils.hide_widgets(identifiers)
            })
            .build()?;

        module
            .function("show_widgets", |identifiers| {
                custom_window_utils.show_widgets(identifiers)
            })
            .build()?;

        module
            .function("add_centered_widget_group", |identifier, custom_y| {
                custom_window_utils.add_widget(
                    identifier,
                    WidgetType::CenteredWidgets(IndexMap::new(), custom_y, [0.0, 0.0]),
                )
            })
            .build()?;

        module
            .function("set_auto_center_into", |identifier| {
                custom_window_utils.set_widget_auto_centered_into(identifier)
            })
            .build()?;

        module
            .function(
                "add_input_text_multiline",
                |identifier, label, width, height| {
                    custom_window_utils.add_widget(
                        identifier,
                        WidgetType::InputTextMultiLine(label, String::default(), width, height),
                    )
                },
            )
            .build()?;

        module
            .function("get_input_text_multiline_value", |identifier| {
                custom_window_utils.get_input_text_multiline_value(identifier)
            })
            .build()?;

        module
            .function("retain_widgets_by_identifiers", |identifiers| {
                custom_window_utils.retain_widgets_by_identifiers(identifiers)
            })
            .build()?;

        module
            .function("is_cursor_in_ui", || {
                IS_CURSOR_IN_UI.load(Ordering::Relaxed)
            })
            .build()?;

        Ok(module)
    }
}
