/// Color-related utilities.
pub struct ColorUtils;

impl ColorUtils {
    /// Converts RGBA to Float RGBA.
    pub const fn rgba_to_frgba(rgba: [u8; 4]) -> [f32; 4] {
        const DIV: f32 = 255.0;
        let r = rgba[0] as f32 / DIV;
        let g = rgba[1] as f32 / DIV;
        let b = rgba[2] as f32 / DIV;
        let a = rgba[3] as f32 / DIV;
        [r, g, b, a]
    }
}
