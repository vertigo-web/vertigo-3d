# vertigo-3d e2e

Browser tests of the storybook (`storybook/`), story by story. They drive Chrome through
[thirtyfour](https://docs.rs/thirtyfour) against the storybook built from the current
workspace, so they check the whole path: server-side rendering, the start of the WASM app and
what the scenes draw.

## Requirements

- Google Chrome. On the first run thirtyfour downloads the matching chromedriver to
  `~/.cache/thirtyfour`. WebGL runs on the GPU, or in software without one
  (`--enable-unsafe-swiftshader`, see `src/env.rs`).
- The same toolchain as the rest of the workspace, with the `wasm32-unknown-unknown` target.

`vertigo-cli` doesn't have to be installed: the suite builds and serves the storybook with the
vertigo-cli library.

## Running

```sh
cargo test -p vertigo-3d-e2e                       # all tests
cargo test -p vertigo-3d-e2e -- background         # only tests with "background" in the name
cargo test -p vertigo-3d-e2e -- --include-ignored  # with the tests of known bugs
```

The `Justfile` in the workspace root has it shorter: `just e2e`, which takes the same arguments
(`just e2e background`), and `just e2e --headed`, which shows the browser.

Plain `cargo test` in the workspace root runs only the library's tests, and CI excludes this
crate from its test run. Use `cargo test`, not nextest: nextest runs each test in a separate
process, and the suite builds and serves the storybook once for all of them.

When a test fails, a screenshot, the page source, the console log and the error land in
`target/e2e/storybook/artifacts/<test>/`. A test also fails when the browser console reports an
error (a panic in WASM, for example), unless the test allows it with `ctx.allow_console(...)`.

## Environment variables

| Variable | Effect |
|---|---|
| `E2E_HEADLESS=0` | Shows the browser window |
| `E2E_SKIP_BUILD=1` | Uses the last build |
| `E2E_RELEASE=1` | Builds the storybook in release mode |
| `E2E_CHROMEDRIVER` | Path to chromedriver to use instead of downloading it |
| `E2E_BACKEND=webgpu` | Draws with WebGPU, in a Chrome with it on SwiftShader's Vulkan; WebGL in one without by default, as the scenes find it has none. `just e2e` runs both but when it is set |

## Writing tests

- Tests of a story are in `tests/storybook/<story>.rs`, named like the story's module in
  `storybook/src/`. A test is `async fn name(ctx: Ctx) -> Result<()>`, registered in
  `tests/storybook/main.rs` with `browser_tests!`.
- `ctx.open(path)` waits until the app takes over the page rendered by the server.
- What a scene draws is read with `ctx.pixel(x, y)` and `ctx.wait_for_pixel(...)`, at a
  fraction of the canvas's width and height. They read a screenshot of the canvas, as the page
  shows it: the browser lets go of a WebGPU picture once it's shown, and asking the canvas for
  its WebGL context would make one, if the scene hadn't yet. The artifacts of a run with WebGPU
  are in `target/e2e/storybook-webgpu/`.
- A test of a known, not yet fixed bug describes the correct behavior and goes to
  `known_failures(...)` in `main.rs`. It's skipped by default and runs with
  `--include-ignored`.
