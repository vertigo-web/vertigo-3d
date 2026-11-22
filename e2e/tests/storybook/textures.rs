//! The "Textures" story: a crate from an image file, tiles from pixels, a fence with holes.

use vertigo_3d::{
    Camera, Transform,
    math::{Quat, Vec3},
};
use vertigo_3d_e2e::prelude::*;

/// The story's camera.
fn camera() -> Camera {
    Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 3.0, 6.0), Vec3::new(0.0, 0.5, 0.0))
}

/// Where `uv` of the fence's texture is, the fence standing 1.6 across at (0, 0.8, 1).
fn on_the_fence(u: f32, v: f32) -> Vec3 {
    Vec3::new(1.6 * (u - 0.5), 0.8 - 1.6 * (v - 0.5), 1.0)
}

/// The wooden bars of the crate and the fence: brown, red the most and blue the least.
fn is_wood([red, green, blue, _]: [u8; 4]) -> bool {
    red > green && green > blue && u16::from(red) > u16::from(blue) + 50
}

/// The red box: red, by far.
fn is_red([red, green, blue, _]: [u8; 4]) -> bool {
    u16::from(red) > u16::from(green) * 2 && u16::from(red) > u16::from(blue) * 2
}

async fn wait_for(ctx: &Ctx, point: Vec3, what: &str, is: fn([u8; 4]) -> bool) -> Result<()> {
    let (x, y) = ctx.spot(&camera(), point).await?;
    ctx.wait_for(what, async || Ok(is(ctx.pixel(x, y).await?).then_some(())))
        .await
}

/// Metal plates: blue the most, red the least.
fn is_metal([red, green, blue, _]: [u8; 4]) -> bool {
    blue > green && green > red && u16::from(blue) > u16::from(red) + 12
}

pub async fn the_crate_has_its_image_from_a_file(ctx: Ctx) -> Result<()> {
    ctx.open("/textures").await?;
    // On a plank of its front, away from the brace across it, and the ribs of metal.
    let crate_ = Transform::from_translation(Vec3::new(-2.0, 0.5, -0.5))
        .with_rotation(Quat::from_rotation_y(0.4))
        .to_matrix();
    let plank = crate_.transform_point3(Vec3::new(0.25, 0.25, 0.5));
    wait_for(&ctx, plank, "the crate's wood", is_wood).await?;

    ctx.click_button("Metal").await?;
    wait_for(&ctx, plank, "the crate's metal", is_metal).await
}

/// The metal is preloaded: its image comes while the crate is wooden, and the crate is metal in
/// the first picture drawn after it is chosen.
pub async fn the_preloaded_metal_shows_at_once(ctx: Ctx) -> Result<()> {
    const FETCHED: &str = "return performance.getEntriesByType('resource')
        .some((entry) => entry.name.split('/').pop().startsWith('metal.'));";

    ctx.open("/textures").await?;
    let crate_ = Transform::from_translation(Vec3::new(-2.0, 0.5, -0.5))
        .with_rotation(Quat::from_rotation_y(0.4))
        .to_matrix();
    let plank = crate_.transform_point3(Vec3::new(0.25, 0.25, 0.5));
    wait_for(&ctx, plank, "the crate's wood", is_wood).await?;
    ctx.wait_for("the metal fetched", async || {
        Ok(ctx
            .js(FETCHED)
            .await?
            .as_bool()
            .unwrap_or(false)
            .then_some(()))
    })
    .await?;
    // Decoded and put in its texture after it came
    ctx.js("return new Promise((done) => setTimeout(done, 500));")
        .await?;

    ctx.click_button("Metal").await?;
    let (x, y) = ctx.spot(&camera(), plank).await?;
    let first = ctx.pixel(x, y).await?;
    ensure!(
        is_metal(first),
        "the crate was {first:?} when the metal was chosen"
    );
    Ok(())
}

/// The scene tells how many of its images came: the wood and the preloaded metal, then the metal
/// only, the wood let go when the crate is metal, and both again when it's wooden.
pub async fn the_images_that_came_are_counted(ctx: Ctx) -> Result<()> {
    ctx.open("/textures").await?;
    ctx.wait_for_text("Images loaded: 2 of 2, 0 failed").await?;

    ctx.click_button("Metal").await?;
    ctx.wait_for_text("Images loaded: 1 of 1, 0 failed").await?;
    ctx.click_button("Wood").await?;
    ctx.wait_for_text("Images loaded: 2 of 2, 0 failed").await
}

pub async fn the_floor_is_tiled_with_pixels(ctx: Ctx) -> Result<()> {
    ctx.open("/textures").await?;
    // Two tiles side by side, a light and a dark one.
    let (light, dark) = (Vec3::new(0.5, 0.0, 2.5), Vec3::new(1.5, 0.0, 2.5));
    let (x, y) = ctx.spot(&camera(), dark).await?;
    ctx.wait_for("the scene drawn", async || {
        Ok((ctx.brightness(x, y).await? > 0).then_some(()))
    })
    .await?;

    let (light_x, light_y) = ctx.spot(&camera(), light).await?;
    let (light, dark) = (
        ctx.brightness(light_x, light_y).await?,
        ctx.brightness(x, y).await?,
    );
    ensure!(
        light > dark + 150,
        "a light tile {light} next to a dark one {dark}"
    );
    Ok(())
}

pub async fn the_holes_in_the_fence_show_what_is_behind(ctx: Ctx) -> Result<()> {
    ctx.open("/textures").await?;
    // The middle bar, and the hole left of it at the top, the red box behind.
    wait_for(
        &ctx,
        on_the_fence(0.5, 0.28),
        "the fence's middle bar",
        is_wood,
    )
    .await?;
    wait_for(
        &ctx,
        on_the_fence(0.28, 0.28),
        "the red box through a hole",
        is_red,
    )
    .await
}
