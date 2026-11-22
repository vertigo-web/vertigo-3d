//! The "Light" story: a lit ball on a lit floor, under the light of the time of day, or of
//! a lamp, shiny or not.

use vertigo_3d::{Camera, math::Vec3};
use vertigo_3d_e2e::prelude::*;

/// Where on the canvas: the ball is in the middle, half as high as the canvas.
const TOP: (f64, f64) = (0.5, 0.36);
const BOTTOM: (f64, f64) = (0.5, 0.64);
const LEFT: (f64, f64) = (0.44, 0.5);
const RIGHT: (f64, f64) = (0.56, 0.5);

/// Waits until the ball is lighter at `lit` than at `dark`, by a good margin.
async fn wait_for_lighter(ctx: &Ctx, lit: (f64, f64), dark: (f64, f64), what: &str) -> Result<()> {
    ctx.wait_for(what, async || {
        let (lit, dark) = (
            ctx.brightness(lit.0, lit.1).await?,
            ctx.brightness(dark.0, dark.1).await?,
        );
        Ok((lit > dark + 60).then_some(()))
    })
    .await
}

pub async fn at_noon_the_top_is_lit(ctx: Ctx) -> Result<()> {
    ctx.open("/light").await?;
    wait_for_lighter(&ctx, TOP, BOTTOM, "the top lighter than the bottom").await
}

pub async fn the_lit_side_follows_the_sun(ctx: Ctx) -> Result<()> {
    ctx.open("/light").await?;

    ctx.click_button("Morning").await?;
    wait_for_lighter(&ctx, RIGHT, LEFT, "the east side, on the right, lit").await?;

    ctx.click_button("Evening").await?;
    wait_for_lighter(&ctx, LEFT, RIGHT, "the west side, on the left, lit").await?;

    ctx.click_button("Night").await?;
    ctx.wait_for("the ball dark", async || {
        let (left, right) = (
            ctx.brightness(LEFT.0, LEFT.1).await?,
            ctx.brightness(RIGHT.0, RIGHT.1).await?,
        );
        Ok((left < 200 && right < 200 && left.abs_diff(right) <= 6).then_some(()))
    })
    .await
}

/// The lamp is at the left of the ball and in front of it, so the left of the ball is lit, and
/// the right, away from it, is dark.
pub async fn a_lamp_lights_the_side_facing_it(ctx: Ctx) -> Result<()> {
    ctx.open("/light").await?;
    ctx.click_button("Lamp").await?;
    wait_for_lighter(&ctx, LEFT, RIGHT, "the side facing the lamp lit").await
}

/// At noon, with the sun straight down and the camera in front, a shiny ball shines white
/// between its top and its front, where the sun reflects toward the camera: half way between
/// up and toward the camera, on the ball of radius 1 around (0, 1, 0).
pub async fn a_shiny_ball_has_a_highlight(ctx: Ctx) -> Result<()> {
    ctx.open("/light").await?;
    let camera = Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 1.0, 5.0), Vec3::new(0.0, 1.0, 0.0));
    let seen = (camera.eye - Vec3::new(0.0, 1.0, 0.0)).normalize();
    let highlight = Vec3::new(0.0, 1.0, 0.0) + (Vec3::Y + seen).normalize();
    let (x, y) = ctx.spot(&camera, highlight).await?;

    let matte = ctx
        .wait_for("the ball drawn", async || {
            let brightness = ctx.brightness(x, y).await?;
            Ok((brightness > 100).then_some(brightness))
        })
        .await?;
    ctx.click_button("Shiny").await?;
    ctx.wait_for("the highlight", async || {
        let shiny = ctx.brightness(x, y).await?;
        Ok((shiny > matte + 40 && shiny > 230 * 3).then_some(()))
    })
    .await
}
