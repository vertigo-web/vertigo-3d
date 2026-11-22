set ignore-comments

# just list
default:
    @just --list --unsorted

# Everything CI checks: formatting, clippy, unit tests and docs
[group('ci')]
check: fmt-check lint test docs

# Check formatting, without changing files
[group('ci')]
fmt-check:
    cargo fmt -- --check

# Clippy as in CI, with warnings as errors, for the host and the browser
[group('ci')]
lint:
    cargo clippy --all --all-features --tests --locked -- -D warnings
    cargo clippy --all --all-features --tests --locked --target wasm32-unknown-unknown --exclude vertigo-3d-e2e -- -D warnings

# Unit and doc tests (all but e2e), f. ex. `just test color`
[group('ci')]
[positional-arguments]
test *args:
    cargo test --all --all-features --locked --exclude vertigo-3d-e2e -- "$@"

# Docs, with broken links as errors
[group('ci')]
docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked -p vertigo-3d

# Storybook tested in Chrome, drawn with WebGL and then with WebGPU (or as E2E_BACKEND says), f. ex. `just e2e --headed background`
[group('e2e')]
[positional-arguments]
[arg('E2E_HEADLESS', long='headed', value='0', help="Show the browser window, best for a few tests (E2E_HEADLESS=0)")]
[arg('E2E_RELEASE', long='release', value='1', help="Build the storybook in release mode (E2E_RELEASE=1)")]
[arg('E2E_WASM_OPT', long='wasm-opt', value='1', help="Run wasm-opt on the storybook, as a shipped build has it (E2E_WASM_OPT=1)")]
[arg('E2E_SKIP_BUILD', long='skip-build', value='1', help="Use the last build (E2E_SKIP_BUILD=1)")]
e2e $E2E_HEADLESS=env('E2E_HEADLESS', '1') $E2E_RELEASE=env('E2E_RELEASE', '') $E2E_WASM_OPT=env('E2E_WASM_OPT', '') $E2E_SKIP_BUILD=env('E2E_SKIP_BUILD', '') *args:
    # The options are exported as the E2E_* variables the harness reads, an empty one counts
    # as off. They are also the first four positional arguments, so shifted out of "$@".
    shift 4; if [ -n "${E2E_BACKEND:-}" ]; then cargo test -p vertigo-3d-e2e -- "$@"; else E2E_BACKEND=webgl cargo test -p vertigo-3d-e2e -- "$@" && E2E_BACKEND=webgpu cargo test -p vertigo-3d-e2e -- "$@"; fi

# The storybook on http://localhost:4444, rebuilt on change, by vertigo-cli of the vertigo commit Cargo.lock pins, installed in target/vertigo-cli (see README)
storybook:
    rev=$(sed -n 's|^source = "git+https://github.com/vertigo-web/vertigo?branch=master#\([0-9a-f]*\)"$|\1|p' Cargo.lock | head -n 1) && \
    cargo install --locked --git https://github.com/vertigo-web/vertigo --rev "$rev" --root target/vertigo-cli vertigo-cli && \
    target/vertigo-cli/bin/vertigo watch vertigo-3d-storybook

# CI checks and e2e tests
all: check e2e
