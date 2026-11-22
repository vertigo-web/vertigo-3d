use glam::Vec3;

use crate::{Pick, Ray};

/// An object dragged with the mouse, a finger or a pen, see
/// [`Object::on_drag`](crate::Object::on_drag): where it was grabbed, and the line from the
/// camera through the pointer now.
///
/// ```rust
/// use vertigo_3d::{Drag, DragPhase, Pick, Ray, math::Vec3};
///
/// // Grabbed on its top, 1 above the floor, and the pointer moved to look at 2, 1, 0.
/// let drag = Drag {
///     phase: DragPhase::Move,
///     grabbed: Pick { point: Vec3::new(0.0, 1.0, 0.0), distance: 5.0 },
///     ray: Ray {
///         origin: Vec3::new(0.0, 5.0, 3.0),
///         direction: Vec3::new(2.0, -4.0, -3.0).normalize(),
///     },
/// };
/// // Along the floor, the point grabbed has moved by 2 to the right.
/// let moved = drag.moved_on(Vec3::Y).unwrap_or_default();
/// assert!(moved.abs_diff_eq(Vec3::new(2.0, 0.0, 0.0), 1e-5));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    pub phase: DragPhase,
    /// The point of the object's surface pressed on, as it was then.
    pub grabbed: Pick,
    /// From the camera through the pointer now.
    pub ray: Ray,
}

/// Which part of a drag a [`Drag`] tells of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragPhase {
    /// The pointer pressed on the object and moved further than a click may, so it's no click.
    Start,
    /// The pointer moved on.
    Move,
    /// The pointer was let go, or the browser took it over, f. ex. to scroll the page.
    End,
}

impl Drag {
    /// How far the pointer has moved the point grabbed, along the plane through it facing
    /// `normal`: f. ex. `Vec3::Y` for along the floor, the point grabbed staying under the
    /// pointer. `None` while the pointer looks away from the plane, f. ex. above the horizon.
    pub fn moved_on(&self, normal: Vec3) -> Option<Vec3> {
        self.ray
            .on_plane(self.grabbed.point, normal)
            .map(|point| point - self.grabbed.point)
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::{Drag, DragPhase};
    use crate::{Pick, Ray};

    fn drag(direction: Vec3) -> Drag {
        Drag {
            phase: DragPhase::Move,
            grabbed: Pick {
                point: Vec3::new(1.0, 1.0, 1.0),
                distance: 4.0,
            },
            ray: Ray {
                origin: Vec3::new(1.0, 5.0, 1.0),
                direction: direction.normalize(),
            },
        }
    }

    #[test]
    fn the_point_grabbed_follows_the_pointer_along_the_plane() {
        let Some(moved) = drag(Vec3::new(1.0, -2.0, 0.5)).moved_on(Vec3::Y) else {
            panic!("the plane seen");
        };
        assert!(moved.abs_diff_eq(Vec3::new(2.0, 0.0, 1.0), 1e-5), "{moved}");
    }

    #[test]
    fn looking_away_from_the_plane_moves_nothing() {
        assert_eq!(drag(Vec3::new(1.0, 1.0, 0.0)).moved_on(Vec3::Y), None);
        assert_eq!(drag(Vec3::X).moved_on(Vec3::Y), None, "along it");
    }
}
