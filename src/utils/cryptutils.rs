pub struct CryptUtils;

impl CryptUtils {
    /// Decrypts the given `&str`.
    #[optimize(speed)]
    pub fn decrypt(data: &str) -> String {
        magic_crypt::MagicCryptTrait::decrypt_base64_to_string(
            &magic_crypt::new_magic_crypt!(
                obfstr!(include_str!("/home/stackalloc/.config/dnx_encryption")),
                256
            ),
            data,
        )
        .unwrap_or_else(|error| {
            panic!(
                "{}{error}",
                obfstr!("[ERROR] String decryption failed, error: ")
            )
        })
    }
}
