//! Ten thousand cubes, one mesh, so one call draws them all; the camera goes round them.

use vertigo::{DomNode, Value, dom};
use vertigo_3d::{
    Camera, Color, Material, Mesh, Object, Scene, SceneParams, Transform, frame_time, math::Vec3,
};

use crate::background::css_frame;

/// Cubes along each side of the square.
const SIDE: usize = 100;

pub fn render() -> DomNode {
    let cube = Mesh::cube();
    let middle = (SIDE as f32 - 1.0) / 2.0;
    let objects: Vec<Object> = (0..SIDE * SIDE)
        .map(|index| {
            let (row, column) = ((index / SIDE) as f32, (index % SIDE) as f32);
            let height = 0.3 + 0.25 * ((row * 0.3).sin() + (column * 0.2).cos() + 2.0);
            Object::new(cube.clone())
                .transform(
                    Transform::from_translation(Vec3::new(
                        column - middle,
                        height / 2.0,
                        row - middle,
                    ))
                    .with_scale(Vec3::new(0.8, height, 0.8)),
                )
                .material(Material::lit(Color::rgb(
                    row / SIDE as f32,
                    0.4,
                    column / SIDE as f32,
                )))
        })
        .collect();

    // A turn every 30 seconds.
    let camera = frame_time().map(|time| {
        let angle = time * std::f32::consts::TAU / 30.0;
        Camera::perspective(45_f32.to_radians()).looking_at(
            Vec3::new(90.0 * angle.sin(), 60.0, 90.0 * angle.cos()),
            Vec3::ZERO,
        )
    });
    let params = SceneParams::default().background(Color::rgb(0.05, 0.05, 0.15));

    dom! {
        <div>
            <p>"10000 cubes of one mesh, drawn with one call, the camera going round them."</p>
            <div css={css_frame()}>
                <Scene {camera} objects={Value::new(objects).to_computed()} {params} />
            </div>
        </div>
    }
}
