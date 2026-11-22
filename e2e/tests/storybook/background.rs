//! The "Background" story: a scene and buttons choosing its background.

use vertigo_3d_e2e::{
    env::{Drawing, drawing},
    prelude::*,
};

/// `Color::rgb(0.05, 0.05, 0.15)` of the story, "Night", in bytes.
const NIGHT: [u8; 4] = [13, 13, 38, 255];
/// "Sunset", `Color::rgb(0.98, 0.55, 0.24)`.
const SUNSET: [u8; 4] = [250, 140, 61, 255];

pub async fn scene_shows_its_background(ctx: Ctx) -> Result<()> {
    ctx.open("/background").await?;
    ctx.wait_for_pixel(0.5, 0.5, NIGHT, "the night background")
        .await
}

pub async fn scene_draws_again_when_the_background_changes(ctx: Ctx) -> Result<()> {
    ctx.open("/background").await?;
    ctx.wait_for_pixel(0.5, 0.5, NIGHT, "the night background")
        .await?;

    ctx.click_button("Sunset").await?;
    ctx.wait_for_pixel(0.1, 0.9, SUNSET, "the sunset background, all over")
        .await
}

/// The scene is drawn with WebGPU in a run with it, and with WebGL in one without, as the scene
/// found it has none. Asked once drawn: a canvas's context of the other kind is none.
pub async fn the_scenes_are_drawn_as_the_run_draws(ctx: Ctx) -> Result<()> {
    const KIND: &str = "const canvas = document.querySelector('canvas');
        return canvas.getContext('webgpu') !== null ? 'webgpu'
            : canvas.getContext('webgl2') !== null ? 'webgl' : 'none';";

    ctx.open("/background").await?;
    ctx.wait_for_pixel(0.5, 0.5, NIGHT, "the night background")
        .await?;
    let kind = ctx.js(KIND).await?;
    let expected = match drawing() {
        Drawing::WebGl => "webgl",
        Drawing::WebGpu => "webgpu",
    };
    ensure!(kind.as_str() == Some(expected), "drawn with {kind}");
    Ok(())
}
