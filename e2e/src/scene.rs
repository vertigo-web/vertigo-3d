//! Helpers for scenes: what their canvases show.

use anyhow::{Context, Result};
use thirtyfour::{By, ElementRect, WebElement};
use vertigo_3d::{Camera, math::Vec3};
use vertigo_testing::xpath_literal;

use crate::Ctx;

/// Helpers of the storybook's scenes.
pub trait SceneExt {
    async fn click_button(&self, label: &str) -> Result<()>;
    async fn pixel(&self, x: f64, y: f64) -> Result<[u8; 4]>;
    async fn wait_for_pixel(&self, x: f64, y: f64, rgba: [u8; 4], what: &str) -> Result<()>;
    async fn wait_for_pixel_not(&self, x: f64, y: f64, rgba: [u8; 4], what: &str) -> Result<()>;
    async fn picture(&self) -> Result<String>;
    async fn row_coverage(&self, rgba: [u8; 4]) -> Result<(usize, usize)>;
    async fn canvas_picture(&self) -> Result<Picture>;
    async fn canvas(&self) -> Result<(WebElement, ElementRect)>;
    async fn brightness(&self, x: f64, y: f64) -> Result<u32>;
    async fn pixels_around(&self, x: f64, y: f64, radius: u32) -> Result<Vec<[u8; 4]>>;
    async fn spot(&self, camera: &Camera, point: Vec3) -> Result<(f64, f64)>;
    async fn aim(&self, camera: &Camera, point: Vec3) -> Result<(WebElement, i64, i64)>;
    async fn hover_at(&self, camera: &Camera, point: Vec3) -> Result<()>;
    async fn click_at(&self, camera: &Camera, point: Vec3) -> Result<()>;
}

/// What a canvas shows: its pixels, red, green, blue and alpha, row after row from the top.
pub struct Picture {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl Picture {
    fn from_png(png: &[u8]) -> Result<Self> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(png));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().context("the screenshot isn't a PNG")?;
        let mut bytes = vec![0; reader.output_buffer_size().context("the PNG's size")?];
        let info = reader
            .next_frame(&mut bytes)
            .context("the screenshot isn't a PNG")?;
        bytes.truncate(info.buffer_size());
        let pixels = match info.color_type {
            png::ColorType::Rgba => bytes,
            png::ColorType::Rgb => bytes
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|[red, green, blue]| [*red, *green, *blue, 255])
                .collect(),
            other => anyhow::bail!("a screenshot of {other:?}"),
        };
        Ok(Self {
            width: info.width as usize,
            height: info.height as usize,
            pixels,
        })
    }

    /// The pixel at `x` and `y` in pixels from the top left corner.
    pub fn pixel(&self, x: usize, y: usize) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = (y * self.width + x) * 4;
        self.pixels.get(at..at + 4)?.try_into().ok()
    }

    /// Where `x` and `y` of the width and the height are, in pixels.
    fn place(&self, x: f64, y: f64) -> (usize, usize) {
        let along = |part: f64, length: usize| {
            ((part * length as f64).floor().max(0.0) as usize).min(length.saturating_sub(1))
        };
        (along(x, self.width), along(y, self.height))
    }

    /// The pixel at `x` and `y` of the width and the height.
    pub fn at(&self, x: f64, y: f64) -> Option<[u8; 4]> {
        let (x, y) = self.place(x, y);
        self.pixel(x, y)
    }
}

/// Whether two colors are the same, give or take 2 of each part, for rounding.
pub fn close(got: [u8; 4], want: [u8; 4]) -> bool {
    got.iter()
        .zip(want)
        .all(|(got, want)| got.abs_diff(want) <= 2)
}

/// Where `point` is on a canvas of `aspect`, from 0 to 1 across it and down it, as `camera` sees
/// it.
pub fn on_canvas(camera: &Camera, point: Vec3, aspect: f32) -> (f64, f64) {
    let at = camera.view_projection(aspect).project_point3(point);
    (f64::from(at.x + 1.0) / 2.0, f64::from(1.0 - at.y) / 2.0)
}

impl SceneExt for Ctx {
    /// Clicks the `<button>` with exactly this text.
    async fn click_button(&self, label: &str) -> Result<()> {
        self.find(By::XPath(format!(
            "//button[normalize-space(.)={}]",
            xpath_literal(label)
        )))
        .await?
        .click()
        .await?;
        Ok(())
    }

    /// The pixel of the first canvas on the page at `x` and `y` of its width and height, as red,
    /// green, blue and alpha: as the page shows it, see [`Self::canvas_picture`].
    async fn pixel(&self, x: f64, y: f64) -> Result<[u8; 4]> {
        let picture = self.canvas_picture().await?;
        picture.at(x, y).context("the canvas has no pixel there")
    }

    /// Waits until the pixel at `x` and `y` (see [`Self::pixel`]) is `rgba`, give or take 2 of
    /// each part, for rounding.
    async fn wait_for_pixel(&self, x: f64, y: f64, rgba: [u8; 4], what: &str) -> Result<()> {
        self.wait_for(what, async || {
            Ok(close(self.pixel(x, y).await?, rgba).then_some(()))
        })
        .await
    }

    /// Waits until the pixel at `x` and `y` (see [`Self::pixel`]) is anything but `rgba`.
    async fn wait_for_pixel_not(&self, x: f64, y: f64, rgba: [u8; 4], what: &str) -> Result<()> {
        self.wait_for(what, async || {
            Ok((!close(self.pixel(x, y).await?, rgba)).then_some(()))
        })
        .await
    }

    /// All the first canvas shows, as text: to tell when it changes.
    async fn picture(&self) -> Result<String> {
        let picture = self.canvas_picture().await?;
        // FNV-1a of the pixels
        let hash = picture
            .pixels
            .iter()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
            });
        Ok(format!(
            "{} x {}: {hash:016x}",
            picture.width, picture.height
        ))
    }

    /// How light the pixel at `x` and `y` is (see [`Self::pixel`]): its red, green and blue
    /// summed.
    async fn brightness(&self, x: f64, y: f64) -> Result<u32> {
        let [red, green, blue, _] = self.pixel(x, y).await?;
        Ok(u32::from(red) + u32::from(green) + u32::from(blue))
    }

    /// How many pixels of the first canvas's middle row are `rgba`, give or take 2 of each
    /// part, and how many it has: how wide what has that color is, there.
    async fn row_coverage(&self, rgba: [u8; 4]) -> Result<(usize, usize)> {
        let picture = self.canvas_picture().await?;
        let row = picture.height / 2;
        let count = (0..picture.width)
            .filter_map(|x| picture.pixel(x, row))
            .filter(|pixel| close(*pixel, rgba))
            .count();
        Ok((count, picture.width))
    }

    /// The pixels of the first canvas at most `radius` from `x` and `y` of its width and height
    /// (see [`Self::pixel`]) across and down: f. ex. to find a line a pixel thick there.
    async fn pixels_around(&self, x: f64, y: f64, radius: u32) -> Result<Vec<[u8; 4]>> {
        let picture = self.canvas_picture().await?;
        let radius = radius as usize;
        let (middle_x, middle_y) = picture.place(x, y);
        let mut pixels = Vec::new();
        for y in middle_y.saturating_sub(radius)..=middle_y + radius {
            for x in middle_x.saturating_sub(radius)..=middle_x + radius {
                pixels.extend(picture.pixel(x, y));
            }
        }
        Ok(pixels)
    }

    /// The first canvas on the page, and where it is.
    async fn canvas(&self) -> Result<(WebElement, ElementRect)> {
        // A scene trying WebGPU in a browser without it makes its canvas anew for WebGL, soon
        // after the page opens: one let go meanwhile is found again.
        self.wait_for("the canvas", async || {
            let canvas = self.find(By::Css("canvas")).await?;
            let rect = canvas.rect().await?;
            Ok(Some((canvas, rect)))
        })
        .await
    }

    /// What the first canvas shows on the page, as the browser shows it: a screenshot of it.
    /// Read back from the canvas itself, a picture of WebGPU is gone once it's shown, and asking
    /// the canvas for a WebGL context would make one, if the scene hadn't yet.
    async fn canvas_picture(&self) -> Result<Picture> {
        let canvas = self.find(By::Css("canvas")).await?;
        let png = canvas
            .screenshot_as_png()
            .await
            .context("the screenshot of the canvas")?;
        Picture::from_png(&png)
    }

    /// Where `point` is on the first canvas, from 0 to 1 across it and down it, as `camera`
    /// sees it: for [`Self::pixel`].
    async fn spot(&self, camera: &Camera, point: Vec3) -> Result<(f64, f64)> {
        let (_, rect) = self.canvas().await?;
        Ok(on_canvas(camera, point, (rect.width / rect.height) as f32))
    }

    /// The first canvas, and where `point` is on it as `camera` sees it, in pixels from its
    /// middle: where a pointer moves to it from the canvas.
    async fn aim(&self, camera: &Camera, point: Vec3) -> Result<(WebElement, i64, i64)> {
        let (canvas, rect) = self.canvas().await?;
        let (x, y) = on_canvas(camera, point, (rect.width / rect.height) as f32);
        Ok((
            canvas,
            ((x - 0.5) * rect.width).round() as i64,
            ((y - 0.5) * rect.height).round() as i64,
        ))
    }

    /// Moves the mouse over `point` of the first canvas, as `camera` sees it.
    async fn hover_at(&self, camera: &Camera, point: Vec3) -> Result<()> {
        let (canvas, x, y) = self.aim(camera, point).await?;
        self.driver
            .action_chain()
            .move_to_element_with_offset(&canvas, x, y)
            .perform()
            .await
            .context("moving the pointer failed")
    }

    /// Clicks `point` of the first canvas, as `camera` sees it.
    async fn click_at(&self, camera: &Camera, point: Vec3) -> Result<()> {
        let (canvas, x, y) = self.aim(camera, point).await?;
        self.driver
            .action_chain()
            .move_to_element_with_offset(&canvas, x, y)
            .click()
            .perform()
            .await
            .context("clicking failed")
    }
}
