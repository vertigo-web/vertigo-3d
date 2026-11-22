use std::{fmt, rc::Rc};

/// An image on the surface of objects, see [`Material::texture`](crate::Material::texture): from
/// a URL, which the browser loads, or from pixels made in Rust. Shared by the objects showing it,
/// and sent to the browser once for a scene.
///
/// Its corners are where a mesh's texture coordinates are 0 and 1, the top left at 0, 0. It is
/// read smoothly between its pixels and repeats past its edges, see
/// [`Material::repeat`](crate::Material::repeat).
///
/// ```rust
/// use vertigo_3d::Texture;
///
/// // The objects showing it have their base color until the browser has it. In an app, an image
/// // of its own is best given by `vertigo::include_static!`.
/// let bricks = Texture::url("/images/bricks.png");
///
/// // Two by two pixels, red and white, sharp: a checkerboard.
/// let checker = Texture::pixels(2, 2, vec![
///     255, 0, 0, 255,  255, 255, 255, 255,
///     255, 255, 255, 255,  255, 0, 0, 255,
/// ]);
/// assert!(checker.is_ok());
/// ```
#[derive(Clone)]
pub struct Texture {
    data: Rc<TextureData>,
}

#[derive(Clone)]
pub(crate) struct TextureData {
    pub image: Image,
    /// Read from the nearest pixel, not between them.
    pub sharp: bool,
}

#[derive(Clone)]
pub(crate) enum Image {
    Url(String),
    /// Rows of red, green, blue and alpha, from the top.
    Pixels {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
}

/// Why [`Texture::pixels`] didn't make a texture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextureError {
    /// Not 4 bytes for each pixel of the size.
    Pixels {
        width: u32,
        height: u32,
        bytes: usize,
    },
}

impl fmt::Display for TextureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pixels {
                width,
                height,
                bytes,
            } => write!(f, "{bytes} bytes aren't {width} x {height} pixels of 4"),
        }
    }
}

impl std::error::Error for TextureError {}

impl Texture {
    /// The image at `url`, which the browser loads when a scene first shows it: until then the
    /// objects showing it have their base color. An image from another origin needs CORS headers
    /// allowing it.
    pub fn url(url: impl Into<String>) -> Self {
        Self::of(Image::Url(url.into()))
    }

    /// `width` by `height` pixels, 4 bytes each - red, green, blue and alpha - in rows from the
    /// top.
    pub fn pixels(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, TextureError> {
        if width == 0 || height == 0 || pixels.len() != width as usize * height as usize * 4 {
            return Err(TextureError::Pixels {
                width,
                height,
                bytes: pixels.len(),
            });
        }
        Ok(Self::of(Image::Pixels {
            width,
            height,
            pixels,
        }))
    }

    /// Read from the nearest pixel rather than between them: sharp when it is shown bigger than
    /// it is, f. ex. for pixel art.
    pub fn sharp(self) -> Self {
        let mut data = Rc::unwrap_or_clone(self.data);
        data.sharp = true;
        Self {
            data: Rc::new(data),
        }
    }

    fn of(image: Image) -> Self {
        Self {
            data: Rc::new(TextureData {
                image,
                sharp: false,
            }),
        }
    }

    pub(crate) fn data(&self) -> &TextureData {
        &self.data
    }
}

/// How far the images of a scene's textures from URLs are loaded, told by
/// [`SceneParams::on_load`](crate::SceneParams::on_load): of its objects and preloaded. Those of
/// pixels are in at once, and not counted.
///
/// ```rust
/// use vertigo_3d::Loading;
///
/// let loading = Loading { loaded: 40, failed: 2, all: 42 };
/// // Nothing is coming any more, though two didn't come.
/// assert!(loading.is_done());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Loading {
    /// Images in their textures.
    pub loaded: usize,
    /// Images the browser couldn't load or decode: their textures stay white.
    pub failed: usize,
    /// The textures from URLs, loaded or not.
    pub all: usize,
}

impl Loading {
    /// Whether no image is coming any more: each came or failed.
    pub fn is_done(&self) -> bool {
        self.loaded + self.failed >= self.all
    }
}

impl PartialEq for Texture {
    /// The same texture, not one alike: objects of one are drawn together.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.data, &other.data)
    }
}

impl fmt::Debug for Texture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("Texture");
        match &self.data.image {
            Image::Url(url) => debug.field("url", url),
            Image::Pixels { width, height, .. } => {
                debug.field("width", width).field("height", height)
            }
        };
        debug.field("sharp", &self.data.sharp).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{Texture, TextureError};

    #[test]
    fn pixels_are_four_bytes_each() {
        assert!(Texture::pixels(1, 2, vec![0; 8]).is_ok());
        assert_eq!(
            Texture::pixels(2, 2, vec![0; 8]).err(),
            Some(TextureError::Pixels {
                width: 2,
                height: 2,
                bytes: 8
            })
        );
        assert!(Texture::pixels(0, 0, Vec::new()).is_err(), "no pixels");
    }

    #[test]
    fn a_texture_is_itself_and_its_clones() {
        let texture = Texture::url("/a.png");
        assert_eq!(texture, texture.clone());
        assert_ne!(texture, Texture::url("/a.png"));
    }
}
