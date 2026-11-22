//! What one suite runs against: the storybook, built from the current workspace and served
//! by vertigo-cli in this process.

use std::ops::Deref;

use anyhow::Result;
use vertigo_testing::{ChromeConfig, Core, Env, Settings, build, serve::Server};

pub const STORYBOOK: &str = "vertigo-3d-storybook";

/// What draws the scenes in a run of the suite, as `E2E_BACKEND` says: `webgpu`, or WebGL, in a
/// Chrome without WebGPU, the scenes finding it has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drawing {
    WebGl,
    WebGpu,
}

pub fn drawing() -> Drawing {
    match std::env::var("E2E_BACKEND").as_deref() {
        Ok("webgpu") => Drawing::WebGpu,
        _ => Drawing::WebGl,
    }
}

/// The name of the suite of the run, of its build and its artifacts.
pub fn suite() -> &'static str {
    match drawing() {
        Drawing::WebGl => "storybook",
        Drawing::WebGpu => "storybook-webgpu",
    }
}

pub struct TestEnv {
    core: Core,
    server: Server,
}

impl Env for TestEnv {
    async fn start(suite: &'static str) -> Result<Self> {
        let settings = Settings::from_env(env!("CARGO_MANIFEST_DIR"))?;

        let build_dir = settings.build_dir().join("build");
        build::vertigo_app(&settings, STORYBOOK, &build_dir).await?;

        let server = Server::start(&build_dir).await?;
        println!("Storybook served at {}", server.base_url);

        // WebGL on a machine without a GPU: Chrome no longer falls back to drawing it in
        // software by itself. Safe here, as the suite opens only the storybook. WebGPU too, on
        // Vulkan of SwiftShader, with WebGL on ANGLE of SwiftShader: with other renderers Chrome
        // destroys the device once it shows a picture.
        let args: &[&str] = match drawing() {
            Drawing::WebGl => &["--enable-unsafe-swiftshader"],
            Drawing::WebGpu => &[
                "--enable-unsafe-swiftshader",
                "--enable-unsafe-webgpu",
                "--enable-features=Vulkan",
                "--use-vulkan=swiftshader",
                "--use-angle=swiftshader",
            ],
        };
        let chrome = ChromeConfig {
            window_size: (1600, 1200),
            lang: "en-US",
            args,
        };
        let core = Core::new(suite, settings, server.base_url.clone(), chrome)?;

        Ok(Self { core, server })
    }

    async fn shutdown(&self) {
        self.server.stop().await;
    }
}

impl Deref for TestEnv {
    type Target = Core;

    fn deref(&self) -> &Core {
        &self.core
    }
}
