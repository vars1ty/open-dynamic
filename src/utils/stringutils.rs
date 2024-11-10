use std::fmt::Display;
use zstring::ZString;

/// Basic string utils.
pub struct StringUtils;

impl StringUtils {
    /// Generates an 8-character long string from the `UNIX_EPOCH` time.
    pub fn get_random() -> String {
        let mut string = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|error| {
                crash!(
                    "[ERROR] Failed getting time since UNIX_EPOCH, error: ",
                    error
                )
            })
            .subsec_nanos()
            .to_string();

        string.truncate(8);
        string
    }

    /// Converts a hex string to its byte-slice representation.
    pub fn hex_string_to_bytes(hex_string: String) -> Option<Vec<u8>> {
        let hex_string = hex_string.to_owned().replace(' ', "").replace("??", "7F");
        if hex_string.len() % 2 != 0 {
            log!(
                "[ERROR] Hex string must be even (hex_string.len() % 2 == 0 FAILED). String failing: \"",
                hex_string,
                "\""
            );
            return None;
        }

        // Pre-allocate a Vec with the right amount of capacity.
        let mut bytes = Vec::with_capacity(hex_string.len() / 2);

        // Iterate over the string by two characters at a time
        for i in (0..hex_string.len()).step_by(2) {
            // Parse two characters as a hexadecimal number. If successful, add the byte.
            let Ok(byte) = u8::from_str_radix(&hex_string[i..i + 2], 16) else {
                log!("[ERROR] u8 at ", i, " cannot be turned into a byte!");
                return None;
            };

            bytes.push(byte);
        }

        Some(bytes)
    }

    /// Helper for `crash!()` with multiple parameters.
    /// Takes the mutable ZString message and appends `encrypted_arg` at the back of it, reducing
    /// the need for 3 lines of extra code for each additional parameter, down to 1.
    #[inline(never)]
    pub fn crash_helper_append(message: &mut ZString, encrypted_param: impl Display) {
        let param_content = ZString::new(encrypted_param.to_string());
        print!("{param_content}");
        message.push_zstring(param_content);
    }
}
