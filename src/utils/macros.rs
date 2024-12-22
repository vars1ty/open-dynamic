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
    ($ui:expr, $text:literal) => {{
        let text_color = $ui.push_style_color(imgui::StyleColor::Text, [0.0, 0.0, 0.0, 1.0]);
        let result = $ui.button(zencstr!($text));
        text_color.pop();

        result
    }};
    ($ui:expr, $text:expr) => {{
        let text_color = $ui.push_style_color(imgui::StyleColor::Text, [0.0, 0.0, 0.0, 1.0]);
        let result = $ui.button($text);
        text_color.pop();

        result
    }};
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
