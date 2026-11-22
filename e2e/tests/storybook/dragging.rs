//! The "Dragging" story: boxes on a floor walled round, dragged along it.

use vertigo_3d::{Camera, math::Vec3};
use vertigo_3d_e2e::prelude::*;

/// The middle of the red box's top, 1 across, where it starts.
const RED: Vec3 = Vec3::new(-2.0, 1.0, -1.0);
/// On the floor, before the boxes.
const FLOOR: Vec3 = Vec3::new(0.0, 0.0, 4.0);

/// The story's camera.
fn camera() -> Camera {
    Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 9.0, 9.0), Vec3::new(0.0, 0.0, 0.5))
}

/// Whether `pixel` is of the red box: red the most, by far.
fn is_red([red, green, blue, _]: [u8; 4]) -> bool {
    u16::from(red) > u16::from(green) * 2 && u16::from(red) > u16::from(blue) * 2
}

/// Opens the story, once its scene is drawn.
async fn open(ctx: &Ctx) -> Result<()> {
    ctx.open("/dragging").await?;
    ctx.wait_for_text("Drag the boxes").await?;
    let (x, y) = ctx.spot(&camera(), RED).await?;
    ctx.wait_for("the red box drawn", async || {
        Ok(is_red(ctx.pixel(x, y).await?).then_some(()))
    })
    .await
}

/// Drags the red box by its top, the pointer moved by each of `moves` in pixels, and waits for
/// where it went.
async fn drag_red(ctx: &Ctx, moves: &[(i64, i64)]) -> Result<Vec3> {
    let (canvas, from_x, from_y) = ctx.aim(&camera(), RED).await?;
    let pressed = ctx
        .driver
        .action_chain()
        .move_to_element_with_offset(&canvas, from_x, from_y)
        .click_and_hold();
    moves
        .iter()
        .fold(pressed, |chain, (x, y)| chain.move_by_offset(*x, *y))
        .release()
        .perform()
        .await
        .context("dragging failed")?;

    ctx.wait_for_text("Moved: red to").await?;
    let text = ctx.page_text().await?;
    let to: Vec<f32> = text
        .split_once("Moved: red to (")
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(to, _)| {
            to.split(", ")
                .filter_map(|part| part.parse().ok())
                .collect()
        })
        .unwrap_or_default();
    match to[..] {
        [x, z] => Ok(Vec3::new(x, 1.0, z)),
        _ => bail!("no place in {text:?}"),
    }
}

/// Whether the red box's top at `top` is under the pointer moved `x` and `y` pixels from where it
/// grabbed it.
async fn ensure_under_the_pointer(ctx: &Ctx, top: Vec3, x: i64, y: i64) -> Result<()> {
    let (_, from_x, from_y) = ctx.aim(&camera(), RED).await?;
    let (_, to_x, to_y) = ctx.aim(&camera(), top).await?;
    let (pointer_x, pointer_y) = (from_x + x, from_y + y);
    ensure!(
        (to_x - pointer_x).abs() <= 2 && (to_y - pointer_y).abs() <= 2,
        "the top at {to_x}, {to_y}, the pointer at {pointer_x}, {pointer_y}"
    );
    Ok(())
}

pub async fn a_box_goes_with_the_pointer_along_the_floor(ctx: Ctx) -> Result<()> {
    open(&ctx).await?;

    let top = drag_red(&ctx, &[(20, 0), (20, 0), (20, 0)]).await?;
    // Sideways on the screen is sideways on the floor, the camera looking along it.
    ensure!(top.x > RED.x + 0.5 && (top.z - RED.z).abs() < 0.02, "{top}");
    // The point grabbed stays under the pointer.
    ensure_under_the_pointer(&ctx, top, 60, 0).await?;

    let (x, y) = ctx.spot(&camera(), top).await?;
    ensure!(
        is_red(ctx.pixel(x, y).await?),
        "the box drawn where it went"
    );
    let (x, y) = ctx.spot(&camera(), RED).await?;
    ensure!(!is_red(ctx.pixel(x, y).await?), "and not where it was");
    Ok(())
}

pub async fn a_box_stops_at_the_wall(ctx: Ctx) -> Result<()> {
    open(&ctx).await?;

    // Far past the floor, which ends inside the canvas, to 20 px from its right edge.
    let (canvas, from_x, _) = ctx.aim(&camera(), RED).await?;
    let step = (canvas.rect().await?.width / 2.0) as i64 - 20 - from_x;
    let top = drag_red(
        &ctx,
        &[(step / 3, 0), (step / 3, 0), (step - 2 * (step / 3), 0)],
    )
    .await?;
    ensure!(
        top.x == 4.5 && top.z == -1.0,
        "against the right wall: {top}"
    );
    Ok(())
}

pub async fn a_drag_goes_on_outside_the_canvas(ctx: Ctx) -> Result<()> {
    open(&ctx).await?;

    // Out past the right edge of the canvas, and back in a little right of where it started: the
    // canvas holds the pointer while it drags.
    let (canvas, from_x, _) = ctx.aim(&camera(), RED).await?;
    let out = (canvas.rect().await?.width / 2.0) as i64 + 40 - from_x;
    let top = drag_red(&ctx, &[(20, 0), (out - 20, 0), (40 - out, 30)]).await?;
    ensure_under_the_pointer(&ctx, top, 40, 30).await
}

pub async fn the_pointer_shows_a_box_can_be_grabbed(ctx: Ctx) -> Result<()> {
    open(&ctx).await?;
    let canvas = ctx.find(By::Css("canvas")).await?;
    // A finger on it drags the boxes, not the page.
    ensure!(ctx.style(&canvas, "touch-action").await? == "none");

    ctx.hover_at(&camera(), RED).await?;
    ctx.wait_for("a hand to grab with", async || {
        Ok((ctx.style(&canvas, "cursor").await? == "grab").then_some(()))
    })
    .await?;

    ctx.driver
        .action_chain()
        .click_and_hold()
        .move_by_offset(10, 0)
        .move_by_offset(10, 0)
        .perform()
        .await
        .context("starting a drag failed")?;
    ctx.wait_for("a hand holding", async || {
        Ok((ctx.style(&canvas, "cursor").await? == "grabbing").then_some(()))
    })
    .await?;

    ctx.driver
        .action_chain()
        .release()
        .perform()
        .await
        .context("ending the drag failed")?;
    ctx.wait_for_text("Moved: red to").await?;
    ctx.wait_for("a hand to grab with again", async || {
        Ok((ctx.style(&canvas, "cursor").await? == "grab").then_some(()))
    })
    .await?;

    ctx.hover_at(&camera(), FLOOR).await?;
    ctx.wait_for("the cursor as it was", async || {
        Ok((ctx.style(&canvas, "cursor").await? == "auto").then_some(()))
    })
    .await
}
