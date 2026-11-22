//! The "Picking" story: cubes told of clicks and of the pointer over them, a ball in the way of
//! one.

use vertigo_3d::{Camera, math::Vec3};
use vertigo_3d_e2e::prelude::*;

/// The middle of the cubes' top faces, which the camera sees from above.
const LEFT: Vec3 = Vec3::new(-2.0, 1.0, 0.0);
const MIDDLE: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const RIGHT: Vec3 = Vec3::new(2.0, 1.0, 0.0);
/// The middle of the ball, on the way from the camera to the right cube's middle.
const BALL: Vec3 = Vec3::new(4.0 / 3.0, 1.0, 2.0);
/// On the floor, before the cubes.
const FLOOR: Vec3 = Vec3::new(0.0, 0.0, 3.0);

/// The story's camera.
fn camera() -> Camera {
    Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 2.0, 6.0), Vec3::new(0.0, 0.5, 0.0))
}

async fn hover(ctx: &Ctx, point: Vec3) -> Result<()> {
    ctx.hover_at(&camera(), point).await
}

async fn click(ctx: &Ctx, point: Vec3) -> Result<()> {
    ctx.click_at(&camera(), point).await
}

/// Opens the story, once its scene is drawn.
async fn open(ctx: &Ctx) -> Result<()> {
    ctx.open("/picking").await?;
    ctx.wait_for_text("Over: nothing").await?;
    let (x, y) = ctx.spot(&camera(), MIDDLE).await?;
    ctx.wait_for("the middle cube drawn", async || {
        let [red, green, blue, _] = ctx.pixel(x, y).await?;
        Ok((blue > red && blue > green).then_some(()))
    })
    .await
}

/// Waits until the cube with its top at `point` is picked, green, with the pointer away.
async fn wait_for_picked(ctx: &Ctx, point: Vec3, picked: bool, what: &str) -> Result<()> {
    let (x, y) = ctx.spot(&camera(), point).await?;
    ctx.wait_for(what, async || {
        let [red, green, blue, _] = ctx.pixel(x, y).await?;
        let green_most = green > red && green > blue;
        let blue_most = blue > red && blue > green;
        Ok((if picked { green_most } else { blue_most }).then_some(()))
    })
    .await
}

pub async fn the_pointer_over_a_cube_is_told_and_shown(ctx: Ctx) -> Result<()> {
    open(&ctx).await?;
    let canvas = ctx.find(By::Css("canvas")).await?;

    hover(&ctx, LEFT).await?;
    ctx.wait_for_text("Over: left").await?;
    ensure!(ctx.style(&canvas, "cursor").await? == "pointer");

    // From one cube straight onto another.
    hover(&ctx, MIDDLE).await?;
    ctx.wait_for_text("Over: middle").await?;

    hover(&ctx, FLOOR).await?;
    ctx.wait_for_text("Over: nothing").await?;
    ensure!(ctx.style(&canvas, "cursor").await? != "pointer");
    // Nothing to drag: a finger scrolls the page.
    ensure!(ctx.style(&canvas, "touch-action").await? != "none");
    Ok(())
}

pub async fn a_click_on_a_cube_tells_where(ctx: Ctx) -> Result<()> {
    open(&ctx).await?;

    click(&ctx, MIDDLE).await?;
    ctx.wait_for_text("Clicked: middle at").await?;
    // On its top, a pixel or so from its middle.
    let text = ctx.page_text().await?;
    ensure!(text.contains(", 1.00, "), "{text}");

    hover(&ctx, FLOOR).await?;
    wait_for_picked(&ctx, MIDDLE, true, "the middle cube picked").await?;
    click(&ctx, MIDDLE).await?;
    hover(&ctx, FLOOR).await?;
    wait_for_picked(&ctx, MIDDLE, false, "the middle cube let go").await
}

pub async fn the_ball_in_the_way_is_clicked_not_the_cube_behind(ctx: Ctx) -> Result<()> {
    open(&ctx).await?;

    hover(&ctx, RIGHT).await?;
    ctx.wait_for_text("Over: right").await?;
    hover(&ctx, BALL).await?;
    ctx.wait_for_text("Over: nothing").await?;

    // Had the ball's click gone to the cube, the cube's own would let it go again.
    click(&ctx, BALL).await?;
    click(&ctx, RIGHT).await?;
    ctx.wait_for_text("Clicked: right at").await?;
    hover(&ctx, FLOOR).await?;
    wait_for_picked(&ctx, RIGHT, true, "the right cube picked").await
}

pub async fn dragging_over_a_cube_doesnt_click_it(ctx: Ctx) -> Result<()> {
    open(&ctx).await?;

    let (canvas, x, y) = ctx.aim(&camera(), LEFT).await?;
    ctx.driver
        .action_chain()
        .move_to_element_with_offset(&canvas, x, y)
        .click_and_hold()
        .move_by_offset(20, 0)
        .release()
        .perform()
        .await
        .context("dragging failed")?;

    // A click after it is told, and the drag before it wasn't.
    click(&ctx, MIDDLE).await?;
    ctx.wait_for_text("Clicked: middle at").await?;
    hover(&ctx, FLOOR).await?;
    wait_for_picked(&ctx, LEFT, false, "the left cube not picked").await
}
