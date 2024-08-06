#![allow(non_camel_case_types)]
#![allow(clippy::upper_case_acronyms)]

/// Equivalent to `const char*` in C/C++.
pub type char_ptr = *const u8;
