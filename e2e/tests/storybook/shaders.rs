//! The "Shaders" story: a ball with code of its own in its shaders, going on with the frames.

use vertigo_3d_e2e::prelude::*;

/// The middle of the ball, and its edge on the right: the ball is 2 across, seen from 5 away.
const MIDDLE: (f64, f64) = (0.5, 0.5);
const EDGE: (f64, f64) = (0.5 + 0.11, 0.5);

/// Two pictures, 300 ms apart, waited for in the browser.
async fn two_pictures(ctx: &Ctx) -> Result<(String, String)> {
    let first = ctx.picture().await?;
    ctx.js("return new Promise((done) => setTimeout(done, 300));")
        .await?;
    Ok((first, ctx.picture().await?))
}

pub async fn a_ball_without_code_of_its_own_stands_still(ctx: Ctx) -> Result<()> {
    ctx.open("/shaders").await?;
    ctx.wait_for("the ball drawn", async || {
        Ok((ctx.brightness(MIDDLE.0, MIDDLE.1).await? > 300).then_some(()))
    })
    .await?;
    let (first, second) = two_pictures(&ctx).await?;
    ensure!(first == second, "the plain ball changed with the frames");
    Ok(())
}

/// The ripples move the ball's outline, so its edge is sometimes the ball and sometimes the
/// background.
pub async fn the_ripples_of_a_vertex_shader_move_the_outline(ctx: Ctx) -> Result<()> {
    ctx.open("/shaders").await?;
    ctx.click_button("Rippling").await?;
    let (first, second) = two_pictures(&ctx).await?;
    ensure!(first != second, "the rippling ball stood still");

    let mut seen = Vec::new();
    ctx.wait_for("the edge moving", async || {
        seen.push(ctx.brightness(EDGE.0, EDGE.1).await?);
        let (least, most) = (seen.iter().min(), seen.iter().max());
        Ok(least
            .zip(most)
            .filter(|(least, most)| **most > **least + 60)
            .map(|_| ()))
    })
    .await
}

/// The rainbow colors the middle of the ball, its red, green and blue apart, and in other ones
/// with the time.
pub async fn a_fragment_shader_colors_the_ball_with_the_time(ctx: Ctx) -> Result<()> {
    ctx.open("/shaders").await?;
    ctx.click_button("Rainbow").await?;

    let mut seen: Vec<[u8; 4]> = Vec::new();
    ctx.wait_for("the colors changing", async || {
        let pixel = ctx.pixel(MIDDLE.0, MIDDLE.1).await?;
        seen.push(pixel);
        let colorful = seen.iter().any(|[red, green, blue, _]| {
            let (least, most) = ((*red).min(*green).min(*blue), (*red).max(*green).max(*blue));
            most > least + 60
        });
        let changing = seen.iter().any(|other| other != &pixel);
        Ok((colorful && changing).then_some(()))
    })
    .await
}
