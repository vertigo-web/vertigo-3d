use std::{any::Any, fmt, rc::Rc};

use glam::Mat4;
use vertigo::{
    Computed, Context,
    webgl::{Size, WebGl},
    webgpu::{RenderPass, TextureFormat, WebGpu},
};

use crate::Camera;

/// Drawing of its own in a scene, with [`vertigo::webgl`] or [`vertigo::webgpu`]: what the objects
/// can't be, f. ex. lines, points, or a shader of its own. Given to a scene with
/// [`SceneParams::layers`](crate::SceneParams::layers). A scene with a layer of WebGL alone is
/// drawn with WebGL, see [`Layer::webgpu`] and [`Layer::or`] for WebGPU, and
/// [`Backend`](crate::Backend).
///
/// A layer is made of three things:
/// - `data` - what it draws, a [`Computed`] or anything becoming one: when it changes, the scene
///   is drawn again,
/// - `make` - makes the layer's programs, buffers and textures on the scene's context, before it
///   is first drawn, and again when the browser gives a lost context back, as all were gone,
/// - `draw` - draws `data` with what `make` made, with the scene's camera, see [`View`].
///
/// A layer is drawn after the scene's opaque objects, in the order of the layers, and before the
/// see-through ones: it hides what is behind it, and is seen through glass. It finds the depth
/// test and the culling of back faces on, and blending off, and may change any of them, but not
/// the viewport: the scene sets what it needs again after it.
///
/// ```rust
/// use vertigo::{
///     Value,
///     webgl::{BufferTarget, BufferUsage, DataType, Primitive, WebGl},
/// };
/// use vertigo_3d::{Layer, SceneParams};
///
/// // A line from the origin up, as high as `height`.
/// let height = Value::new(2.0_f32);
/// let line = Layer::new(
///     height.clone(),
///     |gl: &WebGl| {
///         let program = gl.create_program(
///             "#version 300 es
///             layout(location = 0) in float up;
///             uniform mat4 view_projection;
///             uniform float height;
///             void main() { gl_Position = view_projection * vec4(0.0, up * height, 0.0, 1.0); }",
///             "#version 300 es
///             precision mediump float;
///             out vec4 pixel;
///             void main() { pixel = vec4(1.0, 1.0, 0.0, 1.0); }",
///         );
///         let view_projection = gl.uniform_location(&program, "view_projection");
///         let height = gl.uniform_location(&program, "height");
///         let vertices = gl.create_vertex_array();
///         gl.bind_vertex_array(Some(&vertices));
///         let buffer = gl.create_buffer();
///         gl.bind_buffer(BufferTarget::Array, Some(&buffer));
///         gl.buffer_data(BufferTarget::Array, &[0.0_f32, 1.0], BufferUsage::StaticDraw);
///         gl.enable_vertex_attrib_array(0);
///         gl.vertex_attrib_pointer(0, 1, DataType::Float, false, 0, 0);
///         (program, view_projection, height, vertices, buffer)
///     },
///     |(program, view_projection, at_height, vertices, _), gl, height: &f32, view| {
///         gl.use_program(Some(program));
///         gl.uniform_matrix_4f(view_projection, &view.view_projection.to_cols_array());
///         gl.uniform_1f(at_height, *height);
///         gl.bind_vertex_array(Some(vertices));
///         gl.draw_arrays(Primitive::Lines, 0, 2);
///     },
/// );
///
/// let params = SceneParams::default().layers(vec![line]);
/// // Higher it goes, and the scene showing it draws it so.
/// height.set(3.0);
/// ```
#[derive(Clone)]
pub struct Layer {
    inner: Rc<Sides>,
}

/// How a layer draws with WebGL and with WebGPU, as far as it does.
struct Sides {
    gl: Option<Rc<dyn GlLayering>>,
    gpu: Option<Rc<dyn GpuLayering>>,
}

/// The scene as a layer draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub camera: Camera,
    /// The camera's view and projection, for the canvas's aspect: from a point of the scene to the
    /// canvas, as the objects are drawn, with the depths WebGL has, -1 to 1, or WebGPU, 0 to 1.
    pub view_projection: Mat4,
    /// Of the canvas's bitmap, in device pixels.
    pub size: Size,
}

/// What a layer drawn with WebGPU draws on, for its pipelines, see [`Layer::webgpu`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Targets {
    /// Of the canvas: [`TextureFormat::Canvas`].
    pub color: TextureFormat,
    /// Of the scene's depth texture, which the pipelines test against:
    /// [`TextureFormat::Depth24Plus`].
    pub depth: TextureFormat,
    /// Samples a pixel, 4 with smooth edges, see
    /// [`SceneParams::antialias`](crate::SceneParams::antialias): the pipelines' `multisample`.
    pub samples: u32,
}

impl Layer {
    /// A layer drawing `data` with WebGL: with `draw`, with what `make` made, see [`Layer`].
    pub fn new<T, S>(
        data: impl Into<Computed<T>>,
        make: impl Fn(&WebGl) -> S + 'static,
        draw: impl Fn(&mut S, &WebGl, &T, &View) + 'static,
    ) -> Self
    where
        T: Clone + PartialEq + 'static,
        S: 'static,
    {
        Self {
            inner: Rc::new(Sides {
                gl: Some(Rc::new(Typed {
                    data: data.into(),
                    make: Box::new(make),
                    draw: Box::new(draw),
                })),
                gpu: None,
            }),
        }
    }

    /// A layer drawing `data` with WebGPU: with `draw`, in the scene's render pass, with what
    /// `make` made for the scene's [`Targets`]. Its pipelines draw on those, and test the depths
    /// of what is drawn before it. It writes its buffers with the [`WebGpu`] it gets, which go
    /// to the GPU before the pass. The pass has the layer's pipeline and bind groups only.
    ///
    /// ```rust
    /// use vertigo::{
    ///     Value,
    ///     webgpu::{
    ///         BindGroupEntry, BindingResource, BufferUsages, ColorTargetState, CompareFunction,
    ///         DepthStencilState, FragmentState, MultisampleState, PrimitiveState,
    ///         PrimitiveTopology, RenderPipelineDescriptor, VertexState,
    ///     },
    /// };
    /// use vertigo_3d::{Layer, SceneParams};
    ///
    /// // A line from the origin up, as high as `height`.
    /// let height = Value::new(2.0_f32);
    /// let line = Layer::webgpu(
    ///     height.clone(),
    ///     |gpu, targets| {
    ///         let shader = gpu.create_shader_module("
    ///             struct Line { view_projection: mat4x4f, height: f32 };
    ///             @group(0) @binding(0) var<uniform> line: Line;
    ///             @vertex fn vertex(@builtin(vertex_index) up: u32) -> @builtin(position) vec4f {
    ///                 return line.view_projection * vec4f(0.0, f32(up) * line.height, 0.0, 1.0);
    ///             }
    ///             @fragment fn fragment() -> @location(0) vec4f { return vec4f(1.0, 1.0, 0.0, 1.0); }
    ///         ");
    ///         let pipeline = gpu.create_render_pipeline(&RenderPipelineDescriptor {
    ///             primitive: PrimitiveState {
    ///                 topology: PrimitiveTopology::LineList,
    ///                 ..PrimitiveState::default()
    ///             },
    ///             depth_stencil: Some(DepthStencilState {
    ///                 format: targets.depth,
    ///                 depth_write_enabled: true,
    ///                 depth_compare: CompareFunction::Less,
    ///             }),
    ///             multisample: MultisampleState { count: targets.samples },
    ///             ..RenderPipelineDescriptor::new(
    ///                 VertexState { module: &shader, entry_point: "vertex", buffers: &[] },
    ///                 Some(FragmentState {
    ///                     module: &shader,
    ///                     entry_point: "fragment",
    ///                     targets: &[ColorTargetState::canvas()],
    ///                 }),
    ///             )
    ///         });
    ///         let uniforms = gpu.create_buffer(80, BufferUsages::UNIFORM | BufferUsages::COPY_DST);
    ///         let group = gpu.create_bind_group(&pipeline.get_bind_group_layout(0), &[BindGroupEntry {
    ///             binding: 0,
    ///             resource: BindingResource::Buffer(&uniforms),
    ///         }]);
    ///         (pipeline, uniforms, group)
    ///     },
    ///     |(pipeline, uniforms, group), gpu, pass, height: &f32, view| {
    ///         let mut data = view.view_projection.to_cols_array().to_vec();
    ///         data.push(*height);
    ///         gpu.write_buffer(uniforms, 0, &data);
    ///         pass.set_pipeline(pipeline);
    ///         pass.set_bind_group(0, group);
    ///         pass.draw(0..2, 0..1);
    ///     },
    /// );
    ///
    /// let params = SceneParams::default().layers(vec![line]);
    /// ```
    pub fn webgpu<T, S>(
        data: impl Into<Computed<T>>,
        make: impl Fn(&WebGpu, &Targets) -> S + 'static,
        draw: impl Fn(&mut S, &WebGpu, &mut RenderPass<'_>, &T, &View) + 'static,
    ) -> Self
    where
        T: Clone + PartialEq + 'static,
        S: 'static,
    {
        Self {
            inner: Rc::new(Sides {
                gl: None,
                gpu: Some(Rc::new(GpuTyped {
                    data: data.into(),
                    make: Box::new(make),
                    draw: Box::new(draw),
                })),
            }),
        }
    }

    /// This layer, drawn as `other` where this has no way of the scene's: one layer of a way for
    /// WebGL and a way for WebGPU, f. ex. `Layer::new(...).or(Layer::webgpu(...))`.
    pub fn or(self, other: Layer) -> Self {
        Self {
            inner: Rc::new(Sides {
                gl: self.inner.gl.clone().or_else(|| other.inner.gl.clone()),
                gpu: self.inner.gpu.clone().or_else(|| other.inner.gpu.clone()),
            }),
        }
    }

    pub(crate) fn has_webgpu(&self) -> bool {
        self.inner.gpu.is_some()
    }

    /// Its data now, of both ways of drawing it, `context` depending on them.
    pub(crate) fn read(&self, context: &Context) -> Rc<dyn Data> {
        Rc::new(Both(
            self.inner.gl.as_ref().map(|gl| gl.read(context)),
            self.inner.gpu.as_ref().map(|gpu| gpu.read(context)),
        ))
    }

    /// What it draws with WebGL with, if it does.
    pub(crate) fn make(&self, gl: &WebGl) -> Option<Box<dyn Any>> {
        self.inner.gl.as_ref().map(|layer| layer.make(gl))
    }

    pub(crate) fn draw(&self, made: &mut dyn Any, gl: &WebGl, data: &dyn Data, view: &View) {
        if let (Some(layer), Some(Both(Some(data), _))) =
            (&self.inner.gl, data.as_any().downcast_ref::<Both>())
        {
            layer.draw(made, gl, &**data, view);
        }
    }

    /// What it draws with WebGPU with, if it does.
    pub(crate) fn make_gpu(&self, gpu: &WebGpu, targets: &Targets) -> Option<Box<dyn Any>> {
        self.inner
            .gpu
            .as_ref()
            .map(|layer| layer.make(gpu, targets))
    }

    pub(crate) fn draw_gpu(
        &self,
        made: &mut dyn Any,
        gpu: &WebGpu,
        pass: &mut RenderPass<'_>,
        data: &dyn Data,
        view: &View,
    ) {
        if let (Some(layer), Some(Both(_, Some(data)))) =
            (&self.inner.gpu, data.as_any().downcast_ref::<Both>())
        {
            layer.draw(made, gpu, pass, &**data, view);
        }
    }
}

impl PartialEq for Layer {
    /// The same layer, not one alike.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl fmt::Debug for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Layer").finish_non_exhaustive()
    }
}

/// A layer drawn with WebGL, whatever its data and what it makes.
trait GlLayering {
    fn read(&self, context: &Context) -> Rc<dyn Data>;
    fn make(&self, gl: &WebGl) -> Box<dyn Any>;
    fn draw(&self, made: &mut dyn Any, gl: &WebGl, data: &dyn Data, view: &View);
}

struct Typed<T: Clone + PartialEq + 'static, S> {
    data: Computed<T>,
    make: Box<dyn Fn(&WebGl) -> S>,
    draw: DrawFn<S, T>,
}

/// How a layer draws its data `T` with what it made, `S`.
type DrawFn<S, T> = Box<dyn Fn(&mut S, &WebGl, &T, &View)>;

impl<T: Clone + PartialEq + 'static, S: 'static> GlLayering for Typed<T, S> {
    fn read(&self, context: &Context) -> Rc<dyn Data> {
        Rc::new(self.data.get(context))
    }

    fn make(&self, gl: &WebGl) -> Box<dyn Any> {
        Box::new((self.make)(gl))
    }

    fn draw(&self, made: &mut dyn Any, gl: &WebGl, data: &dyn Data, view: &View) {
        if let (Some(made), Some(data)) = (made.downcast_mut::<S>(), data.as_any().downcast_ref()) {
            (self.draw)(made, gl, data, view);
        }
    }
}

/// A layer drawn with WebGPU, whatever its data and what it makes.
trait GpuLayering {
    fn read(&self, context: &Context) -> Rc<dyn Data>;
    fn make(&self, gpu: &WebGpu, targets: &Targets) -> Box<dyn Any>;
    fn draw(
        &self,
        made: &mut dyn Any,
        gpu: &WebGpu,
        pass: &mut RenderPass<'_>,
        data: &dyn Data,
        view: &View,
    );
}

struct GpuTyped<T: Clone + PartialEq + 'static, S> {
    data: Computed<T>,
    make: GpuMakeFn<S>,
    draw: GpuDrawFn<S, T>,
}

/// How a layer makes what it draws with, `S`, with WebGPU.
type GpuMakeFn<S> = Box<dyn Fn(&WebGpu, &Targets) -> S>;

/// How a layer draws its data `T` with what it made, `S`, with WebGPU.
type GpuDrawFn<S, T> = Box<dyn Fn(&mut S, &WebGpu, &mut RenderPass<'_>, &T, &View)>;

impl<T: Clone + PartialEq + 'static, S: 'static> GpuLayering for GpuTyped<T, S> {
    fn read(&self, context: &Context) -> Rc<dyn Data> {
        Rc::new(self.data.get(context))
    }

    fn make(&self, gpu: &WebGpu, targets: &Targets) -> Box<dyn Any> {
        Box::new((self.make)(gpu, targets))
    }

    fn draw(
        &self,
        made: &mut dyn Any,
        gpu: &WebGpu,
        pass: &mut RenderPass<'_>,
        data: &dyn Data,
        view: &View,
    ) {
        if let (Some(made), Some(data)) = (made.downcast_mut::<S>(), data.as_any().downcast_ref()) {
            (self.draw)(made, gpu, pass, data, view);
        }
    }
}

/// The data of a layer's ways of drawing, WebGL's and WebGPU's.
struct Both(Option<Rc<dyn Data>>, Option<Rc<dyn Data>>);

impl PartialEq for Both {
    fn eq(&self, other: &Self) -> bool {
        let same = |one: &Option<Rc<dyn Data>>, other: &Option<Rc<dyn Data>>| match (one, other) {
            (Some(one), Some(other)) => one.same(&**other),
            (None, None) => true,
            _ => false,
        };
        same(&self.0, &other.0) && same(&self.1, &other.1)
    }
}

/// A layer's data, of whatever type: to tell whether it changed since it was drawn.
pub(crate) trait Data: Any {
    fn same(&self, other: &dyn Data) -> bool;
    fn as_any(&self) -> &dyn Any;
}

impl<T: PartialEq + 'static> Data for T {
    fn same(&self, other: &dyn Data) -> bool {
        other.as_any().downcast_ref::<T>() == Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A layer with its data, as a frame draws it.
#[derive(Clone)]
pub(crate) struct Drawn {
    pub layer: Layer,
    pub data: Rc<dyn Data>,
}

impl PartialEq for Drawn {
    fn eq(&self, other: &Self) -> bool {
        self.layer == other.layer && self.data.same(&*other.data)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use glam::Mat4;
    use vertigo::{
        Value, dom_element, transaction,
        webgl::{Size, WebGl},
    };

    use super::{Data, Drawn, Layer, View};
    use crate::Camera;

    #[test]
    fn a_layer_drawn_again_only_when_its_data_changed() {
        let height = Value::new(1.0_f32);
        let layer = Layer::new(height.clone(), |_| (), |_, _, _: &f32, _| ());
        let drawn = || {
            transaction(|context| Drawn {
                layer: layer.clone(),
                data: layer.read(context),
            })
        };

        let before = drawn();
        assert!(before == drawn(), "the same data");
        height.set(2.0);
        assert!(before != drawn(), "data changed");

        let other = Layer::new(Value::new(1.0_f32), |_| (), |_, _, _: &f32, _| ());
        assert!(layer != other && layer == layer.clone());
    }

    #[test]
    fn a_layer_or_another_draws_with_the_way_of_each() {
        let gl = Layer::new(Value::new(1_u8), |_| (), |_, _, _: &u8, _| ());
        let gpu = Layer::webgpu(Value::new(2_u8), |_, _| (), |_, _, _, _: &u8, _| ());
        assert!(!gl.has_webgpu() && gpu.has_webgpu());

        let both = gl.clone().or(gpu.clone());
        assert!(both.has_webgpu());
        let canvas = dom_element! { <canvas /> };
        assert!(
            both.make(&WebGl::new(&canvas.get_ref())).is_some(),
            "and WebGL's"
        );
        assert!(gl.clone().or(gl.clone()).inner.gpu.is_none());
        assert!(both != gl, "a layer of its own");
    }

    #[test]
    fn data_of_another_type_isnt_the_same() {
        let (one, other): (Rc<dyn Data>, Rc<dyn Data>) = (Rc::new(1_u32), Rc::new(1_i32));
        assert!(one.same(&*Rc::new(1_u32)));
        assert!(!one.same(&*other));
    }

    #[test]
    fn a_layer_draws_its_data_with_what_it_made() {
        let drawn = Rc::new(RefCell::new(Vec::new()));
        let layer = Layer::new(Value::new("lines".to_string()), |_| 7_u8, {
            let drawn = drawn.clone();
            move |made: &mut u8, _, data: &String, view: &View| {
                *made += 1;
                drawn
                    .borrow_mut()
                    .push(format!("{data} {made} {}", view.size.width));
            }
        });

        let canvas = dom_element! { <canvas /> };
        let gl = WebGl::new(&canvas.get_ref());
        let mut made = layer.make(&gl).expect("a layer of WebGL");
        let data = transaction(|context| layer.read(context));
        let view = View {
            camera: Camera::default(),
            view_projection: Mat4::IDENTITY,
            size: Size {
                width: 640,
                height: 480,
            },
        };
        layer.draw(&mut *made, &gl, &*data, &view);
        layer.draw(&mut *made, &gl, &*data, &view);

        assert_eq!(*drawn.borrow(), vec!["lines 8 640", "lines 9 640"]);
    }
}
