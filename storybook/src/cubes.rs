//! A scene standing still - a floor, cubes and a ball - drawn only when it changes: when the
//! ball is hidden or shown, or the camera goes round. The floor and the cubes are lit, the ball
//! is one color all over.

use std::f32::consts::PI;

use vertigo::{DomNode, Value, bind, dom};
use vertigo_3d::{
    Camera, Color, Material, Mesh, Object, Scene, SceneParams, Transform, math::Vec3,
};

use crate::background::css_frame;

/// The ball's red, `Color::rgb(0.8, 0.2, 0.2)`.
pub const RED: Color = Color::rgb(0.8, 0.2, 0.2);

pub fn render() -> DomNode {
    let floor = Object::new(Mesh::plane())
        .transform(Transform::from_scale(Vec3::new(8.0, 1.0, 8.0)))
        .material(Material::lit(Color::rgb(0.5, 0.5, 0.5)));

    let cube = Mesh::cube();
    let cubes = (0..8).map(|index| {
        let angle = index as f32 * PI / 4.0;
        Object::new(cube.clone())
            .transform(Transform::from_translation(Vec3::new(
                2.5 * angle.cos(),
                0.5,
                2.5 * angle.sin(),
            )))
            .material(Material::lit(Color::rgb(
                0.2,
                0.4 + index as f32 * 0.07,
                0.8,
            )))
    });

    let ball = Object::new(Mesh::sphere(32))
        .transform(Transform::from_translation(Vec3::new(0.0, 0.6, 0.0)))
        .material(Material::color(RED));

    let scenery: Vec<Object> = [floor].into_iter().chain(cubes).collect();
    let shown = Value::new(true);
    let objects = shown.map({
        let scenery = scenery.clone();
        move |shown| {
            let mut objects = scenery.clone();
            if shown {
                objects.push(ball.clone());
            }
            objects
        }
    });

    // The camera goes round the middle, an eighth of a turn a click.
    let turn = Value::new(0_i32);
    let camera = turn.map(|turn| {
        let angle = turn as f32 * PI / 4.0;
        Camera::perspective(45_f32.to_radians()).looking_at(
            Vec3::new(7.0 * angle.sin(), 4.0, 7.0 * angle.cos()),
            Vec3::new(0.0, 0.6, 0.0),
        )
    });

    let hide = bind!(shown, |_| shown.set(false));
    let show = bind!(shown, |_| shown.set(true));
    let left = bind!(turn, |_| turn.change(|turn| *turn -= 1));
    let right = bind!(turn, |_| turn.change(|turn| *turn += 1));

    let params = SceneParams::default().background(Color::rgb(0.05, 0.05, 0.15));

    dom! {
        <div>
            <p>
                <button on_click={hide}>"Hide the ball"</button>
                <button on_click={show}>"Show the ball"</button>
                <button on_click={left}>"Go left"</button>
                <button on_click={right}>"Go right"</button>
            </p>
            <div css={css_frame()}>
                <Scene {camera} {objects} {params} />
            </div>
        </div>
    }
}
