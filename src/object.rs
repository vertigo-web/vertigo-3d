use std::{fmt, rc::Rc};

use vertigo::Computed;

use crate::{Drag, Material, Mesh, Pick, Transform};

/// Something in a scene: a mesh, where it is and what its surface looks like. Each of them is a
/// [`Computed`], or anything becoming one (a constant, a [`Value`](vertigo::Value)), so the
/// object moves and changes with the values it's made of.
///
/// An object is shared when cloned: it's the same object, kept where the scene keeps it.
///
/// ```rust
/// use vertigo::Value;
/// use vertigo_3d::{Color, Material, Mesh, Object, Transform, math::Vec3};
///
/// let height = Value::new(0.0_f32);
///
/// let ball = Object::new(Mesh::sphere(24))
///     .transform(height.map(|height| Transform::from_translation(Vec3::new(0.0, height, 0.0))))
///     .material(Material::color(Color::rgb(0.9, 0.2, 0.2)));
///
/// // Up it goes, and the scene showing it draws it there.
/// height.set(2.0);
/// ```
#[derive(Clone)]
pub struct Object {
    inner: Rc<Inner>,
}

#[derive(Clone)]
struct Inner {
    mesh: Mesh,
    transform: Computed<Transform>,
    material: Computed<Material>,
    on_click: Option<Rc<dyn Fn(Pick)>>,
    on_hover: Option<Rc<dyn Fn(bool)>>,
    on_drag: Option<Rc<dyn Fn(Drag)>>,
}

impl Object {
    /// An object of `mesh`, where the mesh is made, white.
    pub fn new(mesh: Mesh) -> Self {
        Self {
            inner: Rc::new(Inner {
                mesh,
                transform: Transform::IDENTITY.into(),
                material: Material::default().into(),
                on_click: None,
                on_hover: None,
                on_drag: None,
            }),
        }
    }

    pub fn transform(self, transform: impl Into<Computed<Transform>>) -> Self {
        self.with(|inner| inner.transform = transform.into())
    }

    pub fn material(self, material: impl Into<Computed<Material>>) -> Self {
        self.with(|inner| inner.material = material.into())
    }

    /// Clicked, or tapped, where nothing nearer the camera is in the way, with the point clicked.
    /// The pointer over it shows it can be.
    ///
    /// Any object is in the way, also one that isn't clicked itself. A click is a press and a
    /// release of the main button, or a tap, a few pixels apart at most, so dragging over an
    /// object doesn't click it.
    ///
    /// ```rust
    /// use vertigo::Value;
    /// use vertigo_3d::{Color, Material, Mesh, Object};
    ///
    /// let on = Value::new(false);
    /// let lamp = Object::new(Mesh::sphere(24))
    ///     .material(on.map(|on| {
    ///         Material::color(if on { Color::rgb(1.0, 0.9, 0.5) } else { Color::rgb(0.3, 0.3, 0.3) })
    ///     }))
    ///     .on_click({
    ///         let on = on.clone();
    ///         move |_| on.change(|on| *on = !*on)
    ///     });
    /// ```
    pub fn on_click(self, on_click: impl Fn(Pick) + 'static) -> Self {
        self.with(|inner| inner.on_click = Some(Rc::new(on_click)))
    }

    /// The mouse, or a pen, came over it, with `true`, and went away, with `false`: as with
    /// [`on_click`](Self::on_click), where nothing nearer is in the way. It's told when the
    /// pointer moves, not when the object does.
    ///
    /// ```rust
    /// use vertigo::Value;
    /// use vertigo_3d::{Color, Material, Mesh, Object};
    ///
    /// let over = Value::new(false);
    /// let cube = Object::new(Mesh::cube())
    ///     .material(over.map(|over| {
    ///         Material::lit(if over { Color::rgb(1.0, 0.6, 0.2) } else { Color::WHITE })
    ///     }))
    ///     .on_hover({
    ///         let over = over.clone();
    ///         move |hovered| over.set(hovered)
    ///     });
    /// ```
    pub fn on_hover(self, on_hover: impl Fn(bool) + 'static) -> Self {
        self.with(|inner| inner.on_hover = Some(Rc::new(on_hover)))
    }

    /// Dragged with the mouse, a finger or a pen: pressed on, where nothing nearer the camera is
    /// in the way, and moved further than a click may. Told as the drag starts, at each move,
    /// and as it ends, see [`Drag`], also when the pointer leaves the canvas meanwhile. The
    /// pointer over it shows it can be grabbed, and a finger on the scene drags instead of
    /// scrolling the page.
    ///
    /// The object stays where it is: the handler moves it, f. ex. with
    /// [`Drag::moved_on`] from where it was when the drag started.
    ///
    /// ```rust
    /// use std::{cell::Cell, rc::Rc};
    ///
    /// use vertigo::{Value, transaction};
    /// use vertigo_3d::{Drag, DragPhase, Mesh, Object, Transform, math::Vec3};
    ///
    /// let at = Value::new(Vec3::new(0.0, 0.5, 0.0));
    /// let started_at = Rc::new(Cell::new(Vec3::ZERO));
    ///
    /// let crate_ = Object::new(Mesh::cube())
    ///     .transform(at.map(Transform::from_translation))
    ///     .on_drag({
    ///         let at = at.clone();
    ///         move |drag: Drag| {
    ///             if drag.phase == DragPhase::Start {
    ///                 started_at.set(transaction(|context| at.get(context)));
    ///             }
    ///             // Along the floor, the point grabbed under the pointer
    ///             if let Some(moved) = drag.moved_on(Vec3::Y) {
    ///                 at.set(started_at.get() + moved);
    ///             }
    ///         }
    ///     });
    /// ```
    pub fn on_drag(self, on_drag: impl Fn(Drag) + 'static) -> Self {
        self.with(|inner| inner.on_drag = Some(Rc::new(on_drag)))
    }

    fn with(self, change: impl FnOnce(&mut Inner)) -> Self {
        let mut inner = Rc::unwrap_or_clone(self.inner);
        change(&mut inner);
        Self {
            inner: Rc::new(inner),
        }
    }

    pub(crate) fn mesh(&self) -> &Mesh {
        &self.inner.mesh
    }

    pub(crate) fn transform_now(&self) -> &Computed<Transform> {
        &self.inner.transform
    }

    pub(crate) fn material_now(&self) -> &Computed<Material> {
        &self.inner.material
    }

    /// Whether it's told of clicks, of the pointer over it or of dragging it.
    pub(crate) fn is_interactive(&self) -> bool {
        self.inner.on_click.is_some() || self.inner.on_hover.is_some() || self.is_draggable()
    }

    pub(crate) fn is_draggable(&self) -> bool {
        self.inner.on_drag.is_some()
    }

    pub(crate) fn is_clickable(&self) -> bool {
        self.inner.on_click.is_some()
    }

    pub(crate) fn click(&self, pick: Pick) {
        if let Some(on_click) = &self.inner.on_click {
            on_click(pick);
        }
    }

    pub(crate) fn hover(&self, hovered: bool) {
        if let Some(on_hover) = &self.inner.on_hover {
            on_hover(hovered);
        }
    }

    pub(crate) fn drag(&self, drag: Drag) {
        if let Some(on_drag) = &self.inner.on_drag {
            on_drag(drag);
        }
    }
}

impl PartialEq for Object {
    /// The same object, not one alike.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Object")
            .field("mesh", &self.inner.mesh)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Mesh, Object};

    #[test]
    fn an_object_is_itself_and_its_clones() {
        let cube = Mesh::cube();
        let object = Object::new(cube.clone());
        assert_eq!(object, object.clone());
        assert_ne!(object, Object::new(cube));
    }
}
