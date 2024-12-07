/// Logs a message to `stdout` and to the side of the screen, if ImGui is active.
#[macro_export]
macro_rules! log {
    ($arg:literal) => {
        $crate::winutils::WinUtils::log_message(zencstr!("[", file!(), ":", line!(), "]: ", $arg), true);
    };
    ($arg:expr) => {
        $crate::winutils::WinUtils::log_message(zencstr!("[", file!(), ":", line!(), "]: ", $arg), true);
    };
    ($($arg:expr),*) => {
        {
            $crate::winutils::WinUtils::log_message(zencstr!("[", file!(), ":", line!(), "]: "), false);
            $(
                $crate::winutils::WinUtils::log_message(zencstr!(format!("{}", encrypt_arg!($arg))), false);
            )*
            $crate::winutils::WinUtils::log_message(zencstr!(""), true);
        }
    };
}

/// Formats an argument.
/// If literal, it attempts to use encryption.
/// If not, it returns the argument without any processing.
#[macro_export]
macro_rules! encrypt_arg {
    ($arg:literal) => {
        $crate::utils::cryptutils::CryptUtils::decrypt(obfstr!(sbyt::str_to_byte_slice!($arg)))
    };
    ($arg:expr) => {
        $arg
    };
}

/// Combines multiple literals and/or expressions into one single encrypted `ZString`, then returns
/// it.
#[macro_export]
macro_rules! zencstr {
    ($arg:literal) => {
        $crate::zstring::ZString::new(encrypt_arg!($arg))
    };
    ($($arg:expr),*) => {
        {
            let mut output = $crate::zstring::ZString::default();
            $(
                output.push_zstring($crate::zstring::ZString::new(encrypt_arg!($arg).to_string()));
            )*
            output
        }
    };
}

/// Clone of `zencstr`, but takes the encrypted `String` and returns it.
#[macro_export]
macro_rules! ozencstr {
    ($arg:literal) => {
        std::mem::take(&mut $crate::zstring::ZString::new(encrypt_arg!($arg)).data)
    };
    ($($arg:expr),*) => {
        {
            let mut output = $crate::zstring::ZString::default();
            $(
                output.push_zstring($crate::zstring::ZString::new(encrypt_arg!($arg).to_string()));
            )*
            std::mem::take(&mut output.data)
        }
    };
}

/// Crashes the program after 5 seconds of displaying a custom message.
#[macro_export]
macro_rules! crash {
    ($arg:literal) => {{
        log!($arg);
        $crate::winutils::WinUtils::display_message_box(&zencstr!("dynamic").data, &encrypt_arg!($arg), 0x00000010);
        $crate::winutils::WinUtils::sleep_and_exit(5)
    }};
    ($($arg:expr),*) => {
        {
            print!("{}", zencstr!("[", file!(), ":", line!(), "]: "));
            let mut message = $crate::zstring::ZString::default();
            $(
                $crate::utils::stringutils::StringUtils::crash_helper_append(&mut message, encrypt_arg!($arg));
            )*
            println!();
            $crate::winutils::WinUtils::display_message_box(&zencstr!("dynamic").data, &message.data, 0x00000010);
            $crate::winutils::WinUtils::sleep_and_exit(5)
        }
    };
}

/// Quicker way of defining macros, mainly intended as a C++-like replacement for #define.
#[macro_export]
macro_rules! define {
    ($name:ident, $data:literal) => {
        macro_rules! $name {
            () => {
                $data
            };
        }
    };
    ($name:ident, $data:expr) => {
        macro_rules! $name {
            () => {
                $data
            };
        }
    };
}

/// Enables a hook.
#[macro_export]
macro_rules! enable_hook {
    ($hook:expr, $fn_address:expr, $callback:expr, $hook_name:literal) => {
        std::thread::spawn(move || unsafe {
            #[allow(clippy::missing_transmute_annotations)]
            let hook = $hook.initialize(std::mem::transmute($fn_address), $callback);
            if let Ok(hook) = hook {
                if let Err(error) = hook.enable() {
                    log!(
                        "[ERROR] Failed enabling hook ",
                        $hook_name,
                        ", error: ",
                        error
                    );
                } else {
                    log!("Hook ", $hook_name, " loaded successfully!");
                }
            } else {
                log!(
                    "[ERROR] Failed initializing hook ",
                    $hook_name,
                    ", error: ",
                    hook.unwrap_err_unchecked()
                );
            }
        })
    };
}

/// Fills the memory-space at a specific location with `data.len()` amount of bytes.
#[macro_export]
macro_rules! zero {
    ($data:expr) => {{
        unsafe { std::ptr::write_bytes($data.as_ptr() as *mut u8, 0u8, $data.len()) }
    }};
    ($data:expr, $amount:expr) => {{
        unsafe { std::ptr::write_bytes($data.as_ptr() as *mut u8, 0u8, $amount) }
    }};
}

/// Constructs a new `Label`. If the string is literal/constant, it'll be using `zencstr!()`.
#[macro_export]
macro_rules! label {
    ($ui:expr, $data:literal) => {
        $ui.text(zencstr!($data))
    };
    ($ui:expr, $data:expr) => {
        $ui.text($data)
    };
}

/// Constructs a new `Button` with a constant string which uses `zencstr!()`.
#[macro_export]
macro_rules! button {
    ($ui:expr, $text:literal) => {
        $ui.button(zencstr!($text))
    };
    ($ui:expr, $text:expr) => {
        $ui.button($text)
    };
}

/// Makes a list of structure(s) thread-safe using `unsafe impl Send` and `Sync`.
#[macro_export]
macro_rules! thread_safe_structs {
    ($($structures:ty),*) => {
        $(
            unsafe impl Send for $structures {}
            unsafe impl Sync for $structures {}
        )*
    };
}

/// Constructs a new `Slider`. If the string is literal, it'll be using `zencstr!()`.
#[macro_export]
macro_rules! slider {
    ($ui:expr, $text:literal, $min:expr, $max:expr, $out:expr) => {
        $crate::utils::eguiutils::ImGuiUtils::slider($ui, zencstr!($text), $min, $max, &mut $out)
    };
    ($ui:expr, $text:expr, $min:expr, $max:expr, $out:expr) => {
        $crate::utils::eguiutils::ImGuiUtils::slider($ui, $text, $min, $max, &mut $out)
    };
}
