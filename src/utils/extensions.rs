use std::hash::{DefaultHasher, Hash, Hasher};
use zstring::ZString;

/// Extensions for `Option<T>`.
pub trait OptionExt<T> {
    /// Gets the value from `self`, or crashes with the specified error message.
    fn unwrap_or_crash(self, message: ZString) -> T;
}

impl<T> OptionExt<T> for Option<T> {
    fn unwrap_or_crash(self, message: ZString) -> T {
        match self {
            Some(value) => value,
            None => crash!(message),
        }
    }
}

/// Extensions for `f32`.
pub trait F32Ext {
    /// Linearly interpolates between two `f32` values.
    fn lerp(&self, to: f32, time: f32) -> f32;
}

impl F32Ext for f32 {
    fn lerp(&self, to: f32, time: f32) -> f32 {
        self + time * (to - self)
    }
}

/// Extensions for `String`.
pub trait StringExtensions {
    /// Hashes `&self` and returns the hashed string.
    fn get_hash(&self) -> String;
}

impl StringExtensions for String {
    fn get_hash(&self) -> String {
        let mut hash = DefaultHasher::new();
        self.hash(&mut hash);
        hash.finish().to_string()
    }
}
