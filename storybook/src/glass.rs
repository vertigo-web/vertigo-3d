//! See-through objects: two panes of colored glass, one behind the other, the nearer showing
//! more of its color from either side, a bubble with a box inside, and a window of frosted glass
//! in a frame before a ball.

use vertigo::{DomNode, Value, bind, dom};
use vertigo_3d::{
    Camera, Color, Material, Mesh, Object, Scene, SceneParams, Texture, Transform,
    math::{Quat, Vec3},
};

use crate::background::css_frame;

/// A frame, opaque, round glass a third opaque.
fn window() -> Texture {
    const SIZE: u32 = 16;
    let pixels = (0..SIZE)
        .flat_map(|row| (0..SIZE).map(move |column| (column, row)))
        .flat_map(|(column, row)| {
            let frame = column < 2 || row < 2 || column >= SIZE - 2 || row >= SIZE - 2;
            let bar = column == SIZE / 2 || row == SIZE / 2;
            if frame || bar {
                [240, 240, 235, 255]
            } else {
                [200, 230, 255, 85]
            }
        })
        .collect();
    Texture::pixels(SIZE, SIZE, pixels)
        .expect("the pixels are made as many as the size")
        .sharp()
}

/// The camera, in front of the panes or behind them.
fn camera(behind: bool) -> Camera {
    let z = if behind { -6.0 } else { 6.0 };
    Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 1.5, z), Vec3::new(0.0, 1.0, 0.0))
}

pub fn render() -> DomNode {
    let behind = Value::new(false);
    let buttons: Vec<DomNode> = [("From the front", false), ("From the back", true)]
        .into_iter()
        .map(|(name, value)| {
            let on_click = bind!(behind, |_| behind.set(value));
            dom! { <button on_click={on_click}>{name}</button> }
        })
        .collect();

    let pane = |x: f32, z: f32, color: Color| {
        Object::new(Mesh::cube())
            .transform(
                Transform::from_translation(Vec3::new(x, 1.0, z))
                    .with_scale(Vec3::new(1.6, 1.2, 0.05)),
            )
            .material(Material::color(color))
    };
    let red = pane(-0.4, 0.6, Color::rgba(1.0, 0.0, 0.0, 0.5));
    let green = pane(0.4, -0.6, Color::rgba(0.0, 1.0, 0.0, 0.5));

    let floor = Object::new(Mesh::plane())
        .transform(Transform::from_scale(Vec3::new(10.0, 1.0, 10.0)))
        .material(Material::lit(Color::rgb(0.5, 0.5, 0.55)));
    let bubble = Object::new(Mesh::sphere(48))
        .transform(
            Transform::from_translation(Vec3::new(-2.6, 0.8, 0.0)).with_scale(Vec3::splat(1.6)),
        )
        .material(Material::lit(Color::rgba(0.6, 0.8, 1.0, 0.35)));
    let inside = Object::new(Mesh::cube())
        .transform(
            Transform::from_translation(Vec3::new(-2.6, 0.8, 0.0))
                .with_rotation(Quat::from_rotation_y(0.6))
                .with_scale(Vec3::splat(0.5)),
        )
        .material(Material::lit(Color::rgb(1.0, 0.8, 0.1)));
    let ball = Object::new(Mesh::sphere(32))
        .transform(Transform::from_translation(Vec3::new(2.6, 0.5, -0.4)))
        .material(Material::lit(Color::rgb(0.9, 0.3, 0.6)));
    let window = Object::new(Mesh::cube())
        .transform(
            Transform::from_translation(Vec3::new(2.6, 0.8, 0.6))
                .with_scale(Vec3::new(1.4, 1.4, 0.05)),
        )
        .material(Material::lit(Color::WHITE).texture(window()).see_through());

    let objects = vec![floor, inside, ball, red, green, bubble, window];
    let params = SceneParams::default().background(Color::rgb(0.05, 0.05, 0.15));

    dom! {
        <div>
            <p>{..buttons}</p>
            <div css={css_frame()}>
                <Scene
                    camera={behind.map(camera)}
                    objects={Value::new(objects).to_computed()}
                    {params}
                />
            </div>
        </div>
    }
}
