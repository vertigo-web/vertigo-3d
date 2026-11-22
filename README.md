# Vertigo 3D

Declarative 3D scenes in [vertigo](https://crates.io/crates/vertigo), drawn with WebGPU, or with
WebGL 2 where the browser has no WebGPU.

[![crates.io](https://img.shields.io/crates/v/vertigo-3d)](https://crates.io/crates/vertigo-3d)
[![Documentation](https://docs.rs/vertigo-3d/badge.svg)](https://docs.rs/vertigo-3d)
![MIT or Apache 2.0 licensed](https://img.shields.io/crates/l/vertigo-3d.svg)
[![CI](https://github.com/vertigo-web/vertigo-3d/actions/workflows/pipeline.yaml/badge.svg)](https://github.com/vertigo-web/vertigo-3d/actions/workflows/pipeline.yaml)

See [Changelog](https://github.com/vertigo-web/vertigo-3d/blob/master/CHANGES.md) for recent features.

A scene is told by values: its camera, its background, the objects in it, where they are and
what they look like. It is drawn again when one of them changes, not before each frame, so a
scene standing still costs nothing. The objects of a mesh are drawn together, with one call, so
thousands of them cost little more than one. The scene draws through
[`vertigo::webgpu`](https://docs.rs/vertigo/latest/vertigo/webgpu/index.html) or
[`vertigo::webgl`](https://docs.rs/vertigo/latest/vertigo/webgl/index.html), which are there for
whatever it doesn't do, and alike with both (`SceneParams::backend`, `Backend`).

Objects are one color all over, or lit (`Material::lit`) by the scene's ambient, directional and
point lights, with a soft default set of them, and shine with highlights (`Material::specular`). A
`Texture` - an image file the browser loads, ahead of time if asked (`SceneParams::preload`) and
told when it's in (`SceneParams::on_load`), or pixels made in Rust - goes on them by the meshes'
texture coordinates, repeated if asked, with holes where it's transparent, and normal and height
maps give them a relief (`Material::normal_map`, `Material::height_map`). Objects of a color with
alpha below 1 show what is behind them, drawn after the others, the farthest first. An object is
told of clicks on it, with the point clicked, of the mouse over it, and of dragging it with the
mouse, a finger or a pen (`Object::on_click`, `Object::on_hover`, `Object::on_drag`), where nothing
nearer the camera is in the way.

A `Shader` puts code of its own into the shaders drawing a material, moving its corners or
coloring its pixels with the scene's time, in GLSL for WebGL and WGSL for WebGPU. A slow computer may draw a scene in fewer pixels,
without smoothing its edges or without relief (`SceneParams::bitmap_scale`, `antialias`,
`relief`). What objects can't be - lines, points, drawing of its
own - a `Layer` draws with `vertigo::webgl` or `vertigo::webgpu` in the scene, with its camera,
hidden by the objects before it and seen through glass.

## Example

Dependencies:

```toml
vertigo = "0.14"
vertigo-3d = "0.1"
```

A cube turning, lit by the scene's default light:

```rust
use vertigo::{DomNode, Value, dom, main};
use vertigo_3d::{
    Camera, Color, Material, Mesh, Object, Scene, SceneParams, Transform, frame_time,
    math::{Quat, Vec3},
};

#[main]
fn render() -> DomNode {
    let turning = frame_time().map(|time| Transform::from_rotation(Quat::from_rotation_y(time)));
    let cube = Object::new(Mesh::cube())
        .transform(turning)
        .material(Material::lit(Color::rgb(0.8, 0.2, 0.2)));
    let camera = Value::new(
        Camera::perspective(45_f32.to_radians()).looking_at(Vec3::new(0.0, 2.0, 4.0), Vec3::ZERO),
    );
    let objects = Value::new(vec![cube]);

    dom! {
        <html>
            <head />
            <body>
                <div style="width: 640px; height: 400px">
                    <Scene
                        camera={camera.to_computed()}
                        objects={objects.to_computed()}
                        params={SceneParams::default()}
                    />
                </div>
            </body>
        </html>
    }
}
```

## Development

`vertigo::webgl` and `vertigo::webgpu` aren't released yet: until they are, the workspace takes
vertigo from its [master branch](https://github.com/vertigo-web/vertigo/tree/master) (see
`[patch.crates-io]` in `Cargo.toml`), at the commit `Cargo.lock` pins. No local checkout of
vertigo is needed. `cargo update -p vertigo` moves vertigo, vertigo-cli and vertigo-testing to the
branch's latest commit.

- `just check` - what CI checks: formatting, clippy for the host and the browser, tests, docs
- `just e2e` - the storybook tested in Chrome, drawn with WebGL and then with WebGPU, see
  [e2e/README.md](e2e/README.md)
- `just storybook` - the storybook on <http://localhost:4444>, served by vertigo-cli of that
  commit, which it installs in `target/vertigo-cli` the first time (a released vertigo-cli 0.14
  doesn't copy the WebGL code to the build)
