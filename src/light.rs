use glam::Vec3;

use crate::Color;

/// Directional lights a scene is lit by at most: those after are left out.
pub const MAX_DIRECTIONAL: usize = 4;

/// Point lights a scene is lit by at most: those after are left out.
pub const MAX_POINT: usize = 4;

/// How far a point light reaches when not told, see [`Light::range`].
const RANGE: f32 = 10.0;

/// Light falling on the objects of [`Material::lit`](crate::Material::lit): white and as
/// strong as 1 at first, and their sum lights a surface. The directional and point lights also
/// shine on the objects of a material with [`specular`](crate::Material::specular) highlights.
///
/// ```rust
/// use vertigo_3d::{Color, Light, math::Vec3};
///
/// let lights = vec![
///     // So no side is quite dark
///     Light::ambient(0.2),
///     // The sun, low and warm, from the left
///     Light::directional(Vec3::new(1.0, -0.3, 0.0))
///         .color(Color::rgb(1.0, 0.8, 0.6))
///         .intensity(0.9),
///     // A lamp above the table, reaching 6 away
///     Light::point(Vec3::new(0.0, 2.5, 0.0)).range(6.0),
/// ];
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    kind: Kind,
    color: Color,
    intensity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Ambient,
    Directional(Vec3),
    Point { position: Vec3, range: f32 },
}

impl Light {
    /// Light from everywhere alike, as `intensity` of the surface's color.
    pub fn ambient(intensity: f32) -> Self {
        Self {
            kind: Kind::Ambient,
            color: Color::WHITE,
            intensity,
        }
    }

    /// Light from far away shining along `direction`, as the sun does: all of a surface facing
    /// it, the most where it faces it straight, none where it faces away.
    pub fn directional(direction: Vec3) -> Self {
        Self {
            kind: Kind::Directional(direction),
            color: Color::WHITE,
            intensity: 1.0,
        }
    }

    /// Light shining all around from `position`, as a lamp does: on what faces it, the most
    /// near it, less and less farther away, and none past its [`range`](Self::range), 10 at
    /// first.
    pub fn point(position: Vec3) -> Self {
        Self {
            kind: Kind::Point {
                position,
                range: RANGE,
            },
            color: Color::WHITE,
            intensity: 1.0,
        }
    }

    /// How far a point light reaches: its light fades smoothly to none there. Nothing for the
    /// other lights.
    pub fn range(self, range: f32) -> Self {
        match self.kind {
            Kind::Point { position, .. } => Self {
                kind: Kind::Point { position, range },
                ..self
            },
            Kind::Ambient | Kind::Directional(_) => self,
        }
    }

    pub fn color(self, color: Color) -> Self {
        Self { color, ..self }
    }

    pub fn intensity(self, intensity: f32) -> Self {
        Self { intensity, ..self }
    }

    /// The light's color as strong as it is.
    fn strength(&self) -> Vec3 {
        Vec3::new(self.color.red, self.color.green, self.color.blue) * self.intensity
    }

    /// The light a scene has without any given: soft from everywhere, and stronger from above,
    /// front and left.
    pub fn defaults() -> Vec<Light> {
        vec![
            Light::ambient(0.35),
            Light::directional(Vec3::new(0.5, -1.0, -0.7)).intensity(0.75),
        ]
    }
}

/// What the shader takes of lights.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Lighting {
    /// The ambient lights summed.
    pub ambient: Vec3,
    /// Each directional light's direction, normalized, with its strength.
    pub directional: Vec<(Vec3, Vec3)>,
    /// Each point light's position, range and strength.
    pub point: Vec<(Vec3, f32, Vec3)>,
}

/// What the shader takes of `lights`. A directional light without a direction, and a point
/// light reaching nowhere, light nothing, and are left out.
pub(crate) fn lighting(lights: &[Light]) -> Lighting {
    let mut lighting = Lighting::default();

    for light in lights {
        match light.kind {
            Kind::Ambient => lighting.ambient += light.strength(),
            Kind::Directional(direction) => {
                let direction = direction.normalize_or_zero();
                if direction != Vec3::ZERO && lighting.directional.len() < MAX_DIRECTIONAL {
                    lighting.directional.push((direction, light.strength()));
                }
            }
            Kind::Point { position, range } => {
                if range > 0.0 && range.is_finite() && lighting.point.len() < MAX_POINT {
                    lighting.point.push((position, range, light.strength()));
                }
            }
        }
    }

    lighting
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::{Light, Lighting, MAX_DIRECTIONAL, MAX_POINT, lighting};
    use crate::Color;

    #[test]
    fn ambient_lights_sum_and_directional_ones_point_the_way_they_shine() {
        let Lighting {
            ambient,
            directional,
            point,
        } = lighting(&[
            Light::ambient(0.25),
            Light::ambient(0.5).color(Color::rgb(1.0, 0.0, 0.0)),
            Light::directional(Vec3::new(0.0, -4.0, 0.0)).intensity(0.5),
        ]);

        assert!(
            ambient.abs_diff_eq(Vec3::new(0.75, 0.25, 0.25), 1e-6),
            "{ambient}"
        );
        assert_eq!(directional, vec![(Vec3::NEG_Y, Vec3::splat(0.5))]);
        assert!(point.is_empty());
    }

    #[test]
    fn directional_lights_without_a_way_or_too_many_are_left_out() {
        let lights: Vec<Light> = std::iter::once(Light::directional(Vec3::ZERO))
            .chain((0..10).map(|_| Light::directional(Vec3::X)))
            .collect();

        let lighting = lighting(&lights);
        assert_eq!(lighting.ambient, Vec3::ZERO);
        assert_eq!(lighting.directional.len(), MAX_DIRECTIONAL);
    }

    #[test]
    fn point_lights_shine_from_where_they_are_as_far_as_they_reach() {
        let lamp = Light::point(Vec3::new(1.0, 2.0, 3.0));
        let lights = [
            lamp.range(4.0).intensity(0.5),
            lamp,
            // Reaching nowhere
            lamp.range(0.0),
            lamp.range(f32::INFINITY),
            // A range is only of a point light
            Light::ambient(0.1).range(2.0),
        ];

        let lighting = lighting(&lights);
        assert_eq!(
            lighting.point,
            vec![
                (Vec3::new(1.0, 2.0, 3.0), 4.0, Vec3::splat(0.5)),
                (Vec3::new(1.0, 2.0, 3.0), 10.0, Vec3::ONE)
            ]
        );
        assert_eq!(lighting.ambient, Vec3::splat(0.1));

        let many = vec![lamp; MAX_POINT + 3];
        assert_eq!(super::lighting(&many).point.len(), MAX_POINT);
    }
}
