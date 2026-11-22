//! A lit ball on a lit floor, under the light of the time of day chosen with buttons, or of a
//! lamp at night, shiny or not.

use vertigo::{DomNode, Value, bind, dom};
use vertigo_3d::{
    Camera, Color, Light, Material, Mesh, Object, Scene, SceneParams, Transform, math::Vec3,
};

use crate::background::css_frame;

/// Where the lamp is.
pub const LAMP: Vec3 = Vec3::new(-2.0, 1.5, 1.0);

fn time_of_day(name: &str) -> Vec<Light> {
    let sun = |direction: Vec3, color: Color| {
        vec![
            Light::ambient(0.15),
            Light::directional(direction).color(color),
        ]
    };
    match name {
        // From the east, +X, low and warm
        "Morning" => sun(Vec3::new(-1.0, -0.3, 0.0), Color::rgb(1.0, 0.85, 0.7)),
        // Straight down
        "Noon" => sun(Vec3::NEG_Y, Color::WHITE),
        // From the west, -X
        "Evening" => sun(Vec3::new(1.0, -0.3, 0.0), Color::rgb(1.0, 0.6, 0.4)),
        // Left of the ball and in front of it, reaching not much further than the ball
        "Lamp" => vec![
            Light::ambient(0.1),
            Light::point(LAMP)
                .range(5.0)
                .color(Color::rgb(1.0, 0.9, 0.7)),
        ],
        _ => vec![Light::ambient(0.15)],
    }
}

pub fn render() -> DomNode {
    let chosen = Value::new("Noon");
    let lights = chosen.map(time_of_day);
    let shiny = Value::new(false);

    let buttons: Vec<DomNode> = ["Morning", "Noon", "Evening", "Night", "Lamp"]
        .into_iter()
        .map(|name| {
            let on_click = bind!(chosen, name, |_| chosen.set(name));
            dom! { <button on_click={on_click}>{name}</button> }
        })
        .collect();
    let toggle_shiny = bind!(shiny, |_| shiny.change(|shiny| *shiny = !*shiny));

    let floor = Object::new(Mesh::plane())
        .transform(Transform::from_scale(Vec3::new(20.0, 1.0, 20.0)))
        .material(Material::lit(Color::rgb(0.6, 0.6, 0.6)));
    let ball = Object::new(Mesh::sphere(48))
        .transform(
            Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)).with_scale(Vec3::splat(2.0)),
        )
        .material(shiny.map(|shiny| {
            let ball = Material::lit(Color::rgb(0.9, 0.9, 0.9));
            if shiny {
                ball.specular(Color::WHITE, 30.0)
            } else {
                ball
            }
        }));

    let camera = Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 1.0, 5.0), Vec3::new(0.0, 1.0, 0.0));
    let params = SceneParams::default()
        .background(Color::rgb(0.05, 0.05, 0.15))
        .lights(lights);

    dom! {
        <div>
            <p>{..buttons} <button on_click={toggle_shiny}>"Shiny"</button></p>
            <div css={css_frame()}>
                <Scene
                    camera={Value::new(camera).to_computed()}
                    objects={Value::new(vec![floor, ball]).to_computed()}
                    {params}
                />
            </div>
        </div>
    }
}
