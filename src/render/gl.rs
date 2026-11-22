//! A scene's frame drawn with WebGL: its opaque objects first, a call for those of each mesh and
//! texture, then its layers, then the objects it sees through, the farthest first, blended with
//! what is behind them.

use std::any::Any;

use vertigo::webgl::{
    BlendFactor, Buffer, BufferTarget, BufferUsage, Capability, ClearBuffers, Compare, DataType,
    Face, ImageLoad, IndexType, Primitive, Program, TextureFilter, TextureWrap, UniformLocation,
    VertexArray, WebGl, Winding,
};

use super::{Frame, INSTANCE, Made, arrived, batches, loading_of, see_through};
use crate::{
    Layer, Loading, MAX_DIRECTIONAL, MAX_POINT, Mesh, Shader, Texture, View,
    layer::Drawn,
    light::{Lighting, lighting},
    material::Surface,
    texture::Image,
};

const VERTEX: &str = "#version 300 es
layout(location = 0) in vec3 corner;
layout(location = 1) in vec3 corner_normal;
layout(location = 2) in mat4 transform;
layout(location = 6) in vec4 color;
layout(location = 7) in float lit;
layout(location = 8) in vec2 corner_uv;
layout(location = 9) in vec2 repeat;
layout(location = 10) in vec4 specular;
layout(location = 11) in vec4 tangent;
layout(location = 12) in float depth;
uniform mat4 view_projection;
uniform float time;
out vec4 shade;
out vec3 facing;
out vec4 along;
out vec3 world;
out float lighted;
out vec2 texel;
out vec4 shine;
out float depth_of;

void main() {
    vec3 position = corner;
    vec3 normal = corner_normal;
    vec2 uv = corner_uv;
    // The material's own code
    {code}
    vec4 placed = transform * vec4(position, 1.0);
    gl_Position = view_projection * placed;
    world = placed.xyz;
    shade = color;
    // So a surface stretched still faces the way it does.
    facing = transpose(inverse(mat3(transform))) * normal;
    along = vec4(mat3(transform) * tangent.xyz, tangent.w);
    lighted = lit;
    texel = uv * repeat;
    shine = specular;
    depth_of = depth;
}";

const FRAGMENT: &str = "#version 300 es
precision highp float;
const int DIRECTIONAL = 4;
const int POINTS = 4;
in vec4 shade;
in vec3 facing;
in vec4 along;
in vec3 world;
in float lighted;
in vec2 texel;
in vec4 shine;
in float depth_of;
uniform vec3 eye;
uniform float time;
uniform vec3 ambient;
uniform int light_count;
uniform vec3 light_directions[DIRECTIONAL];
uniform vec3 light_colors[DIRECTIONAL];
uniform int point_count;
uniform vec3 point_positions[POINTS];
uniform vec3 point_colors[POINTS];
uniform float point_ranges[POINTS];
uniform sampler2D image;
uniform sampler2D normal_map;
uniform sampler2D height_map;
// Whether the surface has a normal map.
uniform bool bumped;
// Whether normal and height maps show.
uniform bool relief_shown;
// Drawing the opaque objects: what is less than half opaque is left out, the rest is opaque.
uniform bool cutout;
out vec4 pixel;

vec3 diffuse;
vec3 highlight;
// The texture's right, its up and the normal, the way a normal map's colors go.
mat3 relief = mat3(1.0);

// Light of `strength` coming from `toward`, onto a surface facing `normal`, seen from `seen`.
void shine_on(vec3 normal, vec3 seen, vec3 toward, vec3 strength) {
    float facing_light = dot(normal, toward);
    if (facing_light <= 0.0) {
        return;
    }
    diffuse += strength * facing_light;
    highlight += strength * pow(max(dot(normal, normalize(toward + seen)), 0.0), shine.a);
}

// `surface` lit, facing `normal`, seen from `seen`, its normal map read at `at` if `bumpy`.
void lit(vec4 surface, vec3 normal, vec3 seen, vec2 at, bool bumpy) {
    if (bumpy) {
        normal = normalize(relief * (texture(normal_map, at).xyz * 2.0 - 1.0));
    }
    diffuse = ambient;
    highlight = vec3(0.0);
    for (int i = 0; i < DIRECTIONAL; i++) {
        if (i >= light_count) {
            break;
        }
        shine_on(normal, seen, -light_directions[i], light_colors[i]);
    }
    for (int i = 0; i < POINTS; i++) {
        if (i >= point_count) {
            break;
        }
        vec3 to_light = point_positions[i] - world;
        float distance = length(to_light);
        float fade = clamp(1.0 - distance / point_ranges[i], 0.0, 1.0);
        shine_on(normal, seen, to_light / max(distance, 0.0001), point_colors[i] * fade * fade);
    }
    pixel = vec4(surface.rgb * diffuse + shine.rgb * highlight, surface.a);
}

void main() {
    vec3 normal = normalize(facing);
    vec3 seen = normalize(eye - world);
    vec2 at = texel;
    bool bumpy = bumped && relief_shown;
    float deep = relief_shown ? depth_of : 0.0;
    if (bumpy || deep > 0.0) {
        vec3 right = along.xyz - normal * dot(normal, along.xyz);
        right = length(right) > 0.0 ? normalize(right) : vec3(1.0, 0.0, 0.0);
        relief = mat3(right, cross(normal, right) * along.w, normal);
    }
    if (deep > 0.0) {
        vec3 seen_on = transpose(relief) * seen;
        float height = texture(height_map, at).r - 0.5;
        // The texture's down is its up's other way.
        at += vec2(seen_on.x, -seen_on.y) * height * deep;
    }
    vec4 surface = shade * texture(image, at);
    if (cutout) {
        if (surface.a < 0.5) {
            discard;
        }
        surface.a = 1.0;
    }
    if (lighted < 0.5) {
        pixel = surface;
    } else {
        lit(surface, normal, seen, at, bumpy);
    }
    // The material's own code
    {code}
}";

/// The source of the shaders drawing objects, vertex and fragment, with `shader`'s code of its
/// own, or none.
fn sources(shader: Option<&Shader>) -> (String, String) {
    let (vertex, fragment) = shader.map_or(("", ""), |shader| {
        (shader.vertex_code(), shader.fragment_code())
    });
    (
        VERTEX.replacen("{code}", vertex, 1),
        FRAGMENT.replacen("{code}", fragment, 1),
    )
}

/// The program drawing objects with a shader's code of its own, or none, and where its uniforms
/// are.
struct Shading {
    shader: Option<Shader>,
    program: Program,
    view_projection: UniformLocation,
    eye: UniformLocation,
    time: UniformLocation,
    ambient: UniformLocation,
    light_count: UniformLocation,
    /// Of each directional light, its direction and its color.
    lights: Vec<(UniformLocation, UniformLocation)>,
    point_count: UniformLocation,
    /// Of each point light, its position, its color and its range.
    points: Vec<(UniformLocation, UniformLocation, UniformLocation)>,
    image: UniformLocation,
    normal_map: UniformLocation,
    height_map: UniformLocation,
    bumped: UniformLocation,
    relief: UniformLocation,
    cutout: UniformLocation,
}

impl Shading {
    fn new(gl: &WebGl, shader: Option<Shader>) -> Self {
        let (vertex, fragment) = sources(shader.as_ref());
        let program = gl.create_program(&vertex, &fragment);
        let at = |name: &str| gl.uniform_location(&program, name);
        let lights = (0..MAX_DIRECTIONAL)
            .map(|index| {
                (
                    at(&format!("light_directions[{index}]")),
                    at(&format!("light_colors[{index}]")),
                )
            })
            .collect();
        let points = (0..MAX_POINT)
            .map(|index| {
                (
                    at(&format!("point_positions[{index}]")),
                    at(&format!("point_colors[{index}]")),
                    at(&format!("point_ranges[{index}]")),
                )
            })
            .collect();

        Self {
            view_projection: at("view_projection"),
            eye: at("eye"),
            time: at("time"),
            ambient: at("ambient"),
            light_count: at("light_count"),
            lights,
            point_count: at("point_count"),
            points,
            image: at("image"),
            normal_map: at("normal_map"),
            height_map: at("height_map"),
            bumped: at("bumped"),
            relief: at("relief_shown"),
            cutout: at("cutout"),
            program,
            shader,
        }
    }

    /// The uniforms the frame gives all objects alike, on the program, which is used after.
    fn frame(&self, gl: &WebGl, frame: &Frame, lighting: &Lighting) {
        gl.use_program(Some(&self.program));
        gl.uniform_matrix_4f(
            &self.view_projection,
            &frame
                .camera
                .view_projection(gl.size().aspect())
                .to_cols_array(),
        );
        let eye = frame.camera.eye;
        gl.uniform_3f(&self.eye, eye.x, eye.y, eye.z);
        gl.uniform_1f(&self.time, frame.time);
        let ambient = lighting.ambient;
        gl.uniform_3f(&self.ambient, ambient.x, ambient.y, ambient.z);
        gl.uniform_1i(&self.light_count, lighting.directional.len() as i32);
        for ((direction, color), (at_direction, at_color)) in
            lighting.directional.iter().zip(&self.lights)
        {
            gl.uniform_3f(at_direction, direction.x, direction.y, direction.z);
            gl.uniform_3f(at_color, color.x, color.y, color.z);
        }
        gl.uniform_1i(&self.point_count, lighting.point.len() as i32);
        for ((position, range, color), (at_position, at_color, at_range)) in
            lighting.point.iter().zip(&self.points)
        {
            gl.uniform_3f(at_position, position.x, position.y, position.z);
            gl.uniform_3f(at_color, color.x, color.y, color.z);
            gl.uniform_1f(at_range, *range);
        }
        gl.uniform_1i(&self.image, 0);
        gl.uniform_1i(&self.normal_map, 1);
        gl.uniform_1i(&self.height_map, 2);
        gl.uniform_1i(&self.relief, i32::from(frame.relief));
    }
}

/// What a scene made on its context, to draw with.
pub(crate) struct Renderer {
    gl: WebGl,
    /// The program of objects without a shader of their own, first, then those of the shaders
    /// in the scene.
    shadings: Vec<Shading>,
    /// The texture of objects without one: a white pixel.
    white: vertigo::webgl::Texture,
    textures: Vec<Made<vertigo::webgl::Texture>>,
    /// Of the opaque instances of each mesh and surface.
    groups: Vec<Group>,
    /// Of the see-through instances of each mesh.
    see_through: Vec<Group>,
    /// What each layer made on the context.
    layers: Vec<(Layer, Box<dyn Any>)>,
}

impl Renderer {
    pub fn new(gl: &WebGl) -> Self {
        let white = gl.create_texture();
        gl.bind_texture(Some(&white));
        gl.texture_image(1, 1, &[255, 255, 255, 255]);
        gl.texture_filter(TextureFilter::Nearest, TextureFilter::Nearest);

        let renderer = Self {
            gl: gl.clone(),
            shadings: vec![Shading::new(gl, None)],
            white,
            textures: Vec::new(),
            groups: Vec::new(),
            see_through: Vec::new(),
            layers: Vec::new(),
        };
        renderer.set_state();
        renderer
    }

    /// The state of drawing the objects need, set again after the layers, which may change it.
    fn set_state(&self) {
        let gl = &self.gl;
        gl.enable(Capability::DepthTest);
        gl.depth_func(Compare::Less);
        gl.depth_mask(true);
        gl.enable(Capability::CullFace);
        gl.cull_face(Face::Back);
        gl.front_face(Winding::CounterClockwise);
        gl.disable(Capability::Blend);
        gl.blend_func_separate(
            BlendFactor::SourceAlpha,
            BlendFactor::OneMinusSourceAlpha,
            // The canvas stays as opaque as it is.
            BlendFactor::One,
            BlendFactor::OneMinusSourceAlpha,
        );
    }

    pub fn draw(&mut self, frame: &Frame) {
        let gl = self.gl.clone();

        // The groups and textures still in the scene are kept, with what they sent, and the
        // others let go.
        let mut before = std::mem::take(&mut self.groups);
        for batch in batches(&frame.instances) {
            let index = before
                .iter()
                .position(|group| group.mesh == batch.mesh && group.surface == batch.surface);
            let mut group = match index {
                Some(index) => before.swap_remove(index),
                None => Group::new(&gl, batch.mesh, batch.surface),
            };
            group.send(&gl, batch.data);
            self.groups.push(group);
        }

        let (meshes, runs) = see_through(&frame.instances, &frame.camera);
        let mut before = std::mem::take(&mut self.see_through);
        for (mesh, data) in meshes {
            let mut group = match before.iter().position(|group| group.mesh == mesh) {
                Some(index) => before.swap_remove(index),
                None => Group::new(&gl, mesh, Surface::default()),
            };
            group.send(&gl, data);
            self.see_through.push(group);
        }

        // What each layer of WebGL still in the scene made is kept, and made for each new one.
        let mut before = std::mem::take(&mut self.layers);
        for Drawn { layer, .. } in &frame.layers {
            let made = match before.iter().position(|(made, _)| made == layer) {
                Some(index) => before.swap_remove(index).1,
                None => match layer.make(&gl) {
                    Some(made) => made,
                    None => continue,
                },
            };
            self.layers.push((layer.clone(), made));
        }

        let surfaces = || {
            self.groups
                .iter()
                .map(|group| &group.surface)
                .chain(runs.iter().map(|run| &run.surface))
        };
        // And the textures to load before objects show them.
        let used: Vec<Texture> = surfaces()
            .flat_map(Surface::textures)
            .chain(&frame.preload)
            .cloned()
            .collect();
        let shaders: Vec<Shader> = surfaces()
            .filter_map(|surface| surface.shader.clone())
            .collect();
        self.textures.retain(|made| used.contains(&made.texture));
        for texture in used {
            if !self.textures.iter().any(|made| made.texture == texture) {
                let made = upload(&gl, &texture);
                self.textures.push(Made::new(texture, made));
            }
        }
        // The programs of the shaders still in the scene are kept, and made for each new one.
        self.shadings.retain(|shading| {
            shading
                .shader
                .as_ref()
                .is_none_or(|shader| shaders.contains(shader))
        });
        for shader in shaders {
            if !self
                .shadings
                .iter()
                .any(|shading| shading.shader.as_ref() == Some(&shader))
            {
                self.shadings.push(Shading::new(&gl, Some(shader)));
            }
        }

        let background = frame.background;
        gl.clear_color(
            background.red,
            background.green,
            background.blue,
            background.alpha,
        );
        gl.clear(ClearBuffers::COLOR | ClearBuffers::DEPTH);
        let lighting = lighting(&frame.lights);
        for shading in &self.shadings {
            shading.frame(&gl, frame, &lighting);
        }

        for group in &self.groups {
            self.bind(&group.surface, true);
            gl.bind_vertex_array(Some(&group.vertices));
            gl.draw_elements_instanced(
                Primitive::Triangles,
                group.indices,
                IndexType::UnsignedInt,
                0,
                group.count,
            );
        }

        if !frame.layers.is_empty() {
            let size = gl.size();
            let view = View {
                camera: frame.camera,
                view_projection: frame.camera.view_projection(size.aspect()),
                size,
            };
            for (layer, made) in &mut self.layers {
                if let Some(drawn) = frame.layers.iter().find(|drawn| drawn.layer == *layer) {
                    layer.draw(&mut **made, &gl, &*drawn.data, &view);
                }
            }
            self.set_state();
        }

        if !runs.is_empty() {
            // Behind each other, they hide nothing: the farthest is drawn first.
            gl.enable(Capability::Blend);
            gl.depth_mask(false);
            for run in &runs {
                let Some(group) = self.see_through.get(run.mesh) else {
                    continue;
                };
                self.bind(&run.surface, false);
                gl.bind_vertex_array(Some(&group.vertices));
                group.point(&gl, run.first);
                gl.draw_elements_instanced(
                    Primitive::Triangles,
                    group.indices,
                    IndexType::UnsignedInt,
                    0,
                    run.count,
                );
            }
            gl.depth_mask(true);
            gl.disable(Capability::Blend);
        }
    }

    /// An image came or failed: a texture of its URL waiting for it has it, or stays white.
    pub fn image(&mut self, load: &ImageLoad) {
        arrived(&mut self.textures, load);
    }

    /// How far the images of the textures from URLs are loaded.
    pub fn loading(&self) -> Loading {
        loading_of(&self.textures)
    }

    /// The program of a surface, used, and its textures on the units the program reads them
    /// from, white where it has none. Opaque objects are `cutout`: what of them is less than
    /// half opaque is left out, and the rest is opaque.
    fn bind(&self, surface: &Surface, cutout: bool) {
        let shading = self
            .shadings
            .iter()
            .find(|shading| shading.shader == surface.shader)
            .unwrap_or(&self.shadings[0]);
        self.gl.use_program(Some(&shading.program));
        self.gl.uniform_1i(&shading.cutout, i32::from(cutout));
        self.gl
            .uniform_1i(&shading.bumped, i32::from(surface.normal.is_some()));

        let made = |texture: &Option<Texture>| {
            texture
                .as_ref()
                .and_then(|texture| self.textures.iter().find(|made| made.texture == *texture))
                .map_or(&self.white, |made| &made.made)
        };
        for (unit, texture) in [&surface.color, &surface.normal, &surface.height]
            .into_iter()
            .enumerate()
        {
            self.gl.active_texture(unit as u32);
            self.gl.bind_texture(Some(made(texture)));
        }
        self.gl.active_texture(0);
    }
}

/// `texture` on the context: the pixels at once, an image from a URL when the browser has it,
/// white until then.
fn upload(gl: &WebGl, texture: &Texture) -> vertigo::webgl::Texture {
    let data = texture.data();
    let made = gl.create_texture();
    gl.bind_texture(Some(&made));
    gl.texture_wrap(TextureWrap::Repeat, TextureWrap::Repeat);

    match &data.image {
        Image::Url(url) => {
            gl.texture_image(1, 1, &[255, 255, 255, 255]);
            gl.texture_image_url(&made, url, !data.sharp);
        }
        Image::Pixels {
            width,
            height,
            pixels,
        } => {
            gl.texture_image(*width, *height, pixels);
            if !data.sharp {
                gl.generate_mipmap();
            }
        }
    }
    if data.sharp {
        gl.texture_filter(TextureFilter::Nearest, TextureFilter::Nearest);
    } else {
        gl.texture_filter(TextureFilter::LinearMipmapLinear, TextureFilter::Linear);
    }

    made
}

/// A mesh on the context, and the buffer of its instances, read an instance at a time.
struct Group {
    mesh: Mesh,
    surface: Surface,
    vertices: VertexArray,
    indices: u32,
    instances: Buffer,
    /// What the instances' buffer holds, to send only what changed.
    sent: Vec<f32>,
    count: u32,
    _buffers: [Buffer; 2],
}

impl Group {
    fn new(gl: &WebGl, mesh: Mesh, surface: Surface) -> Self {
        let data = mesh.data();
        let corners: Vec<f32> = data
            .positions
            .chunks(3)
            .zip(data.normals.chunks(3))
            .zip(data.uvs.chunks(2))
            .zip(data.tangents.chunks(4))
            .flat_map(|(((position, normal), uv), tangent)| {
                position
                    .iter()
                    .chain(normal)
                    .chain(uv)
                    .chain(tangent)
                    .copied()
            })
            .collect();

        let vertices = gl.create_vertex_array();
        gl.bind_vertex_array(Some(&vertices));

        let corner_buffer = gl.create_buffer();
        gl.bind_buffer(BufferTarget::Array, Some(&corner_buffer));
        gl.buffer_data(BufferTarget::Array, &corners, BufferUsage::StaticDraw);
        // Of each corner: its position, its normal, its texture coordinates and its tangent.
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer(0, 3, DataType::Float, false, 48, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer(1, 3, DataType::Float, false, 48, 12);
        gl.enable_vertex_attrib_array(8);
        gl.vertex_attrib_pointer(8, 2, DataType::Float, false, 48, 24);
        gl.enable_vertex_attrib_array(11);
        gl.vertex_attrib_pointer(11, 4, DataType::Float, false, 48, 32);

        let index_buffer = gl.create_buffer();
        gl.bind_buffer(BufferTarget::ElementArray, Some(&index_buffer));
        gl.buffer_data(
            BufferTarget::ElementArray,
            &data.indices,
            BufferUsage::StaticDraw,
        );

        let instances = gl.create_buffer();
        // A `mat4` takes four locations, a column each.
        for location in [2, 3, 4, 5, 6, 7, 9, 10, 12] {
            gl.enable_vertex_attrib_array(location);
            gl.vertex_attrib_divisor(location, 1);
        }
        let group = Self {
            indices: data.indices.len() as u32,
            mesh,
            surface,
            vertices,
            instances,
            sent: Vec::new(),
            count: 0,
            _buffers: [corner_buffer, index_buffer],
        };
        group.point(gl, 0);
        gl.bind_vertex_array(None);
        group
    }

    /// Points the attributes of the instances, in the bound vertex array, at the instance
    /// `first` of the buffer, which an instanced draw then starts from.
    fn point(&self, gl: &WebGl, first: u32) {
        let stride = (INSTANCE * 4) as u32;
        let at = first * stride;
        gl.bind_buffer(BufferTarget::Array, Some(&self.instances));
        for column in 0..4 {
            gl.vertex_attrib_pointer(
                2 + column,
                4,
                DataType::Float,
                false,
                stride,
                at + column * 16,
            );
        }
        gl.vertex_attrib_pointer(6, 4, DataType::Float, false, stride, at + 64);
        gl.vertex_attrib_pointer(7, 1, DataType::Float, false, stride, at + 80);
        gl.vertex_attrib_pointer(9, 2, DataType::Float, false, stride, at + 84);
        gl.vertex_attrib_pointer(10, 4, DataType::Float, false, stride, at + 92);
        gl.vertex_attrib_pointer(12, 1, DataType::Float, false, stride, at + 108);
    }

    /// The instances, sent only if they aren't what the buffer holds, in a buffer made anew
    /// when they don't fit in the one there is.
    fn send(&mut self, gl: &WebGl, data: Vec<f32>) {
        if data == self.sent {
            return;
        }

        gl.bind_buffer(BufferTarget::Array, Some(&self.instances));
        if data.len() > self.sent.len().max(self.count as usize * INSTANCE) {
            gl.buffer_data(BufferTarget::Array, &data, BufferUsage::DynamicDraw);
        } else {
            gl.buffer_sub_data(BufferTarget::Array, 0, &data);
        }
        self.count = (data.len() / INSTANCE) as u32;
        self.sent = data;
    }
}

#[cfg(test)]
mod tests {
    use super::sources;
    use crate::Shader;

    #[test]
    fn a_shaders_code_goes_into_the_programs_where_it_may_change_what_they_draw() {
        let shader = Shader::new()
            .vertex("position.y += sin(time);")
            .fragment("pixel.r = 1.0;");
        let (vertex, fragment) = sources(Some(&shader));
        let before = |source: &str, code: &str, line: &str| {
            let (Some(code), Some(line)) = (source.find(code), source.find(line)) else {
                panic!("no {code} or {line} in {source}");
            };
            code < line
        };
        assert!(before(&vertex, "position.y += sin(time);", "vec4 placed ="));
        assert!(before(
            &fragment,
            "lit(surface, normal, seen, at, bumpy);",
            "pixel.r = 1.0;"
        ));

        let (vertex, fragment) = sources(None);
        assert!(!vertex.contains("{code}") && !fragment.contains("{code}"));
    }
}
