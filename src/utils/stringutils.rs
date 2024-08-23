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
        // Remove whitespaces.
        let hex_string = hex_string
            .replace(' ', "")
            .replace(&zencstr!("??").data, &zencstr!("7F").data);

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
            // Parse two characters as a hexadecimal number
            let byte = match u8::from_str_radix(&hex_string[i..i + 2], 16) {
                Ok(b) => b,
                Err(_) => {
                    log!(
                        "[ERROR] Failed parsing character at position ",
                        i,
                        " into a valid u8!"
                    );
                    return None;
                }
            };

            // Push the parsed byte into the vector
            bytes.push(byte);
        }

        Some(bytes)
    }
}
