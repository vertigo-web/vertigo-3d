use glam::{
    Mat4, Vec3,
    camera::rh::{proj::opengl, view::look_at_mat4},
};

/// How a camera sees: with perspective, farther things smaller, or in parallel, as a plan does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Projection {
    /// Seeing `fov_y` radians from the bottom of the canvas to its top.
    Perspective { fov_y: f32 },
    /// Seeing `height` of the scene from the bottom of the canvas to its top, wherever it is.
    Orthographic { height: f32 },
}

/// Where a scene is seen from, and how. Its width follows the canvas's: the aspect ratio comes
/// from the canvas, so a scene keeps its shapes at any size.
///
/// The scene's Y is up, and a camera looks along -Z unless it's turned to look elsewhere.
///
/// ```rust
/// use vertigo_3d::{Camera, math::Vec3};
///
/// let camera = Camera::perspective(45_f32.to_radians())
///     .looking_at(Vec3::new(0.0, 5.0, 10.0), Vec3::ZERO);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub projection: Projection,
    /// Where it is.
    pub eye: Vec3,
    /// What is in the middle of the canvas.
    pub target: Vec3,
    /// Which way is up on the canvas, Y at first.
    pub up: Vec3,
    /// What is nearer than this, or farther than `far`, isn't seen.
    pub near: f32,
    pub far: f32,
}

impl Camera {
    /// A camera with perspective, at (0, 0, 5) looking at the origin.
    pub fn perspective(fov_y: f32) -> Self {
        Self::with(Projection::Perspective { fov_y })
    }

    /// A camera seeing in parallel, at (0, 0, 5) looking at the origin.
    pub fn orthographic(height: f32) -> Self {
        Self::with(Projection::Orthographic { height })
    }

    fn with(projection: Projection) -> Self {
        Self {
            projection,
            eye: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            near: 0.1,
            far: 1000.0,
        }
    }

    /// At `eye`, looking at `target`.
    pub fn looking_at(self, eye: Vec3, target: Vec3) -> Self {
        Self {
            eye,
            target,
            ..self
        }
    }

    pub fn with_up(self, up: Vec3) -> Self {
        Self { up, ..self }
    }

    /// Seeing from `near` to `far` only.
    pub fn with_clip(self, near: f32, far: f32) -> Self {
        Self { near, far, ..self }
    }

    /// The matrix taking the scene's points to the canvas, `aspect` its width by its height.
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        let projection = match self.projection {
            Projection::Perspective { fov_y } => {
                opengl::perspective(fov_y, aspect, self.near, self.far)
            }
            Projection::Orthographic { height } => {
                let (half_width, half_height) = (height * aspect / 2.0, height / 2.0);
                opengl::orthographic(
                    -half_width,
                    half_width,
                    -half_height,
                    half_height,
                    self.near,
                    self.far,
                )
            }
        };
        projection * look_at_mat4(self.eye, self.target, self.up.normalize_or(Vec3::Y))
    }
}

impl Default for Camera {
    /// With perspective of 45 degrees, at (0, 0, 5) looking at the origin.
    fn default() -> Self {
        Self::perspective(45_f32.to_radians())
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::Camera;

    #[test]
    fn what_it_looks_at_is_in_the_middle() {
        let target = Vec3::new(1.0, 2.0, 3.0);
        for camera in [Camera::perspective(1.0), Camera::orthographic(10.0)] {
            let camera = camera.looking_at(Vec3::new(-4.0, 6.0, 9.0), target);
            let seen = camera.view_projection(1.5).project_point3(target);
            assert!(
                seen.truncate().abs_diff_eq(glam::Vec2::ZERO, 1e-5),
                "{seen}"
            );
            assert!(
                (-1.0..1.0).contains(&seen.z),
                "between near and far: {seen}"
            );
        }
    }

    #[test]
    fn up_is_up_on_the_canvas() {
        let camera = Camera::default();
        let above = camera.view_projection(1.0).project_point3(Vec3::Y);
        assert!(above.y > 0.0 && above.x.abs() < 1e-6, "{above}");
    }
}
