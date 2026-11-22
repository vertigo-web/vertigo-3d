//! The "Layers" story: a grid of lines and a fountain of points, drawn with WebGL of their own.

use vertigo_3d::{Camera, math::Vec3};
use vertigo_3d_e2e::{
    env::{Drawing, drawing},
    prelude::*,
};

fn camera() -> Camera {
    Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 4.0, 8.0), Vec3::new(0.0, 0.5, 0.0))
}

/// A grey line of the grid.
fn is_grid([red, green, blue, _]: [u8; 4]) -> bool {
    (100..=130).contains(&red) && red == green && green == blue
}

/// The red line of the X axis.
fn is_axis([red, green, blue, _]: [u8; 4]) -> bool {
    red > 230 && green < 70 && blue < 70
}

/// The pane of glass before the background, 0.4 of its color over it.
fn is_glass([red, green, blue, _]: [u8; 4]) -> bool {
    red < 60 && (40..=80).contains(&green) && (95..=140).contains(&blue)
}

/// Whether a pixel within 2 of where `point` is on the canvas is such.
async fn seen(ctx: &Ctx, point: Vec3, is: fn([u8; 4]) -> bool) -> Result<bool> {
    let (x, y) = ctx.spot(&camera(), point).await?;
    Ok(ctx.pixels_around(x, y, 2).await?.into_iter().any(is))
}

async fn wait_until(
    ctx: &Ctx,
    what: &str,
    point: Vec3,
    is: fn([u8; 4]) -> bool,
    shown: bool,
) -> Result<()> {
    ctx.wait_for(what, async || {
        Ok((seen(ctx, point, is).await? == shown).then_some(()))
    })
    .await
}

/// On the lines at x = 1 and 2, between those at z = 2 and 3.
const ONE: Vec3 = Vec3::new(1.0, 0.0, 2.5);
const TWO: Vec3 = Vec3::new(2.0, 0.0, 2.5);

pub async fn the_grid_has_the_spacing_chosen(ctx: Ctx) -> Result<()> {
    ctx.open("/layers").await?;
    wait_until(&ctx, "a line every 1", ONE, is_grid, true).await?;

    ctx.click_button("Coarse").await?;
    wait_until(&ctx, "no line at 1", ONE, is_grid, false).await?;
    ensure!(seen(&ctx, TWO, is_grid).await?, "a line every 2");
    Ok(())
}

pub async fn the_cube_hides_the_lines_behind_it(ctx: Ctx) -> Result<()> {
    ctx.open("/layers").await?;
    wait_until(
        &ctx,
        "the X axis beside the cube",
        Vec3::new(-2.0, 0.0, 0.0),
        is_axis,
        true,
    )
    .await?;
    // Under the cube, the depth of its front face hides it.
    ensure!(
        !seen(&ctx, Vec3::new(0.3, 0.0, 0.0), is_axis).await?,
        "the X axis under the cube"
    );
    Ok(())
}

pub async fn the_fountain_goes_on_with_the_frames(ctx: Ctx) -> Result<()> {
    ctx.open("/layers").await?;
    wait_until(&ctx, "the grid", ONE, is_grid, true).await?;
    let before = ctx.picture().await?;
    ctx.wait_for("the drops moved", async || {
        Ok((ctx.picture().await? != before).then_some(()))
    })
    .await
}

pub async fn the_glass_blends_its_own_way_after_the_fountain(ctx: Ctx) -> Result<()> {
    ctx.open("/layers").await?;
    // Near its top corner, with no drop behind: the fountain adds what it draws to what is there,
    // and the scene blends by alpha again after it.
    wait_until(
        &ctx,
        "the glass",
        Vec3::new(3.3, 1.9, 0.525),
        is_glass,
        true,
    )
    .await
}

/// WebGL's context lost and given back, or WebGPU's device lost and another one given: the
/// layers are made again on it.
pub async fn a_context_given_back_has_its_layers_made_again(ctx: Ctx) -> Result<()> {
    ctx.open("/layers").await?;
    wait_until(&ctx, "the grid", ONE, is_grid, true).await?;

    match drawing() {
        Drawing::WebGl => {
            ctx.js(
                "window.lose = document.querySelector('canvas').getContext('webgl2')
                        .getExtension('WEBGL_lose_context');
                    window.lose.loseContext();",
            )
            .await?;
            wait_until(&ctx, "the grid gone with the context", ONE, is_grid, false).await?;
            ctx.js("window.lose.restoreContext();").await?;
        }
        Drawing::WebGpu => {
            ctx.js(
                "window.lost = document.querySelector('canvas').getContext('webgpu')
                        .getConfiguration().device;
                    window.lost.destroy();",
            )
            .await?;
            ctx.wait_for("another device", async || {
                let other = ctx
                    .js(
                        "return document.querySelector('canvas').getContext('webgpu')
                            .getConfiguration()?.device !== window.lost;",
                    )
                    .await?;
                Ok((other.as_bool() == Some(true)).then_some(()))
            })
            .await?;
        }
    }
    wait_until(&ctx, "the grid back", ONE, is_grid, true).await
}
