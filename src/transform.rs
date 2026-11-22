use glam::{Mat4, Quat, Vec3};

/// Where an object is, how it's turned, and how big it is: scaled, then turned, then moved.
///
/// ```rust
/// use vertigo_3d::{Transform, math::{Quat, Vec3}};
///
/// let lying = Transform::from_translation(Vec3::new(0.0, 1.0, 0.0))
///     .with_rotation(Quat::from_rotation_x(90_f32.to_radians()))
///     .with_scale(Vec3::splat(2.0));
/// assert_eq!(Transform::default(), Transform::IDENTITY);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    /// Where the mesh is made: at the origin, not turned, as big as it is.
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn from_translation(translation: Vec3) -> Self {
        Self::IDENTITY.with_translation(translation)
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        Self::IDENTITY.with_rotation(rotation)
    }

    pub fn from_scale(scale: Vec3) -> Self {
        Self::IDENTITY.with_scale(scale)
    }

    pub fn with_translation(self, translation: Vec3) -> Self {
        Self {
            translation,
            ..self
        }
    }

    pub fn with_rotation(self, rotation: Quat) -> Self {
        Self { rotation, ..self }
    }

    pub fn with_scale(self, scale: Vec3) -> Self {
        Self { scale, ..self }
    }

    /// The matrix taking the mesh's points to the scene's.
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use glam::{Quat, Vec3};

    use super::Transform;

    #[test]
    fn a_point_is_scaled_then_turned_then_moved() {
        let transform = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_z(90_f32.to_radians()))
            .with_scale(Vec3::splat(2.0));

        let point = transform.to_matrix().transform_point3(Vec3::X);
        assert!(
            point.abs_diff_eq(Vec3::new(10.0, 2.0, 0.0), 1e-5),
            "{point}"
        );
    }
}
