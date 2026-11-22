use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use vertigo::{
    Computed, Css, DomElement, DomNode, DropResource, PointerEvent, PointerType, Value, component,
    css, dom_element, get_driver,
    render::render_value,
    webgl::{ImageLoad, Size, WebGl, WebGlOptions},
    webgpu::{AlphaMode, WebGpu, WebGpuOptions},
};

use crate::{
    Camera, Color, Drag, DragPhase, Layer, Light, Loading, Object, Pick, Ray, Texture,
    layer::Drawn,
    pick::{nearest, on_canvas},
    render::{Frame, GlRenderer, GpuRenderer, Instance},
};

/// What a [`Scene`] can do without, each with its default, set by the methods of the same
/// names: each takes anything becoming a [`Computed`], a constant or a [`Value`](vertigo::Value)
/// as well, but [`SceneParams::on_load`], which takes a callback.
///
/// ```rust
/// use vertigo::Value;
/// use vertigo_3d::{Color, SceneParams};
///
/// let night = Value::new(Color::rgb(0.05, 0.05, 0.15));
/// let params = SceneParams::default().background(night);
/// ```
#[derive(Clone)]
pub struct SceneParams {
    /// Black at first.
    pub background: Computed<Color>,
    /// The light falling on the objects of [`Material::lit`](crate::Material::lit):
    /// [`Light::defaults`] at first. None leaves them dark.
    pub lights: Computed<Vec<Light>>,
    /// Drawing of their own, with [`vertigo::webgl`], see [`Layer`]: none at first.
    pub layers: Computed<Vec<Layer>>,
    /// The time [`Shader`](crate::Shader)s read, in seconds: 0 at first. With [`frame_time`],
    /// the scene is drawn before each frame, for the shaders to move.
    ///
    /// [`frame_time`]: crate::frame_time
    pub time: Computed<f32>,
    /// How many of the canvas's pixels on the screen its picture has, across and down: 1 at
    /// first, 0.5 half of them, a quarter in all, stretched over the canvas - f. ex. for a slow
    /// GPU, a choice of its user. A change makes the canvas anew.
    pub bitmap_scale: Computed<f32>,
    /// Whether the edges of objects are smoothed (antialiasing): yes at first. A change makes the
    /// canvas anew.
    pub antialias: Computed<bool>,
    /// Whether normal and height maps show (see
    /// [`Material::normal_map`](crate::Material::normal_map)): yes at first. Without them
    /// surfaces are flat, and drawn faster.
    pub relief: Computed<bool>,
    /// Textures loaded before any object shows them, and kept while listed: none at first. See
    /// [`SceneParams::preload`].
    pub preload: Computed<Vec<Texture>>,
    /// Told how far the images of the textures are loaded: nothing at first. See
    /// [`SceneParams::on_load`].
    pub on_load: Option<Rc<dyn Fn(Loading)>>,
    /// What draws the scene: [`Backend::Auto`] at first. A change makes the canvas anew.
    pub backend: Computed<Backend>,
}

/// What draws a scene, see [`SceneParams::backend`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Backend {
    /// WebGPU, where the browser has it and the scene's shaders and layers have a way of it (see
    /// [`Shader`](crate::Shader) and [`Layer::webgpu`]), and WebGL 2 elsewhere. Once a scene finds
    /// the browser has no WebGPU, the scenes draw with WebGL at once.
    #[default]
    Auto,
    WebGl,
    /// WebGPU, where the browser has it, else nothing: f. ex. to test it.
    WebGpu,
}

impl SceneParams {
    pub fn background(self, background: impl Into<Computed<Color>>) -> Self {
        Self {
            background: background.into(),
            ..self
        }
    }

    pub fn lights(self, lights: impl Into<Computed<Vec<Light>>>) -> Self {
        Self {
            lights: lights.into(),
            ..self
        }
    }

    pub fn layers(self, layers: impl Into<Computed<Vec<Layer>>>) -> Self {
        Self {
            layers: layers.into(),
            ..self
        }
    }

    pub fn time(self, time: impl Into<Computed<f32>>) -> Self {
        Self {
            time: time.into(),
            ..self
        }
    }

    pub fn bitmap_scale(self, bitmap_scale: impl Into<Computed<f32>>) -> Self {
        Self {
            bitmap_scale: bitmap_scale.into(),
            ..self
        }
    }

    pub fn antialias(self, antialias: impl Into<Computed<bool>>) -> Self {
        Self {
            antialias: antialias.into(),
            ..self
        }
    }

    pub fn relief(self, relief: impl Into<Computed<bool>>) -> Self {
        Self {
            relief: relief.into(),
            ..self
        }
    }

    /// `textures` loaded on the scene's canvas before any object shows them, so objects show them
    /// from the first time they are drawn. A texture from a URL is otherwise loaded when an
    /// object first shows it, which has its base color until the image comes. They stay loaded
    /// while listed, each taking memory on the GPU - an image of 512 by 512 pixels, with the
    /// smaller ones made of it, about 1.4 MB.
    ///
    /// ```rust
    /// use vertigo::Value;
    /// use vertigo_3d::{SceneParams, Texture};
    ///
    /// // The textures of the next view, before it's shown
    /// let next = Value::new(vec![Texture::url("/images/next/floor.jpg")]);
    /// let params = SceneParams::default().preload(next);
    /// ```
    pub fn preload(self, textures: impl Into<Computed<Vec<Texture>>>) -> Self {
        Self {
            preload: textures.into(),
            ..self
        }
    }

    /// `callback` told how far the images of the scene's textures from URLs are loaded - of its
    /// objects and preloaded, see [`Loading`]: after a drawing changes which textures it has,
    /// and when an image comes or fails. Only when that changes, and not during SSR, which draws
    /// nothing. A canvas made anew, f. ex. for another [`bitmap_scale`](Self::bitmap_scale),
    /// loads them again, from the browser's cache.
    ///
    /// ```rust
    /// use vertigo::Value;
    /// use vertigo_3d::{SceneParams, Texture};
    ///
    /// // An app shows itself once all its textures are in.
    /// let textures = vec![
    ///     Texture::url("/images/floor.jpg"),
    ///     Texture::url("/images/sky.jpg"),
    /// ];
    /// let ready = Value::new(false);
    /// let params = SceneParams::default().preload(textures).on_load({
    ///     let ready = ready.clone();
    ///     move |loading| ready.set(loading.is_done())
    /// });
    /// ```
    pub fn on_load(self, callback: impl Fn(Loading) + 'static) -> Self {
        Self {
            on_load: Some(Rc::new(callback)),
            ..self
        }
    }

    /// Whether WebGPU or WebGL draws the scene, see [`Backend`]: both draw it alike, WebGPU with
    /// less of the processor's time where there are many objects.
    ///
    /// ```rust
    /// use vertigo_3d::{Backend, SceneParams};
    ///
    /// // WebGL, which a shader of GLSL alone needs anyway
    /// let params = SceneParams::default().backend(Backend::WebGl);
    /// ```
    pub fn backend(self, backend: impl Into<Computed<Backend>>) -> Self {
        Self {
            backend: backend.into(),
            ..self
        }
    }
}

impl Default for SceneParams {
    fn default() -> Self {
        Self {
            background: Color::BLACK.into(),
            lights: Light::defaults().into(),
            layers: Vec::new().into(),
            time: 0.0.into(),
            bitmap_scale: 1.0.into(),
            antialias: true.into(),
            relief: true.into(),
            preload: Vec::new().into(),
            on_load: None,
            backend: Backend::default().into(),
        }
    }
}

/// A 3D scene of `objects` seen by `camera`, on a canvas as big as the element it is in.
///
/// It is drawn when something in it changes - the camera, the list, or an object's transform or
/// material - and when the canvas changes its size or an image of a texture comes, at most once
/// a transaction, not before each frame. Objects of the same [`Mesh`](crate::Mesh) and texture are
/// drawn together, with one call, and see-through ones after the others, the farthest first.
/// Nothing is drawn during SSR: the canvas comes empty and the browser draws it. It is drawn with
/// WebGPU or WebGL, see [`SceneParams::backend`].
///
/// Its objects are told of clicks on them, see [`Object::on_click`], of the pointer over them,
/// see [`Object::on_hover`], and of dragging them, see [`Object::on_drag`].
///
/// ```rust
/// use vertigo::{Value, dom};
/// use vertigo_3d::{Camera, Color, Material, Mesh, Object, Scene, SceneParams, math::Vec3};
///
/// let camera = Value::new(
///     Camera::perspective(45_f32.to_radians()).looking_at(Vec3::new(0.0, 2.0, 5.0), Vec3::ZERO),
/// );
/// let ball = Object::new(Mesh::sphere(24)).material(Material::color(Color::rgb(0.8, 0.2, 0.2)));
///
/// dom! {
///     <div style="width: 640px; height: 400px">
///         <Scene
///             camera={camera.to_computed()}
///             objects={Value::new(vec![ball]).to_computed()}
///             params={SceneParams::default()}
///         />
///     </div>
/// };
/// ```
#[component]
pub fn Scene(camera: Computed<Camera>, objects: Computed<Vec<Object>>, params: SceneParams) {
    let draggable = objects.map(|objects| objects.iter().any(Object::is_draggable));
    let on_load = params.on_load.clone();
    let (bitmap_scale, antialias, backend) = (
        params.bitmap_scale.clone(),
        params.antialias.clone(),
        params.backend.clone(),
    );
    let frame = Computed::from(move |context| Frame {
        camera: camera.get(context),
        background: params.background.get(context),
        lights: params.lights.get(context),
        time: params.time.get(context),
        relief: params.relief.get(context),
        preload: params.preload.get(context),
        instances: objects
            .get(context)
            .iter()
            .map(|object| Instance {
                object: object.clone(),
                transform: object.transform_now().get(context),
                material: object.material_now().get(context),
            })
            .collect(),
        layers: params
            .layers
            .get(context)
            .into_iter()
            .map(|layer| Drawn {
                data: layer.read(context),
                layer,
            })
            .collect(),
    });

    let needs_webgl = frame.map(|frame| frame.needs_webgl());
    let options = Computed::from(move |context| Options {
        bitmap_scale: bitmap_scale.get(context),
        antialias: antialias.get(context),
        webgpu: match backend.get(context) {
            Backend::WebGl => false,
            Backend::WebGpu => true,
            Backend::Auto => !needs_webgl.get(context) && !webgpu_missing().get(context),
        },
    });

    render_value(options, move |options| {
        canvas(options, &frame, &draggable, on_load.clone())
    })
}

thread_local! {
    /// Whether a scene found the browser has no WebGPU, so the others draw with WebGL at once.
    static WEBGPU_MISSING: Value<bool> = Value::new(false);
}

fn webgpu_missing() -> Value<bool> {
    WEBGPU_MISSING.with(Value::clone)
}

/// What the canvas is made with: a change makes it anew.
#[derive(Clone, Copy, PartialEq)]
struct Options {
    bitmap_scale: f32,
    antialias: bool,
    /// Drawn with WebGPU, not WebGL.
    webgpu: bool,
}

/// A scene's context, of WebGL or WebGPU.
#[derive(Clone)]
enum Context {
    Gl(WebGl),
    Gpu(WebGpu),
}

impl Context {
    fn size(&self) -> Size {
        match self {
            Context::Gl(gl) => gl.size(),
            Context::Gpu(gpu) => gpu.size(),
        }
    }

    fn on_resize(&self, callback: impl Fn(Size) + 'static) -> DropResource {
        match self {
            Context::Gl(gl) => gl.on_resize(callback),
            Context::Gpu(gpu) => gpu.on_resize(callback),
        }
    }

    fn on_image_load(&self, callback: impl Fn(&ImageLoad) + 'static) -> DropResource {
        match self {
            Context::Gl(gl) => gl.on_image_load(callback),
            Context::Gpu(gpu) => gpu.on_image_load(callback),
        }
    }

    fn on_restore(&self, callback: impl Fn() + 'static) -> DropResource {
        match self {
            Context::Gl(gl) => gl.on_restore(callback),
            Context::Gpu(gpu) => gpu.on_restore(callback),
        }
    }
}

/// What a scene made on its context, to draw with.
enum Renderer {
    Gl(GlRenderer),
    Gpu(GpuRenderer),
}

impl Renderer {
    /// Made on `context`, anew when it lost what it had, of `options`.
    fn new(context: &Context, options: Options) -> Self {
        match context {
            Context::Gl(gl) => Renderer::Gl(GlRenderer::new(gl)),
            Context::Gpu(gpu) => {
                Renderer::Gpu(GpuRenderer::new(gpu, if options.antialias { 4 } else { 1 }))
            }
        }
    }

    fn draw(&mut self, frame: &Frame) {
        match self {
            Renderer::Gl(renderer) => renderer.draw(frame),
            Renderer::Gpu(renderer) => renderer.draw(frame),
        }
    }

    fn image(&mut self, load: &ImageLoad) {
        match self {
            Renderer::Gl(renderer) => renderer.image(load),
            Renderer::Gpu(renderer) => renderer.image(load),
        }
    }

    fn loading(&self) -> Loading {
        match self {
            Renderer::Gl(renderer) => renderer.loading(),
            Renderer::Gpu(renderer) => renderer.loading(),
        }
    }
}

/// The context of `canvas`, of `options`.
fn context(canvas: &DomElement, options: Options) -> Context {
    if options.webgpu {
        let gpu = WebGpu::with_options(
            &canvas.get_ref(),
            WebGpuOptions {
                // See-through where the background is, as a canvas of WebGL
                alpha_mode: AlphaMode::Premultiplied,
                bitmap_scale: options.bitmap_scale,
                ..WebGpuOptions::default()
            },
        );
        // The scenes draw with WebGL from now on.
        canvas.append_drop_resource(gpu.on_unavailable(|| webgpu_missing().set(true)));
        Context::Gpu(gpu)
    } else {
        // What was drawn stays until it's drawn again, as a scene is drawn only when it changes:
        // so the picture can be read back, f. ex. by a test or a screenshot.
        Context::Gl(WebGl::with_options(
            &canvas.get_ref(),
            WebGlOptions {
                preserve_drawing_buffer: true,
                antialias: options.antialias,
                bitmap_scale: options.bitmap_scale,
                ..WebGlOptions::default()
            },
        ))
    }
}

/// The canvas of a scene, drawing `frame`, telling its objects of the pointer and `on_load` of
/// its textures.
fn canvas(
    options: Options,
    frame: &Computed<Frame>,
    draggable: &Computed<bool>,
    on_load: Option<Rc<dyn Fn(Loading)>>,
) -> DomNode {
    let cursor = Value::new(Cursor::Default);
    let css = Computed::from({
        let (cursor, draggable) = (cursor.clone(), draggable.clone());
        move |context| css_canvas(cursor.get(context), draggable.get(context))
    });
    let canvas = dom_element! { <canvas css={css} /> };
    let gl = context(&canvas, options);

    let renderer = Rc::new(RefCell::new(Renderer::new(&gl, options)));
    // The frame drawn last, to draw again when the canvas changes its size, and to tell what the
    // pointer is over.
    let last: Rc<RefCell<Option<Frame>>> = Rc::default();

    // What `on_load` was told last, to tell it only a change. SSR draws nothing to tell of.
    let telling = on_load.is_some() && get_driver().is_browser();
    let told: Rc<Cell<Option<Loading>>> = Rc::default();
    let tell = {
        let (renderer, told) = (renderer.clone(), told.clone());
        move || {
            let now = renderer.borrow().loading();
            if let Some(on_load) = &on_load
                && telling
                && told.replace(Some(now)) != Some(now)
            {
                on_load(now);
            }
        }
    };

    canvas.append_drop_resource(frame.clone().subscribe({
        let (renderer, last, tell) = (renderer.clone(), last.clone(), tell.clone());
        move |frame| {
            renderer.borrow_mut().draw(&frame);
            *last.borrow_mut() = Some(frame);
            // A drawing may have other textures. It's done as the graph changes, which the
            // callback may not change in turn: it's told after.
            if telling && told.get() != Some(renderer.borrow().loading()) {
                let tell = tell.clone();
                get_driver().spawn(async move { tell() });
            }
        }
    }));

    let again = {
        let (renderer, last) = (renderer.clone(), last.clone());
        move || {
            if let Some(frame) = &*last.borrow() {
                renderer.borrow_mut().draw(frame);
            }
        }
    };
    canvas.append_drop_resource(gl.on_resize({
        let again = again.clone();
        move |_| again()
    }));
    // An image of a texture came, or failed.
    canvas.append_drop_resource(gl.on_image_load({
        let (again, renderer, tell) = (again.clone(), renderer.clone(), tell.clone());
        move |load| {
            renderer.borrow_mut().image(load);
            again();
            tell();
        }
    }));
    canvas.append_drop_resource(gl.on_restore({
        let (gl, renderer) = (gl.clone(), renderer.clone());
        move || {
            // Everything made on the context before is gone with it, the textures loading again.
            *renderer.borrow_mut() = Renderer::new(&gl, options);
            again();
            tell();
        }
    }));

    let pointer = Rc::new(Pointer {
        gl,
        last,
        cursor,
        hovered: RefCell::default(),
        pressed: RefCell::default(),
    });
    let canvas = canvas
        .on_pointer_down({
            let pointer = pointer.clone();
            move |event: PointerEvent| pointer.down(&event)
        })
        .on_pointer_up({
            let pointer = pointer.clone();
            move |event: PointerEvent| pointer.up(&event)
        })
        .on_pointer_move({
            let pointer = pointer.clone();
            move |event: PointerEvent| pointer.moved(&event)
        })
        .on_pointer_cancel({
            let pointer = pointer.clone();
            move |event: PointerEvent| pointer.cancel(&event)
        })
        .on_mouse_leave(move || pointer.left());

    DomNode::from(canvas)
}

/// What the cursor shows over the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cursor {
    Default,
    /// Over an object to click.
    Pointer,
    /// Over an object to drag.
    Grab,
    /// Dragging one.
    Grabbing,
}

fn css_canvas(cursor: Cursor, draggable: bool) -> Css {
    let css = css! {"
        display: block;
        width: 100%;
        height: 100%;
    "};
    let css = match cursor {
        Cursor::Default => css,
        Cursor::Pointer => css.push_str("cursor: pointer"),
        Cursor::Grab => css.push_str("cursor: grab"),
        Cursor::Grabbing => css.push_str("cursor: grabbing"),
    };
    if draggable {
        // A finger on the scene drags its objects, not the page.
        css.push_str("touch-action: none")
    } else {
        css
    }
}

/// Pixels a pointer may move between pressing and letting go for a click, and before a drag.
const CLICK_SLOP: f64 = 5.0;

/// What a scene's objects are told of the pointer by.
struct Pointer {
    gl: Context,
    last: Rc<RefCell<Option<Frame>>>,
    cursor: Value<Cursor>,
    /// The object the pointer is over, if it's told so.
    hovered: RefCell<Option<Object>>,
    /// The main button pressed, or a finger touching.
    pressed: RefCell<Option<Press>>,
}

/// Where the pointer pressed: to click if let go there, or to drag what it pressed on.
struct Press {
    id: i32,
    x: f64,
    y: f64,
    grab: Option<Grab>,
}

/// An object pressed on, to drag.
struct Grab {
    object: Object,
    grabbed: Pick,
    /// Since the pointer moved further than a click may.
    dragging: bool,
    /// Where the pointer looked last, for an end the pointer doesn't tell where.
    ray: Ray,
}

impl Pointer {
    /// From the camera through the pointer at `event`.
    fn ray(&self, event: &PointerEvent) -> Option<Ray> {
        let last = self.last.borrow();
        let frame = last.as_ref()?;
        let at = on_canvas(
            event.pointer_x,
            event.pointer_y,
            event.element_width,
            event.element_height,
        )?;
        Ray::through(&frame.camera, self.gl.size().aspect(), at)
    }

    /// The interactive object the pointer at `event` is over, nothing nearer in the way, with
    /// where on it, and the ray it's found by.
    fn pick(&self, event: &PointerEvent) -> Option<(Object, Pick, Ray)> {
        let interactive = self.last.borrow().as_ref().is_some_and(|frame| {
            frame
                .instances
                .iter()
                .any(|instance| instance.object.is_interactive())
        });
        if !interactive {
            return None;
        }

        let ray = self.ray(event)?;
        let last = self.last.borrow();
        let frame = last.as_ref()?;
        let (index, pick) = nearest(
            &ray,
            frame
                .instances
                .iter()
                .map(|instance| (instance.object.mesh(), &instance.transform)),
        )?;
        let object = &frame.instances.get(index)?.object;
        object.is_interactive().then(|| (object.clone(), pick, ray))
    }

    fn down(&self, event: &PointerEvent) {
        if !event.is_primary || event.button != 0 {
            return;
        }
        let grab = self
            .pick(event)
            .filter(|(object, ..)| object.is_draggable())
            .map(|(object, grabbed, ray)| Grab {
                object,
                grabbed,
                dragging: false,
                ray,
            });
        if grab.is_some() {
            // The drag goes on outside the canvas, and ends wherever the pointer is let go.
            event.capture_pointer();
        }
        *self.pressed.borrow_mut() = Some(Press {
            id: event.pointer_id,
            x: event.pointer_x,
            y: event.pointer_y,
            grab,
        });
    }

    fn up(&self, event: &PointerEvent) {
        let Some(press) = self.release(event) else {
            return;
        };
        if let Some(grab) = press.grab.filter(|grab| grab.dragging) {
            self.drag(grab, DragPhase::End, Some(event));
            self.hover_at(event);
            return;
        }
        let moved = (event.pointer_x - press.x).hypot(event.pointer_y - press.y);
        if moved > CLICK_SLOP {
            return;
        }
        if let Some((object, pick, _)) = self.pick(event) {
            object.click(pick);
        }
    }

    fn moved(&self, event: &PointerEvent) {
        // The object pressed on, dragged from when the pointer moves further than a click may.
        let dragged = {
            let mut pressed = self.pressed.borrow_mut();
            pressed
                .as_mut()
                .filter(|press| press.id == event.pointer_id)
                .and_then(|press| {
                    let moved = (event.pointer_x - press.x).hypot(event.pointer_y - press.y);
                    let grab = press.grab.as_mut()?;
                    let phase = match grab.dragging {
                        true => DragPhase::Move,
                        false if moved > CLICK_SLOP => DragPhase::Start,
                        false => return None,
                    };
                    grab.dragging = true;
                    if let Some(ray) = self.ray(event) {
                        grab.ray = ray;
                    }
                    Some((grab.object.clone(), grab.grabbed, grab.ray, phase))
                })
        };
        match dragged {
            Some((object, grabbed, ray, phase)) => {
                if phase == DragPhase::Start {
                    self.show_cursor();
                }
                object.drag(Drag {
                    phase,
                    grabbed,
                    ray,
                });
            }
            None => self.hover_at(event),
        }
    }

    /// The browser took the pointer over, f. ex. to scroll the page.
    fn cancel(&self, event: &PointerEvent) {
        let press = self.release(event);
        self.stop(press);
    }

    /// The mouse left the canvas. While it drags, the canvas holds it and it doesn't.
    fn left(&self) {
        let press = self.pressed.borrow_mut().take();
        self.stop(press);
    }

    /// Ends the drag of `press`, if it dragged, where the pointer looked last, and hovers
    /// nothing.
    fn stop(&self, press: Option<Press>) {
        if let Some(grab) = press.and_then(|press| press.grab)
            && grab.dragging
        {
            self.drag(grab, DragPhase::End, None);
        }
        self.hover(None);
    }

    /// The press of the pointer of `event`, let go.
    fn release(&self, event: &PointerEvent) -> Option<Press> {
        let mut pressed = self.pressed.borrow_mut();
        match &*pressed {
            Some(press) if press.id == event.pointer_id => pressed.take(),
            _ => None,
        }
    }

    /// Tells the object of `grab` of a drag, the pointer of `event` looking where it does, or
    /// where it looked last.
    fn drag(&self, grab: Grab, phase: DragPhase, event: Option<&PointerEvent>) {
        let ray = event.and_then(|event| self.ray(event)).unwrap_or(grab.ray);
        self.show_cursor();
        grab.object.drag(Drag {
            phase,
            grabbed: grab.grabbed,
            ray,
        });
    }

    /// Hovers what the pointer of `event` is over: a finger isn't over anything before it
    /// touches, and after it's lifted.
    fn hover_at(&self, event: &PointerEvent) {
        if event.pointer_type != PointerType::Touch {
            self.hover(self.pick(event).map(|(object, ..)| object));
        }
    }

    /// Tells the object hovered before it isn't, and the one now that it is.
    fn hover(&self, now: Option<Object>) {
        let before = self.hovered.replace(now.clone());
        if before != now {
            if let Some(before) = before {
                before.hover(false);
            }
            if let Some(now) = now {
                now.hover(true);
            }
        }
        self.show_cursor();
    }

    fn show_cursor(&self) {
        let dragging = self
            .pressed
            .borrow()
            .as_ref()
            .and_then(|press| press.grab.as_ref())
            .is_some_and(|grab| grab.dragging);
        let cursor = match &*self.hovered.borrow() {
            _ if dragging => Cursor::Grabbing,
            Some(object) if object.is_draggable() => Cursor::Grab,
            Some(object) if object.is_clickable() => Cursor::Pointer,
            _ => Cursor::Default,
        };
        self.cursor.set(cursor);
    }
}
