//! Boxes and balls falling onto a floor walled round, moved by rapier3d, a physics engine of its
//! own (see `world`). Each frame it steps its world, and the scene's objects take the places of
//! its bodies. A click on a body throws it up from the point clicked, so it spins as it flies,
//! unless the click is right above its middle.

use std::{cell::RefCell, rc::Rc};

use vertigo::{DomNode, Value, bind, dom};
use vertigo_3d::{Camera, Color, Material, Object, Pick, Scene, SceneParams, math::Vec3};

use crate::{
    background::css_frame,
    world::{Body, Meshes, Shape, Stuff, World, ground, shown_with_frames},
};

/// How fast a click throws a body up, in units a second: a body thrown from right below its
/// middle flies about 2.5 up.
const THROW: f32 = 7.0;

/// As heavy as water, bouncing a little.
const BOX: Stuff = Stuff {
    density: 1.0,
    bounce: 0.2,
    drag: 0.1,
    gravity: 1.0,
};

/// As heavy as water, bouncing more.
const BALL: Stuff = Stuff { bounce: 0.5, ..BOX };

/// Each body, with its color.
const BODIES: [(Body, Color); 7] = [
    // Alone, falling flat where it lands.
    (
        Body {
            name: "red",
            shape: Shape::Box,
            size: 1.0,
            at: [-3.0, 3.0, 1.5],
            turn: [0.0; 3],
            stuff: BOX,
        },
        Color::rgb(0.85, 0.25, 0.2),
    ),
    // A pile, tumbling.
    (
        Body {
            name: "green",
            shape: Shape::Box,
            size: 1.2,
            at: [1.5, 2.0, -0.5],
            turn: [0.0, 0.3, 0.0],
            stuff: BOX,
        },
        Color::rgb(0.3, 0.75, 0.35),
    ),
    (
        Body {
            name: "blue",
            shape: Shape::Box,
            size: 0.8,
            at: [1.7, 4.0, -0.3],
            turn: [0.4, 0.2, 0.1],
            stuff: BOX,
        },
        Color::rgb(0.25, 0.45, 0.9),
    ),
    (
        Body {
            name: "yellow",
            shape: Shape::Ball,
            size: 0.9,
            at: [1.2, 6.0, -0.7],
            turn: [0.0; 3],
            stuff: BALL,
        },
        Color::rgb(0.95, 0.8, 0.2),
    ),
    (
        Body {
            name: "purple",
            shape: Shape::Box,
            size: 0.6,
            at: [1.6, 7.5, -0.4],
            turn: [0.5, 0.0, 0.7],
            stuff: BOX,
        },
        Color::rgb(0.6, 0.3, 0.75),
    ),
    (
        Body {
            name: "orange",
            shape: Shape::Ball,
            size: 0.7,
            at: [2.0, 9.0, -0.5],
            turn: [0.0; 3],
            stuff: BALL,
        },
        Color::rgb(1.0, 0.55, 0.15),
    ),
    // Bouncing where it falls.
    (
        Body {
            name: "white",
            shape: Shape::Ball,
            size: 1.0,
            at: [-1.0, 5.0, -2.5],
            turn: [0.0; 3],
            stuff: BALL,
        },
        Color::rgb(0.9, 0.9, 0.9),
    ),
];

fn bodies() -> impl Iterator<Item = &'static Body> {
    BODIES.iter().map(|(body, _)| body)
}

pub fn render() -> DomNode {
    let meshes = Meshes::default();
    let world = Rc::new(RefCell::new(World::new(&meshes, bodies())));
    let shown = shown_with_frames(&world);

    let thrown = Value::new("Click a body to throw it up.".to_string());

    let bodies = BODIES.iter().enumerate().map(|(index, (body, color))| {
        let (world, thrown, name) = (world.clone(), thrown.clone(), body.name);
        Object::new(meshes.of(body.shape).clone())
            .transform(shown.map(move |shown| shown.place(index)))
            .material(Material::lit(*color).specular(Color::WHITE, 32.0))
            .on_click(move |pick: Pick| {
                world.borrow_mut().throw(index, pick.point, THROW);
                thrown.set(format!("Thrown: {name}"));
            })
    });
    let floor = Material::lit(Color::rgb(0.75, 0.7, 0.6));
    let objects: Vec<Object> = ground(&meshes, floor).chain(bodies).collect();

    let status = shown.map(|shown| shown.status());
    let again = bind!(world, meshes, thrown, |_| {
        *world.borrow_mut() = World::new(&meshes, self::bodies());
        thrown.set("Dropped again.".to_string());
    });

    let camera = Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 8.0, 11.0), Vec3::new(0.0, 0.5, 0.0));
    let params = SceneParams::default().background(Color::rgb(0.05, 0.05, 0.15));

    dom! {
        <div>
            <p>
                <button on_click={again}>"Drop again"</button>
                " " {status} " " {thrown}
            </p>
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

#[cfg(test)]
mod tests {
    use vertigo_3d::math::Vec3;

    use super::{BODIES, THROW, bodies};
    use crate::world::{
        FLOOR, Meshes, World,
        tests::{rest, run},
    };

    fn world() -> World {
        World::new(&Meshes::default(), bodies())
    }

    #[test]
    fn the_bodies_fall_and_come_to_rest_on_the_floor() {
        let mut world = world();
        assert_eq!(world.shown().moving, BODIES.len(), "falling at first");
        assert!(rest(&mut world, 0, 6).is_some(), "at rest in 6 seconds");

        let places = world.shown().places;
        // The red box, alone, flat on the floor where it fell.
        let red = places[0].translation;
        assert!(red.abs_diff_eq(Vec3::new(-3.0, 0.5, 1.5), 0.1), "{red}");
        for (place, (body, _)) in places.iter().zip(&BODIES) {
            let at = place.translation;
            assert!(
                at.x.abs() < FLOOR && at.z.abs() < FLOOR && at.y > 0.0 && at.y < 3.0,
                "{} on the floor, within the walls: {at}",
                body.name
            );
        }
    }

    #[test]
    fn a_body_thrown_from_below_its_middle_goes_up_and_lands_where_it_was() {
        let mut world = world();
        let Some(rested) = rest(&mut world, 0, 6) else {
            panic!("at rest in 6 seconds");
        };
        let before = world.shown().places[0].translation;

        world.throw(0, before + Vec3::Y * 0.5, THROW);
        assert_eq!(world.shown().moving, 1, "the red box alone");
        let up = run(&mut world, rested, 0.5);
        let flying = world.shown().places[0].translation;
        assert!(flying.y > before.y + 1.5, "half a second up: {flying}");

        assert!(rest(&mut world, up, 6).is_some(), "at rest again");
        let after = world.shown().places[0].translation;
        assert!(after.abs_diff_eq(before, 0.1), "{before} then {after}");
    }
}
