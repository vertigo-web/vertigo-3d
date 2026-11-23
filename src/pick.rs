use glam::{Vec2, Vec3};

use crate::{Camera, Mesh, Transform};

/// Where a [`Ray`] meets a surface: the point, in the scene, and how far along the ray it is.
/// For a click, the point of the object the pointer was over, and how far it is from the camera.
///
/// ```rust
/// use vertigo::Value;
/// use vertigo_3d::{Mesh, Object, Pick};
///
/// let clicked = Value::new(None);
/// let cube = Object::new(Mesh::cube()).on_click({
///     let clicked = clicked.clone();
///     move |pick: Pick| clicked.set(Some(pick.point))
/// });
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pick {
    pub point: Vec3,
    pub distance: f32,
}

/// A line going one way into the scene from its origin. The pointer's - from the camera through
/// the pointer, where it looks - finds what's clicked and dragged, and one of one's own what
/// else it meets, as clicks find it: f. ex. what an object stands on, or whether it sees another.
///
/// ```rust
/// use vertigo_3d::{Ray, math::Vec3};
///
/// let down = Ray::new(Vec3::new(1.0, 5.0, 2.0), Vec3::NEG_Y);
/// // Where it meets the floor
/// assert_eq!(down.on_plane(Vec3::ZERO, Vec3::Y), Some(Vec3::new(1.0, 0.0, 2.0)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    /// Where it starts: the pointer's on the camera's near plane.
    pub origin: Vec3,
    /// Of length 1, see [`Ray::new`].
    pub direction: Vec3,
}

impl Ray {
    /// From `origin` the way of `direction`, of any length but 0: made of length 1. A ray of
    /// direction 0 meets nothing.
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize_or_zero(),
        }
    }

    /// Where it meets the plane through `point` facing `normal`, ahead of its origin. `None`
    /// when it goes along the plane or away from it.
    pub fn on_plane(&self, point: Vec3, normal: Vec3) -> Option<Vec3> {
        let towards = self.direction.dot(normal);
        if towards.abs() < 1e-6 {
            return None;
        }
        let along = (point - self.origin).dot(normal) / towards;
        (along >= 0.0).then(|| self.origin + self.direction * along)
    }

    /// Where it first meets a triangle of `mesh`, moved by `transform` as an object is, ahead of
    /// its origin, from either side: from inside the mesh, where it goes out. `None` when it
    /// misses it.
    ///
    /// ```rust
    /// use vertigo_3d::{Mesh, Ray, Transform, math::Vec3};
    ///
    /// let crate_ = Transform::from_translation(Vec3::new(2.0, 0.5, 0.0));
    /// let down = Ray::new(Vec3::new(2.0, 10.0, 0.0), Vec3::NEG_Y);
    /// // On the crate's top
    /// let pick = down.on_mesh(&Mesh::cube(), &crate_);
    /// assert_eq!(pick.map(|pick| pick.point), Some(Vec3::new(2.0, 1.0, 0.0)));
    /// assert_eq!(pick.map(|pick| pick.distance), Some(9.0));
    /// ```
    pub fn on_mesh(&self, mesh: &Mesh, transform: &Transform) -> Option<Pick> {
        self.distance_to(mesh, transform)
            .map(|distance| self.pick(distance))
    }

    /// Of `meshes`, each moved by its transform, the one it meets first, by its place among
    /// them, with where, see [`Ray::on_mesh`]: as a click finds the object nearest the camera.
    ///
    /// ```rust
    /// use vertigo_3d::{Mesh, Ray, Transform, math::Vec3};
    ///
    /// let (cube, ball) = (Mesh::cube(), Mesh::sphere(16));
    /// let low = Transform::from_translation(Vec3::new(0.0, 0.5, 0.0));
    /// let high = Transform::from_translation(Vec3::new(0.0, 3.0, 0.0));
    /// let down = Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y);
    ///
    /// // The ball above the cube, its top at 3.5
    /// let found = down.nearest([(&cube, &low), (&ball, &high)]);
    /// assert_eq!(found.map(|(index, _)| index), Some(1));
    /// assert!(found.is_some_and(|(_, pick)| (pick.distance - 6.5).abs() < 1e-4));
    /// ```
    pub fn nearest<'a>(
        &self,
        meshes: impl IntoIterator<Item = (&'a Mesh, &'a Transform)>,
    ) -> Option<(usize, Pick)> {
        meshes
            .into_iter()
            .enumerate()
            .filter_map(|(index, (mesh, transform))| {
                self.distance_to(mesh, transform)
                    .map(|distance| (index, distance))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, distance)| (index, self.pick(distance)))
    }

    /// Through `at`, from -1 to 1 across the canvas and from -1 at its bottom to 1 at its top,
    /// as `camera` sees it on a canvas of `aspect`.
    pub(crate) fn through(camera: &Camera, aspect: f32, at: Vec2) -> Option<Self> {
        let inverse = camera.view_projection(aspect).inverse();
        let near = inverse.project_point3(at.extend(-1.0));
        let far = inverse.project_point3(at.extend(1.0));
        let direction = (far - near).normalize_or_zero();
        (direction != Vec3::ZERO && near.is_finite()).then_some(Self {
            origin: near,
            direction,
        })
    }

    /// The point `distance` along it.
    fn pick(&self, distance: f32) -> Pick {
        Pick {
            point: self.origin + self.direction * distance,
            distance,
        }
    }

    /// Where along it, from its origin, it meets the mesh moved by `transform`, if it does.
    fn distance_to(&self, mesh: &Mesh, transform: &Transform) -> Option<f32> {
        let model = transform.to_matrix();
        let data = mesh.data();

        // First the sphere around the mesh, which leaves out most of what the ray misses.
        let scale = transform.scale.abs().max_element();
        let center = model.transform_point3(data.bounds.center());
        if !self.near_sphere(center, data.radius * scale) {
            return None;
        }

        // Then each triangle, in the mesh's own space. The direction isn't of length 1 there,
        // but a distance along it is the same as along the ray. A mesh squashed flat has no such
        // space, and is seen edge on at most.
        if model.determinant() == 0.0 {
            return None;
        }
        let inverse = model.inverse();
        if !inverse.is_finite() {
            return None;
        }
        let origin = inverse.transform_point3(self.origin);
        let direction = inverse.transform_vector3(self.direction);
        let corner = |index: u32| Vec3::from_slice(&data.positions[index as usize * 3..][..3]);

        data.indices
            .as_chunks()
            .0
            .iter()
            .filter_map(|&[a, b, c]| {
                triangle_hit(origin, direction, [corner(a), corner(b), corner(c)])
            })
            .min_by(f32::total_cmp)
    }

    /// Whether the ray, ahead of its origin, comes within `radius` of `center`.
    fn near_sphere(&self, center: Vec3, radius: f32) -> bool {
        let to_center = center - self.origin;
        let along = to_center.dot(self.direction);
        let apart = to_center.length_squared() - along * along;
        apart <= radius * radius && (along >= 0.0 || to_center.length() <= radius)
    }
}

/// Where along `direction` from `origin` the line meets the triangle, ahead of it (Möller and
/// Trumbore).
fn triangle_hit(origin: Vec3, direction: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f32> {
    let (ab, ac) = (b - a, c - a);
    // A triangle of no area, its corners in a line, has no plane to meet: f. ex. those at a
    // pole of a ball, its corners there apart by a rounding only. The angle at `a` tells.
    if ab.cross(ac).length_squared() <= 1e-12 * ab.length_squared() * ac.length_squared() {
        return None;
    }
    let cross = direction.cross(ac);
    let determinant = ab.dot(cross);
    if determinant.abs() < 1e-9 {
        return None;
    }

    // A line through a corner or an edge the triangle shares with others, f. ex. down onto the
    // top of a ball, meets them all there, not none of them, whichever way it's rounded.
    const EDGE: f32 = 1e-5;
    let inverse = 1.0 / determinant;
    let from_a = origin - a;
    let u = from_a.dot(cross) * inverse;
    if !(-EDGE..=1.0 + EDGE).contains(&u) {
        return None;
    }
    let crossed = from_a.cross(ab);
    let v = direction.dot(crossed) * inverse;
    if v < -EDGE || u + v > 1.0 + EDGE {
        return None;
    }
    let along = ac.dot(crossed) * inverse;
    (along > 0.0).then_some(along)
}

/// The distance from `center` to the farthest of `points`: the radius of a sphere around them.
pub(crate) fn radius(points: &[Vec3], center: Vec3) -> f32 {
    points
        .iter()
        .map(|point| point.distance(center))
        .fold(0.0, f32::max)
}

/// The canvas from -1 to 1, from where the pointer is in CSS pixels on an element as big.
pub(crate) fn on_canvas(x: f64, y: f64, width: f64, height: f64) -> Option<Vec2> {
    (width > 0.0 && height > 0.0).then(|| {
        Vec2::new(
            (2.0 * x / width - 1.0) as f32,
            (1.0 - 2.0 * y / height) as f32,
        )
    })
}

#[cfg(test)]
mod tests {
    use glam::{Quat, Vec2, Vec3};

    use super::{Ray, on_canvas, radius};
    use crate::{Camera, Mesh, Transform};

    fn down_the_z() -> Ray {
        Ray {
            origin: Vec3::new(0.0, 0.0, 10.0),
            direction: Vec3::NEG_Z,
        }
    }

    #[test]
    fn a_ray_through_the_middle_goes_to_what_the_camera_looks_at() {
        let camera = Camera::default().looking_at(Vec3::new(3.0, 4.0, 5.0), Vec3::ZERO);
        let Some(ray) = Ray::through(&camera, 1.5, Vec2::ZERO) else {
            panic!("a ray through the middle");
        };
        let to_target = (Vec3::ZERO - ray.origin).normalize();
        assert!(ray.direction.abs_diff_eq(to_target, 1e-4), "{ray:?}");
    }

    #[test]
    fn a_ray_hits_a_cube_on_its_face() {
        let cube = Mesh::cube();
        let hit = down_the_z().distance_to(&cube, &Transform::IDENTITY);
        assert_eq!(hit, Some(9.5), "the face at z = 0.5");

        // Moved aside, missed; turned and grown, hit sooner.
        let aside = Transform::from_translation(Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(down_the_z().distance_to(&cube, &aside), None);
        let turned = Transform::from_rotation(Quat::from_rotation_y(45_f32.to_radians()))
            .with_scale(Vec3::splat(2.0));
        let Some(hit) = down_the_z().distance_to(&cube, &turned) else {
            panic!("the turned cube hit");
        };
        assert!((hit - (10.0 - 2.0_f32.sqrt())).abs() < 1e-4, "{hit}");
    }

    #[test]
    fn behind_the_ray_or_flat_nothing_is_hit_but_small_is() {
        let behind = Transform::from_translation(Vec3::new(0.0, 0.0, 20.0));
        assert_eq!(down_the_z().distance_to(&Mesh::cube(), &behind), None);
        let flat = Transform::from_scale(Vec3::new(1.0, 0.0, 1.0));
        assert_eq!(
            down_the_z().distance_to(&Mesh::cube(), &flat),
            None,
            "squashed flat"
        );
        let tiny = Transform::from_scale(Vec3::splat(0.001));
        assert!(
            down_the_z().distance_to(&Mesh::cube(), &tiny).is_some(),
            "however small"
        );
    }

    #[test]
    fn the_nearest_of_what_the_ray_meets_is_picked() {
        let (cube, ball) = (Mesh::cube(), Mesh::sphere(16));
        let far = Transform::from_translation(Vec3::new(0.0, 0.0, -5.0));
        let near = Transform::from_translation(Vec3::new(0.0, 0.0, 2.0));
        let aside = Transform::from_translation(Vec3::new(5.0, 0.0, 0.0));

        let Some((index, pick)) =
            down_the_z().nearest([(&cube, &far), (&ball, &near), (&cube, &aside)])
        else {
            panic!("something picked");
        };
        assert_eq!(index, 1);
        assert!((pick.distance - 7.5).abs() < 1e-3, "{pick:?}");
        assert!(pick.point.abs_diff_eq(Vec3::new(0.0, 0.0, 2.5), 1e-3));
    }

    #[test]
    fn the_sphere_around_a_mesh_holds_it() {
        let points = [
            Vec3::ZERO,
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        let center = Vec3::new(1.0, 0.5, 0.0);
        assert!((radius(&points, center) - 1.25_f32.sqrt()).abs() < 1e-6);
        assert_eq!(radius(&[], Vec3::ZERO), 0.0);
    }

    #[test]
    fn a_ray_through_a_corner_of_triangles_meets_them() {
        // The top of a ball, where all the triangles of its top ring meet.
        let ball = Mesh::sphere(16);
        let lifted = Transform::from_translation(Vec3::new(0.0, 3.0, 0.0));
        let down = Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y);
        let Some(pick) = down.on_mesh(&ball, &lifted) else {
            panic!("the ball's top met");
        };
        assert!((pick.distance - 6.5).abs() < 1e-4, "{pick:?}");
    }

    #[test]
    fn a_ray_of_its_own_is_of_length_one_and_meets_a_mesh_from_inside() {
        let ray = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -4.0, 0.0));
        assert_eq!(ray.direction, Vec3::NEG_Y);
        let Some(pick) = ray.on_mesh(&Mesh::cube(), &Transform::IDENTITY) else {
            panic!("the cube's top met");
        };
        assert_eq!((pick.point, pick.distance), (Vec3::new(0.0, 0.5, 0.0), 4.5));

        // From inside, where it goes out.
        let inside = Ray::new(Vec3::ZERO, Vec3::X);
        let out = inside.on_mesh(&Mesh::cube(), &Transform::IDENTITY);
        assert_eq!(out.map(|pick| pick.point), Some(Vec3::new(0.5, 0.0, 0.0)));

        let nowhere = Ray::new(Vec3::ZERO, Vec3::ZERO);
        assert_eq!(nowhere.on_mesh(&Mesh::cube(), &Transform::IDENTITY), None);
        assert_eq!(down_the_z().nearest([]), None, "nothing to meet");
    }

    #[test]
    fn a_ray_meets_a_plane_ahead_of_it_only() {
        let ray = down_the_z();
        assert_eq!(
            ray.on_plane(Vec3::new(3.0, 3.0, 1.0), Vec3::Z),
            Some(Vec3::new(0.0, 0.0, 1.0))
        );
        assert_eq!(
            ray.on_plane(Vec3::new(0.0, 0.0, 12.0), Vec3::Z),
            None,
            "behind"
        );
        assert_eq!(ray.on_plane(Vec3::ZERO, Vec3::Y), None, "along it");
    }

    #[test]
    fn the_pointer_is_from_minus_one_to_one_across_the_canvas() {
        assert_eq!(
            on_canvas(0.0, 0.0, 200.0, 100.0),
            Some(Vec2::new(-1.0, 1.0))
        );
        assert_eq!(
            on_canvas(150.0, 75.0, 200.0, 100.0),
            Some(Vec2::new(0.5, -0.5))
        );
        assert_eq!(on_canvas(1.0, 1.0, 0.0, 100.0), None);
    }
}
