//! The "Relief" story: a wall of ridges lit from above, flat, bumpy by a normal map, or deep by
//! a height map too.

use vertigo_3d::{Camera, math::Vec3};
use vertigo_3d_e2e::prelude::*;

/// The story's camera, from above the wall.
fn camera() -> Camera {
    Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 1.2, 2.6), Vec3::new(0.0, -0.2, 0.0))
}

/// Where on the wall a ridge faces up, toward the light, and where it faces down, away from it:
/// the texture repeats twice down the wall from its top at 1, with ridges 16 of its 64 rows
/// apart, facing up where they rise down, at a row of 16, 32, ...
const FACING_UP: Vec3 = Vec3::new(0.0, -0.25, 0.0);
const FACING_DOWN: Vec3 = Vec3::new(0.0, -0.375, 0.0);

/// How light the wall is where a ridge faces up, and where one faces down.
async fn ridges(ctx: &Ctx) -> Result<(u32, u32)> {
    let up = ctx.spot(&camera(), FACING_UP).await?;
    let down = ctx.spot(&camera(), FACING_DOWN).await?;
    Ok((
        ctx.brightness(up.0, up.1).await?,
        ctx.brightness(down.0, down.1).await?,
    ))
}

pub async fn a_normal_map_shows_ridges_the_light_falls_on(ctx: Ctx) -> Result<()> {
    ctx.open("/relief").await?;
    ctx.wait_for("the flat wall, as light all over", async || {
        let (up, down) = ridges(&ctx).await?;
        Ok((up > 150 && up.abs_diff(down) <= 9).then_some(()))
    })
    .await?;

    ctx.click_button("Bumpy").await?;
    ctx.wait_for(
        "the ridges facing up lit, those facing down dark",
        async || {
            let (up, down) = ridges(&ctx).await?;
            Ok((up > down + 300).then_some(()))
        },
    )
    .await
}

pub async fn a_height_map_shifts_the_ridges_seen_at_a_slant(ctx: Ctx) -> Result<()> {
    ctx.open("/relief").await?;
    ctx.click_button("Bumpy").await?;
    ctx.wait_for("the ridges", async || {
        let (up, down) = ridges(&ctx).await?;
        Ok((up > down + 300).then_some(()))
    })
    .await?;
    let bumpy = ctx.picture().await?;

    ctx.click_button("Deep").await?;
    ctx.wait_for("the ridges shifted", async || {
        Ok((ctx.picture().await? != bumpy).then_some(()))
    })
    .await
}

/// How many pixels wide the scene's canvas is drawn.
async fn canvas_width(ctx: &Ctx) -> Result<u64> {
    ctx.js("return document.querySelector('canvas').width;")
        .await?
        .as_u64()
        .context("the width of the canvas")
}

/// A low detail draws half as many pixels across, and the wall flat, without its relief: a
/// computer back at the full detail draws all again.
pub async fn a_low_detail_draws_fewer_pixels_and_no_relief(ctx: Ctx) -> Result<()> {
    ctx.open("/relief").await?;
    ctx.click_button("Bumpy").await?;
    ctx.wait_for("the ridges", async || {
        let (up, down) = ridges(&ctx).await?;
        Ok((up > down + 300).then_some(()))
    })
    .await?;
    let full = canvas_width(&ctx).await?;

    ctx.click_button("Low").await?;
    ctx.wait_for("a canvas half as wide, of a flat wall", async || {
        let width = canvas_width(&ctx).await?;
        let (up, down) = ridges(&ctx).await?;
        Ok((width.abs_diff(full / 2) <= 1 && up > 150 && up.abs_diff(down) <= 9).then_some(()))
    })
    .await?;

    ctx.click_button("Full").await?;
    ctx.wait_for(
        "the canvas as wide as before, with the ridges",
        async || {
            let width = canvas_width(&ctx).await?;
            let (up, down) = ridges(&ctx).await?;
            Ok((width == full && up > down + 300).then_some(()))
        },
    )
    .await
}
