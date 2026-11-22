/// A color, each of its parts from 0 to 1.
///
/// ```rust
/// use vertigo_3d::Color;
///
/// let sky = Color::rgb(0.53, 0.81, 0.92);
/// assert_eq!(sky.alpha, 1.0);
/// assert_eq!(Color::default(), Color::BLACK);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    /// How much it covers what is behind it: 1 for all of it.
    pub alpha: f32,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);

    /// An opaque color.
    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self::rgba(red, green, blue, 1.0)
    }

    pub const fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}
