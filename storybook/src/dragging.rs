//! Boxes on a floor walled round, dragged along it with the mouse, a finger or a pen.

use std::{cell::Cell, rc::Rc};

use vertigo::{Computed, DomNode, Value, dom, transaction};
use vertigo_3d::{
    Camera, Color, Drag, DragPhase, Material, Mesh, Object, Scene, SceneParams, Transform,
    math::Vec3,
};

use crate::background::css_frame;

/// Half of the floor's side: it goes from -5 to 5 along X and Z.
const FLOOR: f32 = 5.0;

/// Each box: its name, color, size, and where its middle starts on the floor.
const BOXES: [(&str, Color, f32, (f32, f32)); 5] = [
    ("red", Color::rgb(0.85, 0.25, 0.2), 1.0, (-2.0, -1.0)),
    ("green", Color::rgb(0.3, 0.75, 0.35), 0.8, (1.5, -2.0)),
    ("blue", Color::rgb(0.25, 0.45, 0.9), 1.2, (0.5, 1.5)),
    ("yellow", Color::rgb(0.95, 0.8, 0.2), 0.6, (3.0, 2.0)),
    ("purple", Color::rgb(0.6, 0.3, 0.75), 0.9, (-3.0, 2.5)),
];

/// How high a box is lifted while it's dragged.
const LIFT: f32 = 0.2;

pub fn render() -> DomNode {
    let moved = Value::new("Drag the boxes around the floor.".to_string());

    let cube = Mesh::cube();
    let boxes = BOXES.map(|(name, color, size, (x, z))| {
        let at = Value::new(Vec3::new(x, 0.0, z));
        let lifted = Value::new(false);
        let transform = Computed::from({
            let (at, lifted) = (at.clone(), lifted.clone());
            move |context| {
                let lift = if lifted.get(context) { LIFT } else { 0.0 };
                Transform::from_translation(at.get(context) + Vec3::Y * (size / 2.0 + lift))
                    .with_scale(Vec3::splat(size))
            }
        });

        // Where the box was when the drag started.
        let started_at = Rc::new(Cell::new(Vec3::ZERO));
        let reach = FLOOR - size / 2.0;
        let moved = moved.clone();

        Object::new(cube.clone())
            .transform(transform)
            .material(Material::lit(color))
            .on_drag(move |drag: Drag| {
                if drag.phase == DragPhase::Start {
                    started_at.set(transaction(|context| at.get(context)));
                }
                // Along the floor, the point grabbed under the pointer, the box within the walls.
                if let Some(by) = drag.moved_on(Vec3::Y) {
                    let to = started_at.get() + by;
                    at.set(Vec3::new(
                        to.x.clamp(-reach, reach),
                        0.0,
                        to.z.clamp(-reach, reach),
                    ));
                }
                lifted.set(drag.phase != DragPhase::End);
                if drag.phase == DragPhase::End {
                    let to = transaction(|context| at.get(context));
                    moved.set(format!("Moved: {name} to ({:.2}, {:.2})", to.x, to.z));
                }
            })
    });

    let floor = Object::new(Mesh::plane())
        .transform(Transform::from_scale(Vec3::new(
            2.0 * FLOOR,
            1.0,
            2.0 * FLOOR,
        )))
        .material(Material::lit(Color::rgb(0.75, 0.7, 0.6)));
    let wall = |translation: Vec3, scale: Vec3| {
        Object::new(cube.clone())
            .transform(Transform::from_translation(translation).with_scale(scale))
            .material(Material::lit(Color::rgb(0.45, 0.35, 0.3)))
    };
    let (long, thick, high) = (2.0 * FLOOR + 0.4, 0.2, 0.3);
    let walls = [
        wall(
            Vec3::new(0.0, high / 2.0, -FLOOR - 0.1),
            Vec3::new(long, high, thick),
        ),
        wall(
            Vec3::new(0.0, high / 2.0, FLOOR + 0.1),
            Vec3::new(long, high, thick),
        ),
        wall(
            Vec3::new(-FLOOR - 0.1, high / 2.0, 0.0),
            Vec3::new(thick, high, long),
        ),
        wall(
            Vec3::new(FLOOR + 0.1, high / 2.0, 0.0),
            Vec3::new(thick, high, long),
        ),
    ];

    let objects: Vec<Object> = std::iter::once(floor).chain(walls).chain(boxes).collect();
    let camera = Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 9.0, 9.0), Vec3::new(0.0, 0.0, 0.5));
    let params = SceneParams::default().background(Color::rgb(0.05, 0.05, 0.15));

    dom! {
        <div>
            <p>{moved}</p>
            <div css={css_frame()}>
                <Scene
                    camera={Value::new(camera).to_computed()}
                    objects={Value::new(objects).to_computed()}
                    {params}
                />
            </div>
        </div>
    }
}
