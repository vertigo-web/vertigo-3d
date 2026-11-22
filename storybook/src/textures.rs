//! Textures: a crate from an image file, wooden or metal as chosen with buttons, the metal loaded
//! before it is shown, how many images came, a floor of tiles
//! and a beach ball from pixels made in Rust, and a fence with holes, through which a red box
//! behind it shows.

use vertigo::{DomNode, Value, bind, dom, include_static};
use vertigo_3d::{
    Camera, Color, Loading, Material, Mesh, Object, Scene, SceneParams, Texture, Transform,
    math::{Quat, Vec3},
};

use crate::background::css_frame;

/// `width` by `height` pixels, each of `color` at its column and row.
fn pixels(width: u32, height: u32, color: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    (0..height)
        .flat_map(|row| (0..width).map(move |column| (column, row)))
        .flat_map(|(column, row)| color(column, row))
        .collect()
}

/// A texture of `pixels`, which are made `width` by `height`, read from the nearest pixel.
fn sharp(width: u32, height: u32, pixels: Vec<u8>) -> Texture {
    Texture::pixels(width, height, pixels)
        .expect("the pixels are made as many as the size")
        .sharp()
}

/// Light and dark tiles, two by two.
pub fn checker() -> Texture {
    let tiles = pixels(2, 2, |column, row| {
        if (column + row) % 2 == 0 {
            [200, 200, 190, 255]
        } else {
            [90, 110, 90, 255]
        }
    });
    sharp(2, 2, tiles)
}

/// Stripes of colors round, as a beach ball.
fn beach_ball() -> Texture {
    const STRIPES: [[u8; 4]; 6] = [
        [230, 50, 40, 255],
        [255, 255, 255, 255],
        [40, 100, 220, 255],
        [255, 255, 255, 255],
        [250, 200, 30, 255],
        [255, 255, 255, 255],
    ];
    let stripes = pixels(6, 1, |column, _| STRIPES[column as usize]);
    sharp(6, 1, stripes)
}

/// Pixels across and down the fence's texture: bars of 2 at its edges and in its middle, holes
/// between.
pub const FENCE: u32 = 16;

/// Wooden bars, a lattice with holes between them.
fn fence() -> Texture {
    let lattice = pixels(FENCE, FENCE, |column, row| {
        if column % 7 < 2 || row % 7 < 2 {
            [150, 100, 50, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    sharp(FENCE, FENCE, lattice)
}

pub fn render() -> DomNode {
    let floor = Object::new(Mesh::plane())
        .transform(Transform::from_scale(Vec3::new(10.0, 1.0, 10.0)))
        .material(
            Material::lit(Color::WHITE)
                .texture(checker())
                .repeat(5.0, 5.0),
        );
    // The wood loaded when first shown, the metal with the scene, so the crate shows it at once
    // when chosen.
    let (wood, metal) = (
        Texture::url(include_static!("crate.png")),
        Texture::url(include_static!("metal.png")),
    );
    let metallic = Value::new(false);
    let buttons: Vec<DomNode> = [("Wood", false), ("Metal", true)]
        .into_iter()
        .map(|(name, value)| {
            let on_click = bind!(metallic, |_| metallic.set(value));
            dom! { <button on_click={on_click}>{name}</button> }
        })
        .collect();

    let a_crate = Object::new(Mesh::cube())
        .transform(
            Transform::from_translation(Vec3::new(-2.0, 0.5, -0.5))
                .with_rotation(Quat::from_rotation_y(0.4)),
        )
        .material(metallic.map({
            let metal = metal.clone();
            move |metallic| {
                Material::lit(Color::WHITE).texture(if metallic {
                    metal.clone()
                } else {
                    wood.clone()
                })
            }
        }));
    let ball = Object::new(Mesh::sphere(32))
        .transform(
            Transform::from_translation(Vec3::new(2.0, 0.6, -0.5))
                .with_rotation(Quat::from_rotation_z(0.3))
                .with_scale(Vec3::splat(1.2)),
        )
        .material(Material::lit(Color::WHITE).texture(beach_ball()));
    let red_box = Object::new(Mesh::cube())
        .transform(
            Transform::from_translation(Vec3::new(0.0, 0.5, -0.5))
                .with_scale(Vec3::new(1.4, 1.0, 0.6)),
        )
        .material(Material::lit(Color::rgb(0.9, 0.15, 0.1)));
    // Standing up, facing the camera.
    let fence = Object::new(Mesh::plane())
        .transform(
            Transform::from_translation(Vec3::new(0.0, 0.8, 1.0))
                .with_rotation(Quat::from_rotation_x(90_f32.to_radians()))
                .with_scale(Vec3::new(1.6, 1.0, 1.6)),
        )
        .material(Material::lit(Color::WHITE).texture(fence()));

    let camera = Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 3.0, 6.0), Vec3::new(0.0, 0.5, 0.0));
    let loading = Value::new(Loading::default());
    let params = SceneParams::default()
        .background(Color::rgb(0.05, 0.05, 0.15))
        .preload(vec![metal])
        .on_load(bind!(loading, |now| loading.set(now)));
    let images = loading.map(|loading| {
        format!(
            "Images loaded: {} of {}, {} failed",
            loading.loaded, loading.all, loading.failed
        )
    });

    dom! {
        <div>
            <p>"A crate from an image file: " {..buttons}</p>
            <p>"Tiles and a beach ball from pixels, a fence with holes."</p>
            <p>{images}</p>
            <div css={css_frame()}>
                <Scene
                    camera={Value::new(camera).to_computed()}
                    objects={Value::new(vec![floor, a_crate, ball, red_box, fence]).to_computed()}
                    {params}
                />
            </div>
        </div>
    }
}
