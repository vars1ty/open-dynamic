use rune::Any;

/// Structure returned after returning a value that can either be a i32, i64, f32, or f64.
#[derive(Any, Default)]
pub struct RuneDoubleResultPrimitive {
    /// i32 value.
    #[rune(get)]
    pub i32: i32,

    /// i64 value.
    #[rune(get)]
    pub i64: i64,

    // f32 value.
    #[rune(get)]
    pub f32: f32,

    /// f64 value.
    #[rune(get)]
    pub f64: f64,
}

impl RuneDoubleResultPrimitive {
    /// Constructs a new instance of `Self`.
    pub fn new(i32: i32, i64: i64, f32: f32, f64: f64) -> Self {
        Self { i32, i64, f32, f64 }
    }
}
