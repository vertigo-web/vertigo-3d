//! Declarative 3D scenes in [vertigo](https://crates.io/crates/vertigo), drawn with WebGPU through
//! [`vertigo::webgpu`], or with WebGL 2 through [`vertigo::webgl`] where the browser has no WebGPU,
//! see [`Backend`].
//!
//! A scene is told by values: its background, its camera, the objects in it and where they are.
//! It is drawn again when one of them changes, not before each frame, so a scene standing still
//! costs nothing. A value changing with each frame, f. ex. a turn set in
//! [`Driver::on_animation_frame`](vertigo::Driver::on_animation_frame), draws it each frame.
//!
//! A [`Scene`] shows [`Object`]s - a [`Mesh`] each, with its [`Transform`] and [`Material`] - seen
//! by a [`Camera`]. The objects of a mesh are drawn together, with one call, so thousands of them
//! cost little more than one. [`frame_time`] makes them move, and [`Light`] - from everywhere,
//! from far away or from a point - lights the objects of [`Material::lit`], which shine with
//! highlights of [`Material::specular`]. A [`Texture`] puts an image on them, loaded ahead of
//! time if asked ([`SceneParams::preload`]) and told when it's in ([`SceneParams::on_load`]),
//! normal and height maps a relief
//! ([`Material::normal_map`], [`Material::height_map`]), and an object shows what is behind it
//! where its color's alpha is below 1. An object tells of clicks on it, with
//! [`Object::on_click`], of the pointer over it, with [`Object::on_hover`], and of dragging it,
//! with [`Object::on_drag`]. A [`Ray`] finds what it meets as clicks do, and [`Mesh::bounds`]
//! is the box around a mesh, f. ex. for a physics engine moving the objects. A [`Shader`] puts
//! code of its own into the shaders drawing a material, with the scene's time. A [`Layer`] draws
//! what objects can't be, with [`vertigo::webgl`] or [`vertigo::webgpu`], in the scene, with its
//! camera.

mod bounds;
mod camera;
mod color;
mod drag;
mod frame;
mod layer;
mod light;
mod material;
mod mesh;
mod object;
mod pick;
mod render;
mod scene;
mod shader;
mod texture;
mod transform;

pub use bounds::Bounds;
pub use camera::{Camera, Projection};
pub use color::Color;
pub use drag::{Drag, DragPhase};
pub use frame::frame_time;
pub use layer::{Layer, Targets, View};
pub use light::{Light, MAX_DIRECTIONAL, MAX_POINT};
pub use material::Material;
pub use mesh::{Mesh, MeshError};
pub use object::Object;
pub use pick::{Pick, Ray};
pub use scene::{Backend, Scene, SceneParams};
pub use shader::Shader;
pub use texture::{Loading, Texture, TextureError};
pub use transform::Transform;

/// [glam](https://docs.rs/glam), the vectors, rotations and matrices of scenes.
pub use glam as math;

/// The README's example, compiled with the doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
