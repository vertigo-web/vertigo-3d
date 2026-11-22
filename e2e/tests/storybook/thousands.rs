//! The "Thousands" story: 10000 cubes of one mesh, the camera going round them.

use vertigo_3d_e2e::prelude::*;

/// The background, `Color::rgb(0.05, 0.05, 0.15)`.
const NIGHT: [u8; 4] = [13, 13, 38, 255];

pub async fn the_cubes_are_drawn_and_seen_going_round(ctx: Ctx) -> Result<()> {
    ctx.open("/thousands").await?;
    ctx.wait_for_pixel_not(0.5, 0.5, NIGHT, "cubes in the middle")
        .await?;

    let before = ctx.picture().await?;
    ctx.wait_for("the camera going round", async || {
        Ok((ctx.picture().await? != before).then_some(()))
    })
    .await
}
