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
        unsafe {
            // Remove whitespaces with 1 allocation, then replace "??" with "7F" without allocating.
            Self::replace_zero_alloc::<2>(&mut hex_string.replace(' ', ""), *b"??", *b"7F");
        }

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
                return None;
            };

            bytes.push(byte);
        }

        Some(bytes)
    }

    /// Takes `string` and replaces `lookup` with `replacement`.
    /// # Safety
    /// `lookup` **has** to be the same length as `replacement`, as the function works on the raw
    /// underlying bytes and cannot differ in size.
    /// This is enforced at compile-time.
    pub unsafe fn replace_zero_alloc<const N: usize>(
        string: &mut str,
        lookup: [u8; N],
        replacement: [u8; N],
    ) {
        let bytes = unsafe { string.as_bytes_mut() };
        for i in 0..(bytes.len() - N + 1) {
            let slice = &mut bytes[i..i + N];
            if slice == lookup {
                slice.copy_from_slice(&replacement);
            }
        }
    }
}
