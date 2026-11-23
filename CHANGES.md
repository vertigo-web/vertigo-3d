<!-- markdownlint-configure-file { "no-duplicate-heading": { "siblings_only": true } } -->

<!-- markdownlint-disable-next-line first-line-h1 -->
## Unreleased

### Added

* `Scene` - a 3D scene of `objects` seen by a `camera`, on a canvas as big as the element it is
  in, drawn with WebGL 2 when something in it changes and when the canvas changes its size, at
  most once a transaction, not before each frame. The objects of a mesh are drawn with one call.
  `SceneParams` with its `background`.
* `Object` - a `Mesh` with its `Transform` and `Material`, each a `Computed` (or anything
  becoming one), so it moves and changes with the values it's made of
* `Mesh` - `cube`, `sphere`, `plane` and `from_triangles` (checked, `MeshError`), shared by the
  objects of the shape and sent to the browser once
* `Transform` - translation, rotation and scale
* `Camera` - with perspective or orthographic (`Projection`), looking at a point, its aspect ratio
  from the canvas
* `Material::color` - one color all over, and `Material::lit` - as bright as the light falling on
  it
* `Light` - `ambient`, `directional` (up to `MAX_DIRECTIONAL`) and `point` (up to `MAX_POINT`),
  with a color and an intensity, given to a scene with `SceneParams::lights` - `Light::defaults`
  when not. A point light shines all around from where it is, fading smoothly to none at its
  `range`.
* `Material::specular` - highlights of a color where the directional and point lights shine off
  a lit surface toward the camera, as small and sharp as its shininess makes them
* `Material::normal_map` - the way a lit surface faces at each point, from a texture of normals,
  so the light shows a relief the mesh hasn't, and `Material::height_map` - how high each point
  is, the textures seen shifted the more the more slanting the surface is seen (parallax
  mapping). They lie on the mesh as the texture does. Meshes have tangents for them, worked out
  from their texture coordinates.
* `Shader` - code of its own in the shaders drawing a material's objects (`Material::shader`):
  GLSL statements changing each corner before it's placed and each pixel after it's lit, with
  the scene's time (`SceneParams::time`), f. ex. waves running over a surface or its colors
  changing. The objects keep their material's colors, textures and light, and are drawn together
  as any others.
* The detail of a scene, f. ex. for a slow computer, chosen by its user: `SceneParams::bitmap_scale`
  - fewer pixels than the canvas has on the screen, stretched over it, `SceneParams::antialias` -
  smoothed edges or not, and `SceneParams::relief` - normal and height maps shown or not
* `SceneParams::preload` - textures loaded on a scene's canvas before any object shows them, f. ex.
  those of the next view, so objects show them from the first time they're drawn instead of
  their base color until the image comes, and kept while listed
* `SceneParams::on_load` - told how far the images of a scene's textures from URLs are loaded
  (`Loading`: loaded, failed, of all), f. ex. to show an app once all its textures are in
* WebGPU: a scene draws with `vertigo::webgpu` where the browser has it, and with WebGL 2 where it
  hasn't, alike (`SceneParams::backend`, `Backend`: `Auto`, `WebGl` or `WebGpu`). Its objects in
  one render pass, with smooth edges of 4 samples a pixel, its textures with the smaller images
  the GPU makes of them. A `Shader` takes its code in WGSL for it too (`Shader::vertex_wgsl`,
  `Shader::fragment_wgsl`), and a `Layer` a way of its own (`Layer::webgpu`, drawing in the
  scene's pass on its `Targets`, and `Layer::or` for one of each); a scene with a shader or a
  layer of WebGL alone draws with WebGL.
* `Object::on_click` - clicked or tapped, with the point clicked (`Pick`), and `Object::on_hover` -
  the mouse over it or gone, where nothing nearer the camera is in the way. Found in Rust, by the
  objects' triangles. The pointer over an object to click is a hand.
* `Object::on_drag` - dragged with the mouse, a finger or a pen, told as the drag starts, moves
  and ends (`Drag`, `DragPhase`), also outside the canvas, with the `Ray` from the camera through
  the pointer. `Drag::moved_on` - how far the point grabbed has moved along a plane, f. ex. the
  floor. The pointer shows the object can be grabbed, and a finger on a scene of objects to drag
  drags them instead of scrolling the page.
* `Ray::on_mesh` and `Ray::nearest` - where a ray meets a mesh moved by a transform, or which of
  several it meets first, found as clicks are, f. ex. what an object stands on. `Ray::new` makes
  one of one's own.
* `Mesh::bounds` - the box around a mesh (`Bounds`), and `Bounds::transformed` the box around an
  object of it, f. ex. to size the shapes of a physics engine moving the objects or to tell
  whether two objects may touch (`Bounds::overlaps`). The storybook's "Physics" story moves
  objects with rapier3d, and its "DPT" story drags them around.
* `Texture` - an image on objects, from a URL the browser loads (the scene is drawn again when it
  comes) or from pixels made in Rust (`TextureError`), `sharp` for pixel art, shared by the
  objects showing it. `Material::texture` puts it on, times the material's color,
  `Material::repeat` tiles it, and where it's less than half opaque the surface is left out.
  Meshes have texture coordinates (`Mesh::with_uvs` for one of triangles).
* See-through objects - of a color with alpha below 1, or `Material::see_through` for a texture's
  alpha - drawn after the others, the farthest first, blended with what is behind them
* `Layer` - drawing of its own with `vertigo::webgl` in a scene, f. ex. lines, points or a
  shader, with the scene's camera (`View`): made on the scene's context and again when a lost one
  is given back, drawn when its data changes, after the opaque objects and before the
  see-through ones, which it may set its own blending, depth and culling for. Given to a scene
  with `SceneParams::layers`.
* `Color` - its parts from 0 to 1
* `frame_time` - seconds going on with the frames the browser draws, only while shown
* `math` - [glam](https://docs.rs/glam), re-exported
