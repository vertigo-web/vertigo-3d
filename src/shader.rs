use std::{fmt, rc::Rc};

/// Code of its own in the shaders drawing the objects of a material: GLSL ES 3.00 statements,
/// run for each corner of their meshes before it's placed, and for each pixel after it's lit -
/// f. ex. waves running over a surface, or its colors changing with time. Given to a material
/// with [`Material::shader`](crate::Material::shader). The objects keep their material's colors,
/// textures and light, and are drawn together as any others.
///
/// The code of corners, [`Shader::vertex`], may change, in the object's own space:
/// - `vec3 position` - where the corner is,
/// - `vec3 normal` - the way its surface faces,
/// - `vec2 uv` - its texture coordinates.
///
/// The code of pixels, [`Shader::fragment`], may change `vec4 pixel`, its color, lit and as
/// see-through as it is, and reads:
/// - `vec3 world` - where the pixel is in the scene,
/// - `vec2 texel` - the texture coordinates there, repeated.
///
/// Both read `float time`, the time of the scene, see
/// [`SceneParams::time`](crate::SceneParams::time). Code that doesn't compile is told in the
/// browser's console, and its objects aren't drawn.
///
/// A scene drawn with WebGPU (see [`Backend`](crate::Backend)) takes the code in
/// [WGSL](https://www.w3.org/TR/WGSL/) instead, [`Shader::vertex_wgsl`] and
/// [`Shader::fragment_wgsl`], with the same names: `position: vec3f`, `normal: vec3f`,
/// `uv: vec2f`, `pixel: vec4f`, `world: vec3f`, `texel: vec2f` and `time: f32`. A scene with a
/// shader of GLSL alone is drawn with WebGL, and one of WGSL alone draws its objects without the
/// code where the browser has no WebGPU. Code of WGSL that doesn't compile leaves the whole scene
/// undrawn, as WebGPU leaves out all the calls of a frame then.
///
/// ```rust
/// use vertigo_3d::{Color, Material, Shader};
///
/// let jelly = Material::lit(Color::rgb(0.3, 0.8, 0.4)).shader(
///     Shader::new()
///         // Ripples running up the surface
///         .vertex("position += normal * 0.05 * sin(position.y * 12.0 + time * 4.0);")
///         .vertex_wgsl("position += normal * 0.05 * sin(position.y * 12.0 + time * 4.0);")
///         // Pulsing lighter and darker, a second a pulse
///         .fragment("pixel.rgb *= 0.8 + 0.2 * sin(time * 6.2832);")
///         .fragment_wgsl("pixel = vec4f(pixel.rgb * (0.8 + 0.2 * sin(time * 6.2832)), pixel.a);"),
/// );
/// ```
#[derive(Clone, Default)]
pub struct Shader {
    code: Rc<Code>,
}

#[derive(Clone, Default)]
struct Code {
    vertex: String,
    fragment: String,
    vertex_wgsl: String,
    fragment_wgsl: String,
}

impl Shader {
    /// Code changing nothing, until given some.
    pub fn new() -> Self {
        Self::default()
    }

    /// Statements run for each corner, before it's placed, see [`Shader`].
    pub fn vertex(self, code: impl Into<String>) -> Self {
        let mut code_now = Rc::unwrap_or_clone(self.code);
        code_now.vertex = code.into();
        Self {
            code: Rc::new(code_now),
        }
    }

    /// Statements run for each pixel, after it's lit, see [`Shader`].
    pub fn fragment(self, code: impl Into<String>) -> Self {
        let mut code_now = Rc::unwrap_or_clone(self.code);
        code_now.fragment = code.into();
        Self {
            code: Rc::new(code_now),
        }
    }

    /// Statements of WGSL run for each corner, before it's placed, for WebGPU, see [`Shader`].
    pub fn vertex_wgsl(self, code: impl Into<String>) -> Self {
        let mut code_now = Rc::unwrap_or_clone(self.code);
        code_now.vertex_wgsl = code.into();
        Self {
            code: Rc::new(code_now),
        }
    }

    /// Statements of WGSL run for each pixel, after it's lit, for WebGPU, see [`Shader`].
    pub fn fragment_wgsl(self, code: impl Into<String>) -> Self {
        let mut code_now = Rc::unwrap_or_clone(self.code);
        code_now.fragment_wgsl = code.into();
        Self {
            code: Rc::new(code_now),
        }
    }

    pub(crate) fn vertex_code(&self) -> &str {
        &self.code.vertex
    }

    pub(crate) fn fragment_code(&self) -> &str {
        &self.code.fragment
    }

    pub(crate) fn vertex_wgsl_code(&self) -> &str {
        &self.code.vertex_wgsl
    }

    pub(crate) fn fragment_wgsl_code(&self) -> &str {
        &self.code.fragment_wgsl
    }

    /// Whether WebGPU draws it as WebGL does: it has the code of WGSL for each of GLSL.
    pub(crate) fn has_wgsl(&self) -> bool {
        let Code {
            vertex,
            fragment,
            vertex_wgsl,
            fragment_wgsl,
        } = &*self.code;
        (vertex.is_empty() || !vertex_wgsl.is_empty())
            && (fragment.is_empty() || !fragment_wgsl.is_empty())
    }
}

impl PartialEq for Shader {
    /// The same shader, not one alike: objects of one are drawn together, and it's compiled once.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.code, &other.code)
    }
}

impl fmt::Debug for Shader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shader")
            .field("vertex", &self.code.vertex)
            .field("fragment", &self.code.fragment)
            .field("vertex_wgsl", &self.code.vertex_wgsl)
            .field("fragment_wgsl", &self.code.fragment_wgsl)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::Shader;

    #[test]
    fn a_shader_is_itself_and_its_clones_and_keeps_its_code() {
        let shader = Shader::new().vertex("position.y += 1.0;");
        assert_eq!(shader, shader.clone());
        assert_ne!(shader, Shader::new().vertex("position.y += 1.0;"));

        let both = shader.fragment("pixel.r = 1.0;");
        assert_eq!(both.vertex_code(), "position.y += 1.0;");
        assert_eq!(both.fragment_code(), "pixel.r = 1.0;");
    }

    #[test]
    fn webgpu_draws_a_shader_with_wgsl_for_each_part_of_glsl() {
        assert!(Shader::new().has_wgsl(), "no code at all");
        let glsl = Shader::new().vertex("position.y += 1.0;");
        assert!(!glsl.has_wgsl());
        let both = glsl.vertex_wgsl("position.y += 1.0;");
        assert!(both.has_wgsl());
        assert_eq!(both.vertex_wgsl_code(), "position.y += 1.0;");
        assert!(
            !both.fragment("pixel.r = 1.0;").has_wgsl(),
            "a part without"
        );
        assert!(Shader::new().fragment_wgsl("pixel.r = 1.0;").has_wgsl());
    }
}
