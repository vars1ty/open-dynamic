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
}
