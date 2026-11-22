//! A scene's frame drawn with WebGPU, in one render pass: its opaque objects first, a call for
//! those of each mesh and surface, then its layers, then the objects it sees through, the farthest
//! first, blended with what is behind them. The shaders do what those of WebGL do, see `gl`.
//!
//! The pipelines share one layout: the bind group 0 is the frame's, its uniforms, and the bind
//! group 1 a surface's, its textures with their samplers and whether it has a normal map. Each
//! shader has two pipelines, of the opaque objects and of the see-through ones.

use std::any::Any;

use glam::Mat4;
use vertigo::webgpu::{
    AddressMode, BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingResource,
    BindingType, BlendState, Buffer, BufferBindingType, BufferUsages, Color, ColorAttachment,
    ColorTargetState, CompareFunction, DepthAttachment, DepthStencilState, Face, FilterMode,
    FragmentState, ImageLoad, ImageOptions, IndexFormat, LoadOp, MultisampleState, PipelineLayout,
    PrimitiveState, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, Size, TextureDescriptor, TextureFormat,
    TextureSampleType, TextureUsages, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode, WebGpu,
};

use super::{Frame, INSTANCE, Made, arrived, batches, loading_of, see_through};
use crate::{
    Layer, Loading, MAX_DIRECTIONAL, MAX_POINT, Mesh, Shader, Targets, Texture, View,
    layer::Drawn,
    light::{Lighting, lighting},
    material::Surface,
    texture::Image,
};

const DEPTH: TextureFormat = TextureFormat::Depth24Plus;

/// From the depths of WebGL, -1 to 1, which the camera's projection gives, to those of WebGPU,
/// 0 to 1.
const DEPTH_ZERO_TO_ONE: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 0.5, 0.0, //
    0.0, 0.0, 0.5, 1.0,
]);

/// The camera's view and projection as WebGPU takes it, for the canvas's `aspect`.
pub(crate) fn view_projection(frame: &Frame, aspect: f32) -> Mat4 {
    DEPTH_ZERO_TO_ONE * frame.camera.view_projection(aspect)
}

/// Floats of the frame's uniforms, see `Frame` in [`SHADER`].
const UNIFORMS: usize = 92;

/// Bytes of a corner of a mesh: its position, its normal, its texture coordinates and its
/// tangent.
const CORNER: u32 = 48;

/// The shaders drawing objects, `{cutout}` whether what is less than half opaque is left out, as
/// of the opaque objects, `{vertex}` and `{fragment}` the code of a [`Shader`]. As `VERTEX` and
/// `FRAGMENT` of `gl`, which they follow line by line where they can.
const SHADER: &str = "
const CUTOUT: bool = {cutout};
const DIRECTIONAL: i32 = 4;
const POINTS: i32 = 4;

struct Frame {
    view_projection: mat4x4f,
    eye: vec3f,
    time: f32,
    ambient: vec3f,
    // Whether normal and height maps show: 1 or 0.
    relief: f32,
    light_count: f32,
    point_count: f32,
    // Of each directional light, its direction and its color.
    lights: array<vec4f, 8>,
    // Of each point light, its position with its range, and its color.
    points: array<vec4f, 8>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
@group(1) @binding(2) var normal_map: texture_2d<f32>;
@group(1) @binding(3) var normal_sampler: sampler;
@group(1) @binding(4) var height_map: texture_2d<f32>;
@group(1) @binding(5) var height_sampler: sampler;
// Whether the surface has a normal map: 1 or 0.
@group(1) @binding(6) var<uniform> bumped: vec4f;

struct Corner {
    @builtin(position) clip: vec4f,
    @location(0) shade: vec4f,
    @location(1) facing: vec3f,
    @location(2) along: vec4f,
    @location(3) world: vec3f,
    @location(4) lighted: f32,
    @location(5) texel: vec2f,
    @location(6) shine: vec4f,
    @location(7) depth_of: f32,
};

@vertex fn vertex(
    @location(0) corner: vec3f,
    @location(1) corner_normal: vec3f,
    @location(8) corner_uv: vec2f,
    @location(11) tangent: vec4f,
    @location(2) column_0: vec4f,
    @location(3) column_1: vec4f,
    @location(4) column_2: vec4f,
    @location(5) column_3: vec4f,
    @location(6) color: vec4f,
    @location(7) lit: f32,
    @location(9) repeat: vec2f,
    @location(10) specular: vec4f,
    @location(12) depth: f32,
) -> Corner {
    let time = frame.time;
    var position = corner;
    var normal = corner_normal;
    var uv = corner_uv;
    // The material's own code
    {vertex}
    let transform = mat4x4f(column_0, column_1, column_2, column_3);
    let placed = transform * vec4f(position, 1.0);
    var out: Corner;
    out.clip = frame.view_projection * placed;
    out.world = placed.xyz;
    out.shade = color;
    // So a surface stretched still faces the way it does: the inverse of the transform, turned,
    // which WGSL has no function for, its cofactors over its determinant.
    let x = transform[0].xyz;
    let y = transform[1].xyz;
    let z = transform[2].xyz;
    let determinant = dot(x, cross(y, z));
    let cofactors = mat3x3f(cross(y, z), cross(z, x), cross(x, y));
    out.facing = cofactors * normal / select(determinant, 1.0, determinant == 0.0);
    out.along = vec4f(mat3x3f(x, y, z) * tangent.xyz, tangent.w);
    out.lighted = lit;
    out.texel = uv * repeat;
    out.shine = specular;
    out.depth_of = depth;
    return out;
}

var<private> diffuse: vec3f;
var<private> highlight: vec3f;

// Light of `strength` coming from `toward`, onto a surface facing `normal`, seen from `seen`.
fn shine_on(normal: vec3f, seen: vec3f, toward: vec3f, strength: vec3f, shininess: f32) {
    let facing_light = dot(normal, toward);
    if (facing_light <= 0.0) {
        return;
    }
    diffuse += strength * facing_light;
    highlight += strength * pow(max(dot(normal, normalize(toward + seen)), 0.0), shininess);
}

@fragment fn fragment(corner: Corner) -> @location(0) vec4f {
    let time = frame.time;
    let world = corner.world;
    let texel = corner.texel;
    var normal = normalize(corner.facing);
    let seen = normalize(frame.eye - world);
    var at = texel;
    let relief_shown = frame.relief > 0.5;
    let bumpy = bumped.x > 0.5 && relief_shown;
    let deep = select(0.0, corner.depth_of, relief_shown);
    // The texture's right, its up and the normal, the way a normal map's colors go.
    var relief = mat3x3f(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
    if (bumpy || deep > 0.0) {
        var right = corner.along.xyz - normal * dot(normal, corner.along.xyz);
        right = select(vec3f(1.0, 0.0, 0.0), normalize(right), length(right) > 0.0);
        relief = mat3x3f(right, cross(normal, right) * corner.along.w, normal);
    }
    // Read where all pixels read, as WebGPU wants it.
    let height = textureSample(height_map, height_sampler, at).r - 0.5;
    if (deep > 0.0) {
        let seen_on = transpose(relief) * seen;
        // The texture's down is its up's other way.
        at += vec2f(seen_on.x, -seen_on.y) * height * deep;
    }
    let bump = textureSample(normal_map, normal_sampler, at).xyz;
    var surface = corner.shade * textureSample(image, image_sampler, at);
    if (CUTOUT) {
        if (surface.a < 0.5) {
            discard;
        }
        surface.a = 1.0;
    }
    var pixel = surface;
    if (corner.lighted >= 0.5) {
        if (bumpy) {
            normal = normalize(relief * (bump * 2.0 - 1.0));
        }
        diffuse = frame.ambient;
        highlight = vec3f(0.0);
        for (var i = 0; i < min(i32(frame.light_count), DIRECTIONAL); i++) {
            shine_on(normal, seen, -frame.lights[2 * i].xyz, frame.lights[2 * i + 1].xyz, corner.shine.a);
        }
        for (var i = 0; i < min(i32(frame.point_count), POINTS); i++) {
            let point = frame.points[2 * i];
            let to_light = point.xyz - world;
            let distance = length(to_light);
            let fade = clamp(1.0 - distance / point.w, 0.0, 1.0);
            shine_on(normal, seen, to_light / max(distance, 0.0001),
                frame.points[2 * i + 1].xyz * fade * fade, corner.shine.a);
        }
        pixel = vec4f(surface.rgb * diffuse + corner.shine.rgb * highlight, surface.a);
    }
    // The material's own code
    {fragment}
    return pixel;
}
";

/// The source of the shaders drawing objects, opaque ones if `cutout`, with `shader`'s code of its
/// own, or none.
fn source(shader: Option<&Shader>, cutout: bool) -> String {
    let (vertex, fragment) = shader.map_or(("", ""), |shader| {
        (shader.vertex_wgsl_code(), shader.fragment_wgsl_code())
    });
    SHADER
        .replacen("{cutout}", if cutout { "true" } else { "false" }, 1)
        .replacen("{vertex}", vertex, 1)
        .replacen("{fragment}", fragment, 1)
}

/// The pipelines drawing objects with a shader's code of its own, or none: of the opaque objects
/// and of the see-through ones.
struct Shading {
    shader: Option<Shader>,
    opaque: RenderPipeline,
    see_through: RenderPipeline,
}

impl Shading {
    fn new(gpu: &WebGpu, layout: &PipelineLayout, samples: u32, shader: Option<Shader>) -> Self {
        let pipeline = |cutout: bool| {
            let module = gpu.create_shader_module(&source(shader.as_ref(), cutout));
            gpu.create_render_pipeline(&RenderPipelineDescriptor {
                layout: Some(layout),
                vertex: VertexState {
                    module: &module,
                    entry_point: "vertex",
                    buffers: &[
                        VertexBufferLayout {
                            array_stride: CORNER,
                            step_mode: VertexStepMode::Vertex,
                            attributes: &[
                                attribute(0, VertexFormat::Float32x3, 0),
                                attribute(1, VertexFormat::Float32x3, 12),
                                attribute(8, VertexFormat::Float32x2, 24),
                                attribute(11, VertexFormat::Float32x4, 32),
                            ],
                        },
                        VertexBufferLayout {
                            array_stride: (INSTANCE * 4) as u32,
                            step_mode: VertexStepMode::Instance,
                            // As `Instance::put` puts them
                            attributes: &[
                                attribute(2, VertexFormat::Float32x4, 0),
                                attribute(3, VertexFormat::Float32x4, 16),
                                attribute(4, VertexFormat::Float32x4, 32),
                                attribute(5, VertexFormat::Float32x4, 48),
                                attribute(6, VertexFormat::Float32x4, 64),
                                attribute(7, VertexFormat::Float32, 80),
                                attribute(9, VertexFormat::Float32x2, 84),
                                attribute(10, VertexFormat::Float32x4, 92),
                                attribute(12, VertexFormat::Float32, 108),
                            ],
                        },
                    ],
                },
                primitive: PrimitiveState {
                    cull_mode: Some(Face::Back),
                    ..PrimitiveState::default()
                },
                // See-through objects hide nothing behind them.
                depth_stencil: Some(DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: cutout,
                    depth_compare: CompareFunction::Less,
                }),
                fragment: Some(FragmentState {
                    module: &module,
                    entry_point: "fragment",
                    targets: &[ColorTargetState {
                        blend: (!cutout).then_some(BlendState::ALPHA_BLENDING),
                        ..ColorTargetState::canvas()
                    }],
                }),
                multisample: MultisampleState { count: samples },
            })
        };

        Self {
            opaque: pipeline(true),
            see_through: pipeline(false),
            shader,
        }
    }
}

fn attribute(location: u32, format: VertexFormat, offset: u32) -> VertexAttribute {
    VertexAttribute {
        format,
        offset,
        shader_location: location,
    }
}

/// A surface's bind group: its textures, white where it has none, with the samplers of their
/// sharpness, and whether it has a normal map.
struct Surfacing {
    surface: Surface,
    group: BindGroup,
    _bumped: Buffer,
}

/// What a scene made on its context, to draw with.
pub(crate) struct Renderer {
    gpu: WebGpu,
    /// Samples a pixel: 4 with smooth edges, or 1.
    samples: u32,
    surface_layout: BindGroupLayout,
    pipeline_layout: PipelineLayout,
    uniforms: Buffer,
    frame_group: BindGroup,
    /// The pipelines of objects without a shader of their own, first, then those of the shaders
    /// in the scene.
    shadings: Vec<Shading>,
    /// The texture of a surface without one: a white pixel.
    white: vertigo::webgpu::Texture,
    sharp: Sampler,
    smooth: Sampler,
    textures: Vec<Made<vertigo::webgpu::Texture>>,
    surfaces: Vec<Surfacing>,
    /// Of the opaque instances of each mesh and surface.
    groups: Vec<Group>,
    /// Of the see-through instances of each mesh.
    see_through: Vec<Group>,
    /// What each layer drawing with WebGPU made on the context.
    layers: Vec<(Layer, Box<dyn Any>)>,
}

impl Renderer {
    /// What the scene draws with on `gpu`, of `samples` samples a pixel.
    pub fn new(gpu: &WebGpu, samples: u32) -> Self {
        let frame_layout = gpu.create_bind_group_layout(&[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            ty: BindingType::Buffer(BufferBindingType::Uniform),
        }]);
        let texture = |binding| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                multisampled: false,
            },
        };
        let sampler = |binding| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
        };
        let surface_layout = gpu.create_bind_group_layout(&[
            texture(0),
            sampler(1),
            texture(2),
            sampler(3),
            texture(4),
            sampler(5),
            BindGroupLayoutEntry {
                binding: 6,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer(BufferBindingType::Uniform),
            },
        ]);
        let pipeline_layout = gpu.create_pipeline_layout(&[&frame_layout, &surface_layout]);

        let uniforms = gpu.create_buffer(
            (UNIFORMS * 4) as u32,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let frame_group = gpu.create_bind_group(
            &frame_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(&uniforms),
            }],
        );

        let white = gpu.create_texture(&TextureDescriptor {
            size: Size {
                width: 1,
                height: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        });
        gpu.write_texture(&white, &[255, 255, 255, 255]);
        let sampler = |filter: FilterMode| {
            gpu.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: filter,
            })
        };

        Self {
            shadings: vec![Shading::new(gpu, &pipeline_layout, samples, None)],
            gpu: gpu.clone(),
            samples,
            surface_layout,
            pipeline_layout,
            uniforms,
            frame_group,
            white,
            sharp: sampler(FilterMode::Nearest),
            smooth: sampler(FilterMode::Linear),
            textures: Vec::new(),
            surfaces: Vec::new(),
            groups: Vec::new(),
            see_through: Vec::new(),
            layers: Vec::new(),
        }
    }

    /// What layers drawing with WebGPU draw on.
    fn targets(&self) -> Targets {
        Targets {
            color: TextureFormat::Canvas,
            depth: DEPTH,
            samples: self.samples,
        }
    }

    pub fn draw(&mut self, frame: &Frame) {
        let gpu = self.gpu.clone();

        // The groups, textures, surfaces, shadings and layers still in the scene are kept, with
        // what they sent, and the others let go.
        let mut before = std::mem::take(&mut self.groups);
        for batch in batches(&frame.instances) {
            let index = before
                .iter()
                .position(|group| group.mesh == batch.mesh && group.surface == batch.surface);
            let mut group = match index {
                Some(index) => before.swap_remove(index),
                None => Group::new(&gpu, batch.mesh, batch.surface),
            };
            group.send(&gpu, batch.data);
            self.groups.push(group);
        }

        let (meshes, runs) = see_through(&frame.instances, &frame.camera);
        let mut before = std::mem::take(&mut self.see_through);
        for (mesh, data) in meshes {
            let mut group = match before.iter().position(|group| group.mesh == mesh) {
                Some(index) => before.swap_remove(index),
                None => Group::new(&gpu, mesh, Surface::default()),
            };
            group.send(&gpu, data);
            self.see_through.push(group);
        }

        let targets = self.targets();
        let mut before = std::mem::take(&mut self.layers);
        for Drawn { layer, .. } in &frame.layers {
            let made = match before.iter().position(|(made, _)| made == layer) {
                Some(index) => before.swap_remove(index).1,
                None => match layer.make_gpu(&gpu, &targets) {
                    Some(made) => made,
                    None => continue,
                },
            };
            self.layers.push((layer.clone(), made));
        }

        let surfaces: Vec<Surface> = self
            .groups
            .iter()
            .map(|group| group.surface.clone())
            .chain(runs.iter().map(|run| run.surface.clone()))
            .collect();
        // And the textures to load before objects show them.
        let used: Vec<Texture> = surfaces
            .iter()
            .flat_map(Surface::textures)
            .chain(&frame.preload)
            .cloned()
            .collect();
        self.textures.retain(|made| used.contains(&made.texture));
        for texture in used {
            if !self.textures.iter().any(|made| made.texture == texture) {
                let made = upload(&gpu, &texture);
                self.textures.push(Made::new(texture, made));
            }
        }
        let mut before = std::mem::take(&mut self.surfaces);
        for surface in &surfaces {
            if self.surfaces.iter().any(|made| made.surface == *surface) {
                continue;
            }
            let surfacing = match before.iter().position(|made| made.surface == *surface) {
                Some(index) => before.swap_remove(index),
                None => self.surfacing(surface),
            };
            self.surfaces.push(surfacing);
        }
        let shaders: Vec<Shader> = surfaces
            .iter()
            .filter_map(|surface| surface.shader.clone())
            .collect();
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
                let shading = Shading::new(&gpu, &self.pipeline_layout, self.samples, Some(shader));
                self.shadings.push(shading);
            }
        }

        let size = gpu.size();
        let view = View {
            camera: frame.camera,
            view_projection: view_projection(frame, size.aspect()),
            size,
        };
        gpu.write_buffer(
            &self.uniforms,
            0,
            &uniforms(frame, &view, &lighting(&frame.lights)),
        );

        let background = frame.background;
        let clear = LoadOp::Clear(Color {
            r: f64::from(background.red),
            g: f64::from(background.green),
            b: f64::from(background.blue),
            a: f64::from(background.alpha),
        });
        let color = if self.samples > 1 {
            ColorAttachment::canvas_multisampled(self.samples, clear)
        } else {
            ColorAttachment::canvas(clear)
        };
        let mut pass = gpu.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[color],
            depth_stencil_attachment: Some(DepthAttachment::canvas(DEPTH, LoadOp::Clear(1.0))),
        });

        let shadings = &self.shadings;
        let shading = |surface: &Surface| {
            shadings
                .iter()
                .find(|shading| shading.shader == surface.shader)
                .unwrap_or(&shadings[0])
        };
        let made_surfaces = &self.surfaces;
        let surfacing = |surface: &Surface| {
            made_surfaces
                .iter()
                .find(|made| made.surface == *surface)
                .map(|made| &made.group)
        };

        for group in &self.groups {
            let (Some(instances), Some(textures)) = (&group.instances, surfacing(&group.surface))
            else {
                continue;
            };
            pass.set_pipeline(&shading(&group.surface).opaque);
            pass.set_bind_group(0, &self.frame_group);
            pass.set_bind_group(1, textures);
            pass.set_vertex_buffer(0, &group.vertices, 0);
            pass.set_vertex_buffer(1, instances, 0);
            pass.set_index_buffer(&group.indices, IndexFormat::Uint32, 0);
            pass.draw_indexed(0..group.index_count, 0, 0..group.count);
        }

        for (layer, made) in &mut self.layers {
            if let Some(drawn) = frame.layers.iter().find(|drawn| drawn.layer == *layer) {
                layer.draw_gpu(&mut **made, &gpu, &mut pass, &*drawn.data, &view);
            }
        }

        // Behind each other, they hide nothing: the farthest is drawn first.
        for run in &runs {
            let (Some(group), Some(textures)) =
                (self.see_through.get(run.mesh), surfacing(&run.surface))
            else {
                continue;
            };
            let Some(instances) = &group.instances else {
                continue;
            };
            pass.set_pipeline(&shading(&run.surface).see_through);
            pass.set_bind_group(0, &self.frame_group);
            pass.set_bind_group(1, textures);
            pass.set_vertex_buffer(0, &group.vertices, 0);
            pass.set_vertex_buffer(1, instances, 0);
            pass.set_index_buffer(&group.indices, IndexFormat::Uint32, 0);
            pass.draw_indexed(0..group.index_count, 0, run.first..run.first + run.count);
        }
    }

    /// The bind group of `surface`, with the textures made for it, which are made before.
    fn surfacing(&self, surface: &Surface) -> Surfacing {
        let made = |texture: &Option<Texture>| {
            texture
                .as_ref()
                .and_then(|texture| self.textures.iter().find(|made| made.texture == *texture))
                .map_or((&self.white, &self.sharp), |made| {
                    let sampler = if made.texture.data().sharp {
                        &self.sharp
                    } else {
                        &self.smooth
                    };
                    (&made.made, sampler)
                })
        };
        let bumped = self.gpu.create_buffer_init(
            BufferUsages::UNIFORM,
            &[f32::from(u8::from(surface.normal.is_some())), 0.0, 0.0, 0.0],
        );
        let [
            (color, color_sampler),
            (normal, normal_sampler),
            (height, height_sampler),
        ] = [&surface.color, &surface.normal, &surface.height].map(made);
        let group = self.gpu.create_bind_group(
            &self.surface_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Texture(color),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(color_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Texture(normal),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(normal_sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Texture(height),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::Sampler(height_sampler),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::Buffer(&bumped),
                },
            ],
        );
        Surfacing {
            surface: surface.clone(),
            group,
            _bumped: bumped,
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
}

/// The uniforms of the frame, as `Frame` of [`SHADER`] has them.
fn uniforms(frame: &Frame, view: &View, lighting: &Lighting) -> Vec<f32> {
    let mut data = Vec::with_capacity(UNIFORMS);
    data.extend(view.view_projection.to_cols_array());
    data.extend(frame.camera.eye.to_array());
    data.push(frame.time);
    data.extend(lighting.ambient.to_array());
    data.push(f32::from(u8::from(frame.relief)));
    data.push(lighting.directional.len().min(MAX_DIRECTIONAL) as f32);
    data.push(lighting.point.len().min(MAX_POINT) as f32);
    // To the lights, at a multiple of 16 bytes
    data.extend([0.0, 0.0]);
    for index in 0..4 {
        let (direction, color) = lighting.directional.get(index).copied().unwrap_or_default();
        data.extend(direction.extend(0.0).to_array());
        data.extend(color.extend(0.0).to_array());
    }
    for index in 0..4 {
        let (position, range, color) = lighting.point.get(index).copied().unwrap_or_default();
        data.extend(position.extend(range).to_array());
        data.extend(color.extend(0.0).to_array());
    }
    data
}

/// `texture` on the context: the pixels at once, an image from a URL when the browser has it,
/// white until then, with smaller images of it unless it's sharp.
fn upload(gpu: &WebGpu, texture: &Texture) -> vertigo::webgpu::Texture {
    let data = texture.data();
    match &data.image {
        Image::Url(url) => gpu.create_texture_from_url(
            url,
            &ImageOptions {
                mipmaps: !data.sharp,
                ..ImageOptions::default()
            },
        ),
        Image::Pixels {
            width,
            height,
            pixels,
        } => {
            let size = Size {
                width: *width,
                height: *height,
            };
            let made = gpu.create_texture(&TextureDescriptor {
                size,
                mip_level_count: if data.sharp { 1 } else { size.mip_levels() },
                sample_count: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
            });
            gpu.write_texture(&made, pixels);
            if !data.sharp {
                gpu.generate_mipmaps(&made);
            }
            made
        }
    }
}

/// A mesh on the context, and the buffer of its instances.
struct Group {
    mesh: Mesh,
    surface: Surface,
    vertices: Buffer,
    indices: Buffer,
    index_count: u32,
    /// None until there are instances.
    instances: Option<Buffer>,
    /// What the instances' buffer holds, to send only what changed.
    sent: Vec<f32>,
    count: u32,
}

impl Group {
    fn new(gpu: &WebGpu, mesh: Mesh, surface: Surface) -> Self {
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

        Self {
            vertices: gpu.create_buffer_init(BufferUsages::VERTEX, &corners),
            indices: gpu.create_buffer_init(BufferUsages::INDEX, &data.indices),
            index_count: data.indices.len() as u32,
            mesh,
            surface,
            instances: None,
            sent: Vec::new(),
            count: 0,
        }
    }

    /// The instances, sent only if they aren't what the buffer holds, in a buffer made anew
    /// when they don't fit in the one there is.
    fn send(&mut self, gpu: &WebGpu, data: Vec<f32>) {
        if data == self.sent && self.instances.is_some() {
            return;
        }

        let bytes = (data.len() * 4) as u32;
        match &self.instances {
            Some(instances) if instances.size() >= bytes => {
                gpu.write_buffer(instances, 0, &data);
            }
            _ => {
                self.instances = (bytes > 0).then(|| {
                    gpu.create_buffer_init(BufferUsages::VERTEX | BufferUsages::COPY_DST, &data)
                });
            }
        }
        self.count = (data.len() / INSTANCE) as u32;
        self.sent = data;
    }
}

#[cfg(test)]
mod tests {
    use glam::{Vec3, Vec4};

    use super::{DEPTH_ZERO_TO_ONE, UNIFORMS, source, uniforms};
    use crate::{Camera, Color, Light, Shader, View, light::lighting, render::Frame};

    #[test]
    fn a_shaders_code_goes_into_the_shaders_of_wgsl() {
        let shader = Shader::new()
            .vertex_wgsl("position.y += sin(time);")
            .fragment_wgsl("pixel.r = 1.0;");
        let code = source(Some(&shader), true);
        let before = |first: &str, then: &str| {
            let (Some(first), Some(then)) = (code.find(first), code.find(then)) else {
                panic!("no {first} or {then} in {code}");
            };
            first < then
        };
        assert!(before("position.y += sin(time);", "let placed ="));
        assert!(before("pixel = vec4f(surface.rgb", "pixel.r = 1.0;"));
        assert!(code.contains("const CUTOUT: bool = true;"));

        let plain = source(None, false);
        assert!(plain.contains("const CUTOUT: bool = false;"));
        assert!(!plain.contains("{vertex}") && !plain.contains("{fragment}"));
    }

    #[test]
    fn the_depths_of_webgpu_go_from_0_to_1() {
        let near = DEPTH_ZERO_TO_ONE * Vec4::new(0.0, 0.0, -1.0, 1.0);
        let far = DEPTH_ZERO_TO_ONE * Vec4::new(0.0, 0.0, 1.0, 1.0);
        assert_eq!((near.z, far.z), (0.0, 1.0));
    }

    #[test]
    fn the_uniforms_are_where_the_shader_reads_them() {
        let camera = Camera::default().looking_at(Vec3::new(1.0, 2.0, 3.0), Vec3::ZERO);
        let lights = vec![
            Light::ambient(0.5),
            Light::directional(Vec3::NEG_Y),
            Light::point(Vec3::new(4.0, 5.0, 6.0)),
        ];
        let frame = Frame {
            camera,
            background: Color::BLACK,
            lights,
            time: 7.0,
            relief: true,
            preload: Vec::new(),
            instances: Vec::new(),
            layers: Vec::new(),
        };
        let view = View {
            camera,
            view_projection: glam::Mat4::IDENTITY,
            size: vertigo::webgl::Size::default(),
        };
        let data = uniforms(&frame, &view, &lighting(&frame.lights));
        assert_eq!(data.len(), UNIFORMS);
        assert_eq!(&data[16..20], &[1.0, 2.0, 3.0, 7.0], "the eye and the time");
        assert_eq!(data[23], 1.0, "relief");
        assert_eq!(&data[24..26], &[1.0, 1.0], "a light of each kind");
        assert_eq!(
            &data[28..31],
            &[0.0, -1.0, 0.0],
            "its direction, at 112 bytes"
        );
        assert_eq!(
            &data[60..64],
            &[4.0, 5.0, 6.0, 10.0],
            "the point and its range"
        );
    }
}
