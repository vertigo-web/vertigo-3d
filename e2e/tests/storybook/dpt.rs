//! The "DPT" story: Dragging + Physics + Textures - wooden crates, a steel ball, a wooden ball
//! and a balloon falling onto a floor of tiles, moved by rapier3d, dragged around.

use vertigo_3d::{Camera, math::Vec3};
use vertigo_3d_e2e::prelude::*;

/// The top of the steel ball, 0.8 across, where it comes to rest: it falls alone.
const STEEL: Vec3 = Vec3::new(3.0, 0.8, 2.5);
/// The middle of the crate's top, 1 across, where it comes to rest: it falls alone, flat.
const CRATE: Vec3 = Vec3::new(-3.0, 1.0, 1.5);
/// On the floor, before the bodies.
const FLOOR: Vec3 = Vec3::new(0.0, 0.0, 4.5);

/// The story's camera.
fn camera() -> Camera {
    Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 8.0, 11.0), Vec3::new(0.0, 0.5, 0.0))
}

/// Whether `pixel` is of the steel ball: grey, or white where it shines, and light.
fn is_steel([red, green, blue, _]: [u8; 4]) -> bool {
    let (most, least) = (red.max(green).max(blue), red.min(green).min(blue));
    most - least < 40 && least > 90
}

/// Whether `pixel` is of wood, the crate's image on it: brown, red more than blue by far.
fn is_wood([red, green, blue, _]: [u8; 4]) -> bool {
    u16::from(red) > u16::from(blue) + 25 && green > blue
}

/// Whether `pixel` is of the floor's slate tiles: blue the most.
fn is_slate([red, green, blue, _]: [u8; 4]) -> bool {
    blue > red && blue > green
}

/// Waits until the pixel where `point` is shows what `is` tells, or doesn't.
async fn wait_for_at(
    ctx: &Ctx,
    point: Vec3,
    is: fn([u8; 4]) -> bool,
    there: bool,
    what: &str,
) -> Result<()> {
    let (x, y) = ctx.spot(&camera(), point).await?;
    ctx.wait_for(what, async || {
        Ok((is(ctx.pixel(x, y).await?) == there).then_some(()))
    })
    .await
}

/// Opens the story, and waits until all the bodies have fallen and are at rest.
async fn open_at_rest(ctx: &Ctx) -> Result<()> {
    ctx.open("/dpt").await?;
    ctx.wait_for_text("All at rest.").await?;
    wait_for_at(ctx, STEEL, is_steel, true, "the steel ball on the floor").await
}

pub async fn the_bodies_fall_onto_the_tiles_and_come_to_rest(ctx: Ctx) -> Result<()> {
    ctx.open("/dpt").await?;
    ctx.wait_for_text("DPT: Dragging + Physics + Textures")
        .await?;
    ctx.wait_for_text("Moving: 7 of 7").await?;
    ctx.wait_for_text("All at rest.").await?;

    wait_for_at(&ctx, STEEL, is_steel, true, "the steel ball on the floor").await?;
    // The crate's image came, and the floor's tiles are on it.
    wait_for_at(&ctx, CRATE, is_wood, true, "the crate wooden").await?;
    wait_for_at(&ctx, FLOOR, is_slate, true, "the floor of slate").await
}

pub async fn the_weights_are_told(ctx: Ctx) -> Result<()> {
    ctx.open("/dpt").await?;
    ctx.wait_for_text("steel ball 2.09").await?;
    ctx.wait_for_text("wooden ball 0.27").await?;
    ctx.wait_for_text("balloon 0.05").await
}

pub async fn a_ball_dragged_goes_with_the_pointer_and_stays_where_let_go(ctx: Ctx) -> Result<()> {
    open_at_rest(&ctx).await?;

    // Two to the left along the floor, in three moves, held there until it's caught up.
    let to = STEEL - Vec3::X * 2.0;
    let (canvas, from_x, from_y) = ctx.aim(&camera(), STEEL).await?;
    let (_, to_x, to_y) = ctx.aim(&camera(), to).await?;
    let (x, y) = (to_x - from_x, to_y - from_y);
    ctx.driver
        .action_chain()
        .move_to_element_with_offset(&canvas, from_x, from_y)
        .click_and_hold()
        .move_by_offset(x / 3, y / 3)
        .move_by_offset(x / 3, y / 3)
        .move_by_offset(x - 2 * (x / 3), y - 2 * (y / 3))
        .perform()
        .await
        .context("dragging failed")?;
    ctx.wait_for_text("Holding: steel ball").await?;
    // Held a little above where it was grabbed, so its top is up there too.
    wait_for_at(
        &ctx,
        to + Vec3::Y * 0.3,
        is_steel,
        true,
        "the steel ball held",
    )
    .await?;

    ctx.driver
        .action_chain()
        .release()
        .perform()
        .await
        .context("letting go failed")?;
    ctx.wait_for_text("Let go: steel ball").await?;
    ctx.wait_for_text("All at rest.").await?;
    wait_for_at(
        &ctx,
        to,
        is_steel,
        true,
        "the steel ball where it was let go",
    )
    .await?;
    wait_for_at(&ctx, STEEL, is_steel, false, "and not where it was").await
}
