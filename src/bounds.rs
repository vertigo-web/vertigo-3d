use glam::Vec3;

use crate::Transform;

/// A box with its sides along the axes, from the least x, y and z of what is in it to the
/// greatest: around a mesh ([`Mesh::bounds`](crate::Mesh::bounds)), or around an object, the
/// mesh's box moved by the object's transform ([`Bounds::transformed`]). F. ex. to tell cheaply
/// whether two objects may touch, or how big a shape a physics engine gives an object.
///
/// ```rust
/// use vertigo_3d::{Mesh, Transform, math::Vec3};
///
/// let cube = Mesh::cube();
/// let at = |x: f32| cube.bounds().transformed(&Transform::from_translation(Vec3::new(x, 0.5, 0.0)));
/// assert!(at(0.0).overlaps(&at(0.8)));
/// assert!(!at(0.0).overlaps(&at(3.0)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    /// Around `points`, or a point at the origin around none.
    pub(crate) fn around(points: impl IntoIterator<Item = Vec3>) -> Self {
        let (min, max) = points.into_iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(min, max), point| (min.min(point), max.max(point)),
        );
        if min.x > max.x {
            return Self {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            };
        }
        Self { min, max }
    }

    /// Its middle.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) / 2.0
    }

    /// How long it is along X, Y and Z.
    ///
    /// ```rust
    /// use vertigo_3d::{Mesh, math::Vec3};
    ///
    /// assert_eq!(Mesh::cube().bounds().size(), Vec3::ONE);
    /// assert_eq!(Mesh::plane().bounds().size(), Vec3::new(1.0, 0.0, 1.0));
    /// ```
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// The box around this one scaled, turned and moved by `transform`, as an object is: as big
    /// as this one scaled, or bigger where it's turned.
    ///
    /// ```rust
    /// use vertigo_3d::{Mesh, Transform, math::{Quat, Vec3}};
    ///
    /// let lying = Transform::from_translation(Vec3::new(0.0, 0.25, 0.0))
    ///     .with_scale(Vec3::new(2.0, 0.5, 1.0));
    /// let bounds = Mesh::cube().bounds().transformed(&lying);
    /// assert_eq!(bounds.min, Vec3::new(-1.0, 0.0, -0.5));
    /// assert_eq!(bounds.max, Vec3::new(1.0, 0.5, 0.5));
    /// ```
    pub fn transformed(&self, transform: &Transform) -> Self {
        let model = transform.to_matrix();
        let center = model.transform_point3(self.center());
        let half = self.size() / 2.0;
        // How far the box reaches from its middle along each axis, its sides turned.
        let reach = model.x_axis.truncate().abs() * half.x
            + model.y_axis.truncate().abs() * half.y
            + model.z_axis.truncate().abs() * half.z;
        Self {
            min: center - reach,
            max: center + reach,
        }
    }

    /// Whether it and `other` have a point in common, a side's as well.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.min.cmple(other.max).all() && other.min.cmple(self.max).all()
    }
}

#[cfg(test)]
mod tests {
    use glam::{Quat, Vec3};

    use super::Bounds;
    use crate::Transform;

    #[test]
    fn the_box_holds_the_points() {
        let bounds = Bounds::around([
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 3.0),
        ]);
        assert_eq!(bounds.min, Vec3::new(0.0, -1.0, 0.0));
        assert_eq!(bounds.max, Vec3::new(2.0, 1.0, 3.0));
        assert_eq!(bounds.center(), Vec3::new(1.0, 0.0, 1.5));
        assert_eq!(bounds.size(), Vec3::new(2.0, 2.0, 3.0));

        let none = Bounds::around([]);
        assert_eq!((none.min, none.max), (Vec3::ZERO, Vec3::ZERO));
    }

    #[test]
    fn a_turned_box_is_held_by_a_bigger_one() {
        let cube = Bounds {
            min: Vec3::splat(-0.5),
            max: Vec3::splat(0.5),
        };
        let turned = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_y(45_f32.to_radians()))
            .with_scale(Vec3::splat(2.0));
        let bounds = cube.transformed(&turned);
        let reach = 2.0_f32.sqrt();
        assert!(
            bounds
                .min
                .abs_diff_eq(Vec3::new(10.0 - reach, -1.0, -reach), 1e-5),
            "{bounds:?}"
        );
        assert!(
            bounds
                .max
                .abs_diff_eq(Vec3::new(10.0 + reach, 1.0, reach), 1e-5),
            "{bounds:?}"
        );
    }

    #[test]
    fn boxes_overlap_when_they_share_a_point() {
        let unit = |x: f32, y: f32| Bounds {
            min: Vec3::new(x, y, 0.0),
            max: Vec3::new(x + 1.0, y + 1.0, 1.0),
        };
        assert!(unit(0.0, 0.0).overlaps(&unit(0.5, 0.5)));
        assert!(unit(0.0, 0.0).overlaps(&unit(1.0, 0.0)), "side by side");
        assert!(!unit(0.0, 0.0).overlaps(&unit(1.5, 0.0)));
        assert!(
            !unit(0.0, 0.0).overlaps(&unit(0.5, 2.0)),
            "apart along one axis"
        );
    }
}
