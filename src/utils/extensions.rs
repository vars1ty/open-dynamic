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

/// Extensions for `Result<T, E>`.
pub trait ResultExtensions<T, E> {
    /// Extension function which works as a replacement for `unwrap()`, which doesn't show the
    /// error message when crashing due to how dynamic operate.
    fn dynamic_unwrap(self) -> T
    where
        E: std::fmt::Debug + std::fmt::Display;

    /// Extension function which works as a replacement for `expect(message)`.
    fn dynamic_expect(self, message: ZString) -> T
    where
        E: std::fmt::Debug + std::fmt::Display;
}

impl<T, E> ResultExtensions<T, E> for Result<T, E> {
    fn dynamic_unwrap(self) -> T
    where
        E: std::fmt::Debug + std::fmt::Display,
    {
        match self {
            Ok(value) => value,
            Err(error) => crash!("[ERROR] Tried to unwrap a `None` value, error: ", error),
        }
    }

    fn dynamic_expect(self, message: ZString) -> T
    where
        E: std::fmt::Debug + std::fmt::Display,
    {
        match self {
            Ok(value) => value,
            Err(error) => crash!("[ERROR] ", message, ", error: ", error),
        }
    }
}
