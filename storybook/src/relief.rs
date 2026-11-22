//! A wall of ridges lit from above, seen at a slant: flat, bumpy by a normal map, or deep by
//! a height map too, which shifts the ridges with the slant they are seen at, in a detail chosen
//! as for a slow computer. Its textures are made here, of pixels.

use std::f32::consts::{FRAC_PI_2, TAU};

use vertigo::{DomNode, Value, bind, dom};
use vertigo_3d::{
    Camera, Color, Light, Material, Mesh, Object, Scene, SceneParams, Texture, Transform,
    math::{Quat, Vec3},
};

use crate::background::css_frame;

/// Pixels a side of the textures has.
const SIZE: usize = 64;
/// Pixels from a ridge to the next.
const PERIOD: f32 = 16.0;

/// From above the wall, looking down at it.
pub fn camera() -> Camera {
    Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 1.2, 2.6), Vec3::new(0.0, -0.2, 0.0))
}

/// The height of the ridges at `row`, from 0 to 1: rising down from the first row.
fn height(row: usize) -> f32 {
    0.5 + 0.5 * (TAU * row as f32 / PERIOD).sin()
}

/// A texture of `SIZE` rows of `SIZE` pixels, each row of `pixel`.
fn rows(pixel: impl Fn(usize) -> [u8; 4]) -> Texture {
    let pixels = (0..SIZE)
        .flat_map(|row| std::iter::repeat_n(pixel(row), SIZE).flatten())
        .collect();
    Texture::pixels(SIZE as u32, SIZE as u32, pixels)
        .expect("the pixels are made as many as the size")
}

fn byte(part: f32) -> u8 {
    (part.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Where the ridges rise down the texture, the surface faces up.
fn normal_map() -> Texture {
    rows(|row| {
        let slope = (TAU * row as f32 / PERIOD).cos();
        let normal = Vec3::new(0.0, slope, 1.0).normalize();
        [
            byte(normal.x * 0.5 + 0.5),
            byte(normal.y * 0.5 + 0.5),
            byte(normal.z * 0.5 + 0.5),
            255,
        ]
    })
}

fn height_map() -> Texture {
    rows(|row| {
        let height = byte(height(row));
        [height, height, height, 255]
    })
}

pub fn render() -> DomNode {
    let chosen = Value::new("Flat");
    let (normal, heights) = (normal_map(), height_map());
    let material = chosen.map(move |chosen| {
        let wall = Material::lit(Color::rgb(0.8, 0.8, 0.8)).repeat(2.0, 2.0);
        match chosen {
            "Bumpy" => wall.normal_map(normal.clone()),
            "Deep" => wall
                .normal_map(normal.clone())
                .height_map(heights.clone(), 0.15),
            _ => wall,
        }
    });

    let buttons: Vec<DomNode> = ["Flat", "Bumpy", "Deep"]
        .into_iter()
        .map(|name| {
            let on_click = bind!(chosen, name, |_| chosen.set(name));
            dom! { <button on_click={on_click}>{name}</button> }
        })
        .collect();

    // As many pixels as the screen has, half as many across and down without smoothing, or
    // a quarter of them without the relief either
    let detail = Value::new("Full");
    let details: Vec<DomNode> = ["Full", "Medium", "Low"]
        .into_iter()
        .map(|name| {
            let on_click = bind!(detail, name, |_| detail.set(name));
            dom! { <button on_click={on_click}>{name}</button> }
        })
        .collect();

    // Standing up, facing the camera, 2 across
    let wall = Object::new(Mesh::plane())
        .transform(
            Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2))
                .with_scale(Vec3::new(2.0, 1.0, 2.0)),
        )
        .material(material);
    let lights = vec![
        Light::ambient(0.15),
        Light::directional(Vec3::new(0.0, -1.0, -0.5)),
    ];
    let params = SceneParams::default()
        .background(Color::rgb(0.05, 0.05, 0.15))
        .lights(lights)
        .bitmap_scale(detail.map(|detail| match detail {
            "Medium" => 0.75,
            "Low" => 0.5,
            _ => 1.0,
        }))
        .antialias(detail.map(|detail| detail == "Full"))
        .relief(detail.map(|detail| detail != "Low"));

    dom! {
        <div>
            <p>{..buttons} " Detail: " {..details}</p>
            <div css={css_frame()}>
                <Scene
                    camera={Value::new(camera()).to_computed()}
                    objects={Value::new(vec![wall]).to_computed()}
                    {params}
                />
            </div>
        </div>
    }
}
