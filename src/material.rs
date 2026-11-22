use glam::Vec2;

use crate::{Color, Shader, Texture};

/// The shininess of highlights when not told, see [`Material::specular`].
const SHININESS: f32 = 32.0;

/// What an object's surface looks like: its color, lit or not, maybe with a [`Texture`] on it,
/// and whether what is behind shows through it.
///
/// An object shows through where its color's alpha is below 1, or the texture's, with
/// [`Material::see_through`]. Such objects are drawn after the others, the farthest first, each
/// blended with what is behind it.
///
/// ```rust
/// use vertigo_3d::{Color, Material, Texture};
///
/// // The same however the light falls, f. ex. a lamp, or a marker.
/// let red = Material::color(Color::rgb(0.9, 0.2, 0.2));
/// // Lighter where the light falls on it straight, darker where it doesn't.
/// let clay = Material::lit(Color::rgb(0.8, 0.5, 0.3));
/// // Bricks, lit, ten across and five down a long wall.
/// let wall = Material::lit(Color::WHITE)
///     .texture(Texture::url("/images/bricks.png"))
///     .repeat(10.0, 5.0);
/// // Blue glass, half of what is behind showing through.
/// let glass = Material::lit(Color::rgba(0.3, 0.5, 1.0, 0.5));
/// // Polished red plastic: small white highlights where the light shines toward the camera.
/// let plastic = Material::lit(Color::rgb(0.8, 0.1, 0.1)).specular(Color::WHITE, 60.0);
/// // Bricks with joints the light shows, which look deep seen at a slant.
/// let bricks = Material::lit(Color::WHITE)
///     .texture(Texture::url("/images/bricks.png"))
///     .normal_map(Texture::url("/images/bricks_normal.png"))
///     .height_map(Texture::url("/images/bricks_height.png"), 0.04);
/// assert_eq!(Material::default(), Material::color(Color::WHITE));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    kind: Kind,
    surface: Surface,
    /// Of the height map, see [`Material::height_map`].
    depth: f32,
    repeat: Vec2,
    see_through: bool,
    specular: Color,
    shininess: f32,
}

/// The textures of a surface and its shader: objects of the same ones are drawn together.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Surface {
    pub color: Option<Texture>,
    pub normal: Option<Texture>,
    pub height: Option<Texture>,
    pub shader: Option<Shader>,
}

impl Surface {
    pub fn textures(&self) -> impl Iterator<Item = &Texture> {
        [&self.color, &self.normal, &self.height]
            .into_iter()
            .flatten()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Color(Color),
    Lit(Color),
}

impl Material {
    /// One color all over, the same however the light falls.
    pub const fn color(color: Color) -> Self {
        Self::of(Kind::Color(color))
    }

    /// `color` as the scene's light makes it, see [`Light`](crate::Light): as bright as the
    /// light falling on it.
    pub const fn lit(color: Color) -> Self {
        Self::of(Kind::Lit(color))
    }

    const fn of(kind: Kind) -> Self {
        Self {
            kind,
            surface: Surface {
                color: None,
                normal: None,
                height: None,
                shader: None,
            },
            depth: 0.0,
            repeat: Vec2::ONE,
            see_through: false,
            specular: Color::BLACK,
            shininess: SHININESS,
        }
    }

    /// `texture` on the surface, by the texture coordinates of the mesh, its colors times the
    /// material's: [`Color::WHITE`] shows it as it is. Where the texture is less than half
    /// opaque, the surface is left out, f. ex. between the leaves of a tree, unless it's
    /// [`see_through`](Self::see_through).
    pub fn texture(self, texture: Texture) -> Self {
        Self {
            surface: Surface {
                color: Some(texture),
                ..self.surface
            },
            ..self
        }
    }

    /// The way the surface faces at each point, from the colors of `texture`, a normal map: its
    /// red toward the texture's right, green toward its top and blue out of the surface, each from
    /// -1 at 0 to 1 at full, as OpenGL reads them, so a flat surface is (0.5, 0.5, 1). The light
    /// then shows a relief the mesh hasn't, f. ex. the joints between bricks. It lies on the mesh
    /// as the texture does, repeated with it. Only of [`Material::lit`].
    pub fn normal_map(self, texture: Texture) -> Self {
        Self {
            surface: Surface {
                normal: Some(texture),
                ..self.surface
            },
            ..self
        }
    }

    /// How high each point of the surface is, from the red of `texture`, a height map: white the
    /// highest, black the deepest, `depth` apart, as a share of the texture's width - f. ex.
    /// 0.05, a twentieth. The texture and the normal map are seen shifted the more the more
    /// slanting the surface is seen, as a relief would show them (parallax mapping), while the
    /// mesh's outline stays flat. It lies on the mesh as the texture does, repeated with it.
    pub fn height_map(self, texture: Texture, depth: f32) -> Self {
        Self {
            surface: Surface {
                height: Some(texture),
                ..self.surface
            },
            depth: depth.max(0.0),
            ..self
        }
    }

    /// `shader`'s code of its own in the shaders drawing the surface, see [`Shader`].
    pub fn shader(self, shader: Shader) -> Self {
        Self {
            surface: Surface {
                shader: Some(shader),
                ..self.surface
            },
            ..self
        }
    }

    /// The texture repeated `across` times across the surface and `down` times down it, f. ex.
    /// tiles on a floor. Once at first.
    pub fn repeat(self, across: f32, down: f32) -> Self {
        Self {
            repeat: Vec2::new(across, down),
            ..self
        }
    }

    /// What is behind shows through it, as much as the texture is transparent, f. ex. a window
    /// with a frame. Without it only a color's alpha below 1 makes an object see-through.
    pub fn see_through(self) -> Self {
        Self {
            see_through: true,
            ..self
        }
    }

    /// Highlights of `color` where the directional and point lights shine off the surface toward
    /// the camera, as off a mirror: small and sharp of a high `shininess`, f. ex. 100 for polished
    /// metal, broad and dim of a low one, f. ex. 10 for stone. None at first. Only of
    /// [`Material::lit`]: one color all over has no highlights.
    pub fn specular(self, color: Color, shininess: f32) -> Self {
        Self {
            specular: color,
            shininess: shininess.max(1.0),
            ..self
        }
    }

    /// The color the surface has before light falls on it.
    pub(crate) fn base_color(&self) -> Color {
        match self.kind {
            Kind::Color(color) | Kind::Lit(color) => color,
        }
    }

    pub(crate) fn is_lit(&self) -> bool {
        matches!(self.kind, Kind::Lit(_))
    }

    pub(crate) fn surface(&self) -> &Surface {
        &self.surface
    }

    /// The depth of its height map, 0 without one.
    pub(crate) fn depth(&self) -> f32 {
        if self.surface.height.is_some() {
            self.depth
        } else {
            0.0
        }
    }

    pub(crate) fn repeated(&self) -> Vec2 {
        self.repeat
    }

    /// The color of its highlights, black for none, and their shininess.
    pub(crate) fn highlights(&self) -> (Color, f32) {
        (self.specular, self.shininess)
    }

    /// Whether it's drawn blended with what is behind it.
    pub(crate) fn is_see_through(&self) -> bool {
        self.see_through || self.base_color().alpha < 1.0
    }
}

impl Default for Material {
    /// White.
    fn default() -> Self {
        Self::color(Color::WHITE)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Color, Material, Texture};

    #[test]
    fn see_through_by_the_alpha_of_its_color_or_when_told() {
        assert!(!Material::lit(Color::WHITE).is_see_through());
        assert!(Material::lit(Color::rgba(1.0, 1.0, 1.0, 0.5)).is_see_through());
        let window = Material::color(Color::WHITE).texture(Texture::url("/window.png"));
        assert!(!window.is_see_through());
        assert!(window.see_through().is_see_through());
    }
}
