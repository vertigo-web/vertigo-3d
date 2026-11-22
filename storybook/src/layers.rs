//! Layers: WebGL or WebGPU of their own in a scene, whichever draws it. A grid of lines with the
//! axes, its spacing chosen with buttons, and a fountain of glowing points moving in a shader with
//! the frames, which blends them as it likes: the scene sets its own way again after it. A cube
//! hides the lines behind it, and a pane of glass tints the fountain behind it.
//!
//! WebGPU has no points bigger than a pixel: its fountain draws a square of two triangles for each
//! drop, as big as WebGL's point, and leaves out its corners.

use vertigo::{
    DomNode, Value, bind, dom,
    webgl::{
        BlendFactor, Buffer, BufferTarget, BufferUsage, Capability, DataType, Primitive, Program,
        UniformLocation, VertexArray, WebGl,
    },
    webgpu::{
        self, BindGroup, BindGroupEntry, BindingResource, BlendComponent, BlendOperation,
        BlendState, BufferUsages, ColorTargetState, CompareFunction, DepthStencilState,
        FragmentState, MultisampleState, PrimitiveState, PrimitiveTopology, RenderPass,
        RenderPipeline, RenderPipelineDescriptor, VertexAttribute, VertexBufferLayout,
        VertexFormat, VertexState, VertexStepMode, WebGpu,
    },
};
use vertigo_3d::{
    Camera, Color, Layer, Material, Mesh, Object, Scene, SceneParams, Targets, Transform, View,
    frame_time, math::Vec3,
};

use crate::background::css_frame;

/// How far the grid goes from the middle, along X and Z.
const REACH: f32 = 5.0;

/// A program, the location of its `view_projection`, and its vertices.
struct Lines {
    program: Program,
    view_projection: UniformLocation,
    vertices: VertexArray,
    buffer: Buffer,
    /// The spacing the buffer has lines for, and how many corners they have.
    built: Option<(f32, u32)>,
}

const LINES_VERTEX: &str = "#version 300 es
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;
uniform mat4 view_projection;
out vec3 shade;
void main() {
    gl_Position = view_projection * vec4(position, 1.0);
    shade = color;
}";

const LINES_FRAGMENT: &str = "#version 300 es
precision mediump float;
in vec3 shade;
out vec4 pixel;
void main() { pixel = vec4(shade, 1.0); }";

fn make_lines(gl: &WebGl) -> Lines {
    let program = gl.create_program(LINES_VERTEX, LINES_FRAGMENT);
    let view_projection = gl.uniform_location(&program, "view_projection");
    let vertices = gl.create_vertex_array();
    gl.bind_vertex_array(Some(&vertices));
    let buffer = gl.create_buffer();
    gl.bind_buffer(BufferTarget::Array, Some(&buffer));
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer(0, 3, DataType::Float, false, 24, 0);
    gl.enable_vertex_attrib_array(1);
    gl.vertex_attrib_pointer(1, 3, DataType::Float, false, 24, 12);
    gl.bind_vertex_array(None);
    Lines {
        program,
        view_projection,
        vertices,
        buffer,
        built: None,
    }
}

/// The grid's lines every `spacing`, grey, and the axes: X red, Y green, Z blue. Position and
/// color of each corner.
fn grid(spacing: f32) -> Vec<f32> {
    let grey = [0.45, 0.45, 0.45];
    let mut corners = Vec::new();
    let mut line = |from: [f32; 3], to: [f32; 3], color: [f32; 3]| {
        corners.extend(from.iter().chain(&color));
        corners.extend(to.iter().chain(&color));
    };

    let count = (REACH / spacing).floor() as i32;
    for step in -count..=count {
        let at = step as f32 * spacing;
        if step != 0 {
            line([at, 0.0, -REACH], [at, 0.0, REACH], grey);
            line([-REACH, 0.0, at], [REACH, 0.0, at], grey);
        }
    }
    line([-REACH, 0.0, 0.0], [REACH, 0.0, 0.0], [1.0, 0.2, 0.2]);
    line([0.0, 0.0, 0.0], [0.0, 2.5, 0.0], [0.2, 1.0, 0.2]);
    line([0.0, 0.0, -REACH], [0.0, 0.0, REACH], [0.3, 0.5, 1.0]);
    corners
}

fn draw_lines(lines: &mut Lines, gl: &WebGl, spacing: &f32, view: &View) {
    // Built again only when the spacing changed: the camera changes nothing in it.
    let count = match lines.built {
        Some((built, count)) if built == *spacing => count,
        _ => {
            let corners = grid(*spacing);
            gl.bind_buffer(BufferTarget::Array, Some(&lines.buffer));
            gl.buffer_data(BufferTarget::Array, &corners, BufferUsage::StaticDraw);
            let count = (corners.len() / 6) as u32;
            lines.built = Some((*spacing, count));
            count
        }
    };

    gl.use_program(Some(&lines.program));
    gl.uniform_matrix_4f(
        &lines.view_projection,
        &view.view_projection.to_cols_array(),
    );
    gl.bind_vertex_array(Some(&lines.vertices));
    gl.draw_arrays(Primitive::Lines, 0, count);
}

/// Points thrown up from where the fountain is, each with its way and its start in a buffer, and
/// where they are at a time worked out in the shader.
struct Fountain {
    program: Program,
    view_projection: UniformLocation,
    time: UniformLocation,
    vertices: VertexArray,
    _buffer: Buffer,
}

const DROPS: u32 = 400;

const FOUNTAIN_VERTEX: &str = "#version 300 es
layout(location = 0) in vec3 drop;
uniform mat4 view_projection;
uniform float time;
out float age;
void main() {
    // Each drop starts again every 2 seconds, from its own moment.
    age = fract(time / 2.0 + drop.z);
    float t = age * 2.0;
    vec3 position = vec3(2.5, 0.0, -1.5)
        + vec3(drop.x, 4.0, drop.y) * t
        + vec3(0.0, -4.0, 0.0) * t * t;
    gl_Position = view_projection * vec4(position, 1.0);
    gl_PointSize = 60.0 / gl_Position.w;
}";

const FOUNTAIN_FRAGMENT: &str = "#version 300 es
precision mediump float;
in float age;
out vec4 pixel;
void main() {
    if (length(gl_PointCoord - 0.5) > 0.5) {
        discard;
    }
    // Added to what is there: where drops are many, it glows.
    pixel = vec4(mix(vec3(0.15, 0.35, 0.6), vec3(0.5), age), 1.0);
}";

/// Each drop's way across and along, and its start: the ways spread round, by the golden angle,
/// and the starts spread over the time.
fn drops() -> Vec<f32> {
    (0..DROPS)
        .flat_map(|drop| {
            let angle = drop as f32 * 2.399_963;
            let spread = 0.3 + 0.5 * ((drop * 7919) % 100) as f32 / 100.0;
            [
                angle.cos() * spread,
                angle.sin() * spread,
                drop as f32 / DROPS as f32,
            ]
        })
        .collect()
}

fn make_fountain(gl: &WebGl) -> Fountain {
    let program = gl.create_program(FOUNTAIN_VERTEX, FOUNTAIN_FRAGMENT);
    let view_projection = gl.uniform_location(&program, "view_projection");
    let time = gl.uniform_location(&program, "time");

    let drops = drops();
    let vertices = gl.create_vertex_array();
    gl.bind_vertex_array(Some(&vertices));
    let buffer = gl.create_buffer();
    gl.bind_buffer(BufferTarget::Array, Some(&buffer));
    gl.buffer_data(BufferTarget::Array, &drops, BufferUsage::StaticDraw);
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer(0, 3, DataType::Float, false, 0, 0);
    gl.bind_vertex_array(None);

    Fountain {
        program,
        view_projection,
        time,
        vertices,
        _buffer: buffer,
    }
}

fn draw_fountain(fountain: &mut Fountain, gl: &WebGl, time: &f32, view: &View) {
    gl.use_program(Some(&fountain.program));
    gl.uniform_matrix_4f(
        &fountain.view_projection,
        &view.view_projection.to_cols_array(),
    );
    gl.uniform_1f(&fountain.time, *time);
    gl.bind_vertex_array(Some(&fountain.vertices));
    // The drops hidden behind what is there, but not by each other.
    gl.enable(Capability::Blend);
    gl.blend_func(BlendFactor::One, BlendFactor::One);
    gl.depth_mask(false);
    gl.draw_arrays(Primitive::Points, 0, DROPS);
}

/// The grid with WebGPU: its pipeline, the uniforms of its camera, and its lines.
struct GpuLines {
    pipeline: RenderPipeline,
    uniforms: webgpu::Buffer,
    group: BindGroup,
    /// The lines for the spacing, and how many corners they have.
    built: Option<(f32, webgpu::Buffer, u32)>,
}

const LINES_WGSL: &str = "
@group(0) @binding(0) var<uniform> view_projection: mat4x4f;

struct Corner {
    @builtin(position) position: vec4f,
    @location(0) shade: vec3f,
};

@vertex fn vertex(@location(0) position: vec3f, @location(1) color: vec3f) -> Corner {
    return Corner(view_projection * vec4f(position, 1.0), color);
}

@fragment fn fragment(corner: Corner) -> @location(0) vec4f {
    return vec4f(corner.shade, 1.0);
}
";

fn make_gpu_lines(gpu: &WebGpu, targets: &Targets) -> GpuLines {
    let module = gpu.create_shader_module(LINES_WGSL);
    let pipeline = gpu.create_render_pipeline(&RenderPipelineDescriptor {
        primitive: PrimitiveState {
            topology: PrimitiveTopology::LineList,
            ..PrimitiveState::default()
        },
        depth_stencil: Some(DepthStencilState {
            format: targets.depth,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
        }),
        multisample: MultisampleState {
            count: targets.samples,
        },
        ..RenderPipelineDescriptor::new(
            VertexState {
                module: &module,
                entry_point: "vertex",
                buffers: &[VertexBufferLayout {
                    array_stride: 24,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            Some(FragmentState {
                module: &module,
                entry_point: "fragment",
                targets: &[ColorTargetState::canvas()],
            }),
        )
    });
    let uniforms = gpu.create_buffer(64, BufferUsages::UNIFORM | BufferUsages::COPY_DST);
    let group = gpu.create_bind_group(
        &pipeline.get_bind_group_layout(0),
        &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::Buffer(&uniforms),
        }],
    );
    GpuLines {
        pipeline,
        uniforms,
        group,
        built: None,
    }
}

fn draw_gpu_lines(
    lines: &mut GpuLines,
    gpu: &WebGpu,
    pass: &mut RenderPass<'_>,
    spacing: &f32,
    view: &View,
) {
    // Built again only when the spacing changed: the camera changes nothing in it.
    if lines
        .built
        .as_ref()
        .is_none_or(|(built, ..)| built != spacing)
    {
        let corners = grid(*spacing);
        let buffer = gpu.create_buffer_init(BufferUsages::VERTEX, &corners);
        lines.built = Some((*spacing, buffer, (corners.len() / 6) as u32));
    }
    let Some((_, buffer, count)) = &lines.built else {
        return;
    };

    gpu.write_buffer(&lines.uniforms, 0, &view.view_projection.to_cols_array());
    pass.set_pipeline(&lines.pipeline);
    pass.set_bind_group(0, &lines.group);
    pass.set_vertex_buffer(0, buffer, 0);
    pass.draw(0..*count, 0..1);
}

/// The fountain with WebGPU: a square of each drop, as big as WebGL's point of it.
struct GpuFountain {
    pipeline: RenderPipeline,
    /// The camera, the time and the size of the canvas.
    uniforms: webgpu::Buffer,
    group: BindGroup,
    drops: webgpu::Buffer,
}

const FOUNTAIN_WGSL: &str = "
struct Uniforms {
    view_projection: mat4x4f,
    time: f32,
    width: f32,
    height: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct Corner {
    @builtin(position) position: vec4f,
    @location(0) age: f32,
    // From -1 to 1 across the square and down it
    @location(1) at: vec2f,
};

@vertex fn vertex(@builtin(vertex_index) index: u32, @location(0) drop: vec3f) -> Corner {
    // Each drop starts again every 2 seconds, from its own moment.
    let age = fract(uniforms.time / 2.0 + drop.z);
    let t = age * 2.0;
    let position = vec3f(2.5, 0.0, -1.5)
        + vec3f(drop.x, 4.0, drop.y) * t
        + vec3f(0.0, -4.0, 0.0) * t * t;
    var clip = uniforms.view_projection * vec4f(position, 1.0);
    let corners = array(vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0),
        vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0));
    let at = corners[index];
    // 60 / w pixels across, as WebGL's point
    let pixels = 60.0 / clip.w;
    clip += vec4f(at * pixels / vec2f(uniforms.width, uniforms.height) * clip.w, 0.0, 0.0);
    return Corner(clip, age, at);
}

@fragment fn fragment(corner: Corner) -> @location(0) vec4f {
    if (length(corner.at) > 1.0) {
        discard;
    }
    // Added to what is there: where drops are many, it glows.
    return vec4f(mix(vec3f(0.15, 0.35, 0.6), vec3f(0.5), corner.age), 1.0);
}
";

fn make_gpu_fountain(gpu: &WebGpu, targets: &Targets) -> GpuFountain {
    let module = gpu.create_shader_module(FOUNTAIN_WGSL);
    let adding = BlendComponent {
        src_factor: webgpu::BlendFactor::One,
        dst_factor: webgpu::BlendFactor::One,
        operation: BlendOperation::Add,
    };
    let pipeline = gpu.create_render_pipeline(&RenderPipelineDescriptor {
        // The drops hidden behind what is there, but not by each other.
        depth_stencil: Some(DepthStencilState {
            format: targets.depth,
            depth_write_enabled: false,
            depth_compare: CompareFunction::Less,
        }),
        multisample: MultisampleState {
            count: targets.samples,
        },
        ..RenderPipelineDescriptor::new(
            VertexState {
                module: &module,
                entry_point: "vertex",
                buffers: &[VertexBufferLayout {
                    array_stride: 12,
                    step_mode: VertexStepMode::Instance,
                    attributes: &[VertexAttribute {
                        format: VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
            },
            Some(FragmentState {
                module: &module,
                entry_point: "fragment",
                targets: &[ColorTargetState {
                    blend: Some(BlendState {
                        color: adding,
                        alpha: adding,
                    }),
                    ..ColorTargetState::canvas()
                }],
            }),
        )
    });
    let uniforms = gpu.create_buffer(80, BufferUsages::UNIFORM | BufferUsages::COPY_DST);
    let group = gpu.create_bind_group(
        &pipeline.get_bind_group_layout(0),
        &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::Buffer(&uniforms),
        }],
    );
    GpuFountain {
        pipeline,
        uniforms,
        group,
        drops: gpu.create_buffer_init(BufferUsages::VERTEX, &drops()),
    }
}

fn draw_gpu_fountain(
    fountain: &mut GpuFountain,
    gpu: &WebGpu,
    pass: &mut RenderPass<'_>,
    time: &f32,
    view: &View,
) {
    let mut uniforms = view.view_projection.to_cols_array().to_vec();
    uniforms.extend([*time, view.size.width as f32, view.size.height as f32, 0.0]);
    gpu.write_buffer(&fountain.uniforms, 0, &uniforms);
    pass.set_pipeline(&fountain.pipeline);
    pass.set_bind_group(0, &fountain.group);
    pass.set_vertex_buffer(0, &fountain.drops, 0);
    pass.draw(0..6, 0..DROPS);
}

pub fn render() -> DomNode {
    let spacing = Value::new(1.0_f32);
    let buttons: Vec<DomNode> = [("Fine", 1.0), ("Coarse", 2.0)]
        .into_iter()
        .map(|(name, value)| {
            let on_click = bind!(spacing, |_| spacing.set(value));
            dom! { <button on_click={on_click}>{name}</button> }
        })
        .collect();

    let grid = Layer::new(spacing.clone(), make_lines, draw_lines).or(Layer::webgpu(
        spacing,
        make_gpu_lines,
        draw_gpu_lines,
    ));
    let time = frame_time();
    let fountain = Layer::new(time.clone(), make_fountain, draw_fountain).or(Layer::webgpu(
        time,
        make_gpu_fountain,
        draw_gpu_fountain,
    ));

    let cube = Object::new(Mesh::cube())
        .transform(Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)))
        .material(Material::lit(Color::rgb(0.9, 0.6, 0.2)));
    let glass = Object::new(Mesh::cube())
        .transform(
            Transform::from_translation(Vec3::new(2.5, 1.0, 0.5))
                .with_scale(Vec3::new(2.0, 2.0, 0.05)),
        )
        .material(Material::color(Color::rgba(0.3, 0.5, 1.0, 0.4)));

    let camera = Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 4.0, 8.0), Vec3::new(0.0, 0.5, 0.0));
    let params = SceneParams::default()
        .background(Color::rgb(0.05, 0.05, 0.1))
        .layers(vec![grid, fountain]);

    dom! {
        <div>
            <p>"Grid: " {..buttons}</p>
            <div css={css_frame()}>
                <Scene
                    camera={Value::new(camera).to_computed()}
                    objects={Value::new(vec![cube, glass]).to_computed()}
                    {params}
                />
            </div>
        </div>
    }
}
