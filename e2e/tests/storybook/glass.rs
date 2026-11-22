//! The "Glass" story: see-through panes, the farthest drawn first.

use vertigo_3d::{Camera, math::Vec3};
use vertigo_3d_e2e::prelude::*;

fn camera(behind: bool) -> Camera {
    let z = if behind { -6.0 } else { 6.0 };
    Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 1.5, z), Vec3::new(0.0, 1.0, 0.0))
}

/// The faces of the panes, a red one at z = 0.6 and a green one at z = -0.6, both 0.05 thick,
/// overlapping between x = -0.4 and 0.4.
const RED_FRONT: f32 = 0.625;
const GREEN_BACK: f32 = -0.625;

async fn pixel_of(ctx: &Ctx, behind: bool, point: Vec3) -> Result<[u8; 4]> {
    let (x, y) = ctx.spot(&camera(behind), point).await?;
    ctx.pixel(x, y).await
}

pub async fn the_nearer_pane_shows_more_of_its_color(ctx: Ctx) -> Result<()> {
    ctx.open("/glass").await?;
    let through_both = Vec3::new(0.0, 1.2, RED_FRONT);
    ctx.wait_for("red over green from the front", async || {
        let [red, green, ..] = pixel_of(&ctx, false, through_both).await?;
        Ok((u16::from(red) > u16::from(green) + 60).then_some(()))
    })
    .await?;

    ctx.click_button("From the back").await?;
    let through_both = Vec3::new(0.0, 1.2, GREEN_BACK);
    ctx.wait_for("green over red from the back", async || {
        let [red, green, ..] = pixel_of(&ctx, true, through_both).await?;
        Ok((u16::from(green) > u16::from(red) + 60).then_some(()))
    })
    .await
}

pub async fn a_pane_tints_what_is_behind_it(ctx: Ctx) -> Result<()> {
    ctx.open("/glass").await?;
    // Through the red pane alone, the night sky behind it.
    let red_alone = Vec3::new(-0.9, 1.4, RED_FRONT);
    ctx.wait_for("the sky tinted red, not covered", async || {
        let [red, green, blue, _] = pixel_of(&ctx, false, red_alone).await?;
        Ok((red > 90 && red < 200 && green < 40 && blue < 60).then_some(()))
    })
    .await
}

pub async fn a_window_shows_through_its_glass_and_not_its_frame(ctx: Ctx) -> Result<()> {
    ctx.open("/glass").await?;
    // Its front, 1.4 across at (2.6, 0.8, 0.6), the frame at its edges.
    let at = |u: f32, v: f32| Vec3::new(2.6 + 1.4 * (u - 0.5), 0.8 - 1.4 * (v - 0.5), 0.625);
    let (frame, glass) = (at(0.05, 0.5), at(0.3, 0.25));
    ctx.wait_for("the window drawn", async || {
        let frame = ctx_brightness(&ctx, frame).await?;
        let glass = ctx_brightness(&ctx, glass).await?;
        Ok((frame > glass + 150).then_some(()))
    })
    .await
}

async fn ctx_brightness(ctx: &Ctx, point: Vec3) -> Result<u32> {
    let (x, y) = ctx.spot(&camera(false), point).await?;
    ctx.brightness(x, y).await
}
