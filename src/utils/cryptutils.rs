pub struct CryptUtils;

impl CryptUtils {
    /// Decrypts the given `&str`.
    #[optimize(speed)]
    pub fn decrypt(data: &str) -> String {
        magic_crypt::MagicCryptTrait::decrypt_base64_to_string(
            &magic_crypt::new_magic_crypt!(
                obfstr!("dzPo4Nzr7f#rmzRNQpL4psTUJYqk9*n9g@2iBzond*z4LQWY!9Zw^Ags3433N6KWxbLcuWuXikq7#EvqiogRWufwja!R8UtaNkt5xf*Zc%M"),
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
