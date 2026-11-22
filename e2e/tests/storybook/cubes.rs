//! The "Cubes" story: a scene standing still, drawn again when its objects or its camera change.

use vertigo_3d_e2e::prelude::*;

/// The ball's red, `Color::rgb(0.8, 0.2, 0.2)`.
const RED: [u8; 4] = [204, 51, 51, 255];

pub async fn the_ball_hides_and_shows_again(ctx: Ctx) -> Result<()> {
    ctx.open("/cubes").await?;
    ctx.wait_for_pixel(0.5, 0.5, RED, "the ball in the middle")
        .await?;

    ctx.click_button("Hide the ball").await?;
    ctx.wait_for_pixel_not(0.5, 0.5, RED, "the ball gone")
        .await?;

    ctx.click_button("Show the ball").await?;
    ctx.wait_for_pixel(0.5, 0.5, RED, "the ball back").await
}

pub async fn the_camera_goes_round(ctx: Ctx) -> Result<()> {
    ctx.open("/cubes").await?;
    ctx.wait_for_pixel(0.5, 0.5, RED, "the ball in the middle")
        .await?;
    let before = ctx.picture().await?;

    ctx.click_button("Go right").await?;
    ctx.wait_for("the scene seen from elsewhere", async || {
        Ok((ctx.picture().await? != before).then_some(()))
    })
    .await?;
    // What the camera looks at stays in the middle.
    ctx.wait_for_pixel(0.5, 0.5, RED, "the ball still in the middle")
        .await
}
