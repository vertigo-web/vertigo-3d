//! The "Spinning" story: a lit cube turned by `frame_time`.

use vertigo_3d_e2e::prelude::*;

/// The background, `Color::rgb(0.05, 0.05, 0.15)`.
const NIGHT: [u8; 4] = [13, 13, 38, 255];

pub async fn the_cube_turns(ctx: Ctx) -> Result<()> {
    ctx.open("/spinning").await?;

    // How wide the cube is across the middle: the pixels of the row that aren't the background.
    let width = async || -> Result<usize> {
        let (night, all) = ctx.row_coverage(NIGHT).await?;
        Ok(all.saturating_sub(night))
    };

    let mut first = 0;
    ctx.wait_for("the cube across the middle", async || {
        first = width().await?;
        Ok((first > 0).then_some(()))
    })
    .await?;

    // Turning, it's wider and narrower across.
    ctx.wait_for("the cube turned", async || {
        let now = width().await?;
        Ok((now > 0 && now.abs_diff(first) >= 4).then_some(()))
    })
    .await
}
