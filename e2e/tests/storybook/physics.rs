//! The "Physics" story: boxes and balls falling onto a floor, moved by rapier3d, thrown up by a
//! click.

use vertigo_3d::{Camera, math::Vec3};
use vertigo_3d_e2e::prelude::*;

/// The middle of the red box's top, 1 across, where it comes to rest: it falls alone, flat.
const RED: Vec3 = Vec3::new(-3.0, 1.0, 1.5);

/// The story's camera.
fn camera() -> Camera {
    Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 8.0, 11.0), Vec3::new(0.0, 0.5, 0.0))
}

/// Whether `pixel` is of the red box: red the most, by far.
fn is_red([red, green, blue, _]: [u8; 4]) -> bool {
    u16::from(red) > u16::from(green) * 2 && u16::from(red) > u16::from(blue) * 2
}

/// Waits until the red box is, or isn't, where it comes to rest.
async fn wait_for_red_at_rest(ctx: &Ctx, there: bool, what: &str) -> Result<()> {
    let (x, y) = ctx.spot(&camera(), RED).await?;
    ctx.wait_for(what, async || {
        Ok((is_red(ctx.pixel(x, y).await?) == there).then_some(()))
    })
    .await
}

/// Opens the story, and waits until all the bodies have fallen and are at rest.
async fn open_at_rest(ctx: &Ctx) -> Result<()> {
    ctx.open("/physics").await?;
    ctx.wait_for_text("All at rest.").await?;
    wait_for_red_at_rest(ctx, true, "the red box on the floor").await
}

pub async fn the_bodies_fall_and_come_to_rest(ctx: Ctx) -> Result<()> {
    ctx.open("/physics").await?;
    ctx.wait_for_text("Moving: 7 of 7").await?;
    ctx.wait_for_text("All at rest.").await?;
    wait_for_red_at_rest(&ctx, true, "the red box on the floor").await
}

pub async fn a_click_throws_a_body_up_and_it_comes_to_rest_again(ctx: Ctx) -> Result<()> {
    open_at_rest(&ctx).await?;

    // Its top, about above its middle: up, alone, the others left asleep. A pixel aside, it
    // turns as it flies, so it may come down elsewhere.
    ctx.click_at(&camera(), RED).await?;
    ctx.wait_for_text("Thrown: red").await?;
    ctx.wait_for_text("Moving: 1 of 7").await?;
    wait_for_red_at_rest(&ctx, false, "the red box up in the air").await?;
    ctx.wait_for_text("All at rest.").await
}

pub async fn drop_again_starts_over(ctx: Ctx) -> Result<()> {
    open_at_rest(&ctx).await?;

    ctx.click_button("Drop again").await?;
    ctx.wait_for_text("Moving: 7 of 7").await?;
    ctx.wait_for_text("All at rest.").await?;
    wait_for_red_at_rest(&ctx, true, "the red box on the floor again").await
}
