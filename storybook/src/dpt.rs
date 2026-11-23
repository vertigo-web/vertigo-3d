//! DPT: Dragging + Physics + Textures, the three stories in one. Wooden crates, a heavy steel
//! ball, a wooden ball and a balloon light as air fall onto a floor of tiles, moved by rapier3d
//! (see `world`), and are dragged around with the mouse, a finger or a pen.
//!
//! A body held is weightless, and drawn to the pointer, a little above where it was grabbed. All
//! follow the pointer alike, but push what's in the way only as hard as they're heavy: the steel
//! ball sweeps crates aside, the balloon can't move them. Let go while moving, a body flies on.

use std::{cell::Cell, f32::consts::TAU, rc::Rc};

use vertigo::{DomNode, Value, bind, dom, include_static};
use vertigo_3d::{
    Camera, Color, Drag, DragPhase, Material, Object, Scene, SceneParams, Texture, math::Vec3,
};

use crate::{
    background::css_frame,
    textures::pixels,
    world::{Body, FLOOR, Meshes, Shape, Stuff, World, ground, shown_with_frames},
};

/// How high above where it was grabbed a body is held.
const LIFT: f32 = 0.3;

/// Dry wood, floating on water.
const WOOD: Stuff = Stuff {
    density: 0.7,
    bounce: 0.3,
    drag: 0.1,
    gravity: 1.0,
};

/// Ten times as heavy as wood, and more.
const STEEL: Stuff = Stuff {
    density: 7.8,
    bounce: 0.3,
    drag: 0.02,
    gravity: 1.0,
};

/// Rubber full of air, held up by the air round it and slowed down by it, bouncing.
const BALLOON: Stuff = Stuff {
    density: 0.05,
    bounce: 0.8,
    drag: 2.0,
    gravity: 0.2,
};

/// What a body looks like.
#[derive(Clone, Copy)]
enum Look {
    Crate,
    Steel,
    Wood,
    Balloon,
}

const BODIES: [(Body, Look); 7] = [
    // Alone, falling flat where it lands.
    (
        Body {
            name: "crate",
            shape: Shape::Box,
            size: 1.0,
            at: [-3.0, 3.0, 1.5],
            turn: [0.0; 3],
            stuff: WOOD,
        },
        Look::Crate,
    ),
    // A pile, tumbling.
    (
        Body {
            name: "big crate",
            shape: Shape::Box,
            size: 1.2,
            at: [1.5, 2.0, -0.5],
            turn: [0.0, 0.3, 0.0],
            stuff: WOOD,
        },
        Look::Crate,
    ),
    (
        Body {
            name: "small crate",
            shape: Shape::Box,
            size: 0.8,
            at: [1.7, 4.0, -0.3],
            turn: [0.4, 0.2, 0.1],
            stuff: WOOD,
        },
        Look::Crate,
    ),
    (
        Body {
            name: "wooden ball",
            shape: Shape::Ball,
            size: 0.9,
            at: [1.2, 6.0, -0.7],
            turn: [0.0; 3],
            stuff: Stuff {
                bounce: 0.45,
                ..WOOD
            },
        },
        Look::Wood,
    ),
    (
        Body {
            name: "tiny crate",
            shape: Shape::Box,
            size: 0.6,
            at: [1.6, 7.5, -0.4],
            turn: [0.5, 0.0, 0.7],
            stuff: WOOD,
        },
        Look::Crate,
    ),
    // Alone, landing where it falls.
    (
        Body {
            name: "steel ball",
            shape: Shape::Ball,
            size: 0.8,
            at: [3.0, 2.5, 2.5],
            turn: [0.0; 3],
            stuff: STEEL,
        },
        Look::Steel,
    ),
    // Floating down slowly.
    (
        Body {
            name: "balloon",
            shape: Shape::Ball,
            size: 1.2,
            at: [-1.0, 3.5, -2.5],
            turn: [0.0; 3],
            stuff: BALLOON,
        },
        Look::Balloon,
    ),
];

fn bodies() -> impl Iterator<Item = &'static Body> {
    BODIES.iter().map(|(body, _)| body)
}

/// Rings of wood, light and dark, wavy round the ball: from pixels made in Rust.
fn wood_grain() -> Texture {
    const SIZE: u32 = 64;
    const LIGHT: [f32; 3] = [200.0, 150.0, 95.0];
    const DARK: [f32; 3] = [135.0, 85.0, 45.0];
    let grain = pixels(SIZE, SIZE, |column, row| {
        let (across, down) = (column as f32 / SIZE as f32, row as f32 / SIZE as f32);
        // Whole waves round, so the texture meets itself where it goes round the ball.
        let wave = (across * TAU * 2.0).sin() * 0.04 + (across * TAU * 5.0).sin() * 0.015;
        let ring = ((down + wave) * 9.0 * TAU).cos() * 0.5 + 0.5;
        let [red, green, blue] =
            std::array::from_fn(|part| LIGHT[part] + (DARK[part] - LIGHT[part]) * ring);
        [red as u8, green as u8, blue as u8, 255]
    });
    Texture::pixels(SIZE, SIZE, grain).expect("the pixels are made as many as the size")
}

/// Tiles of slate, dark and darker, two by two.
fn slate() -> Texture {
    let tiles = pixels(2, 2, |column, row| {
        if (column + row) % 2 == 0 {
            [80, 95, 125, 255]
        } else {
            [55, 65, 95, 255]
        }
    });
    Texture::pixels(2, 2, tiles)
        .expect("the pixels are made as many as the size")
        .sharp()
}

pub fn render() -> DomNode {
    let meshes = Meshes::default();
    let world = Rc::new(std::cell::RefCell::new(World::new(&meshes, bodies())));
    let shown = shown_with_frames(&world);

    let wooden_crate =
        Material::lit(Color::WHITE).texture(Texture::url(include_static!("crate.png")));
    let material = |look: Look| match look {
        Look::Crate => wooden_crate.clone(),
        Look::Steel => Material::lit(Color::rgb(0.6, 0.62, 0.66)).specular(Color::WHITE, 128.0),
        Look::Wood => Material::lit(Color::WHITE)
            .texture(wood_grain())
            .specular(Color::rgb(0.3, 0.3, 0.3), 16.0),
        Look::Balloon => {
            Material::lit(Color::rgba(0.9, 0.12, 0.15, 0.85)).specular(Color::WHITE, 64.0)
        }
    };

    let held = Value::new("Drag the bodies around.".to_string());
    let objects = BODIES.iter().enumerate().map(|(index, (body, look))| {
        let (world, held, name) = (world.clone(), held.clone(), body.name);
        // Where its middle was when it was grabbed.
        let grabbed_at = Rc::new(Cell::new(Vec3::ZERO));
        let reach = FLOOR - body.size / 2.0;
        Object::new(meshes.of(body.shape).clone())
            .transform(shown.map(move |shown| shown.place(index)))
            .material(material(*look))
            .on_drag(move |drag: Drag| {
                let mut world = world.borrow_mut();
                match drag.phase {
                    DragPhase::Start => {
                        grabbed_at.set(world.middle(index).unwrap_or_default());
                        held.set(format!("Holding: {name}"));
                    }
                    DragPhase::Move => {}
                    DragPhase::End => {
                        world.let_go();
                        held.set(format!("Let go: {name}"));
                        return;
                    }
                }
                // Along the floor, the point grabbed under the pointer, within the walls.
                if let Some(moved) = drag.moved_on(Vec3::Y) {
                    let to = grabbed_at.get() + moved;
                    world.hold(
                        index,
                        Vec3::new(
                            to.x.clamp(-reach, reach),
                            to.y + LIFT,
                            to.z.clamp(-reach, reach),
                        ),
                    );
                }
            })
    });
    let floor = Material::lit(Color::WHITE)
        .texture(slate())
        .repeat(FLOOR, FLOOR);
    let objects: Vec<Object> = ground(&meshes, floor).chain(objects).collect();

    let weights = {
        let world = world.borrow();
        let weights: Vec<String> = bodies()
            .enumerate()
            .map(|(index, body)| format!("{} {:.2}", body.name, world.mass(index)))
            .collect();
        format!("Weights: {}.", weights.join(", "))
    };
    let status = shown.map(|shown| shown.status());
    let again = bind!(world, meshes, held, |_| {
        *world.borrow_mut() = World::new(&meshes, self::bodies());
        held.set("Dropped again.".to_string());
    });

    let camera = Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 8.0, 11.0), Vec3::new(0.0, 0.5, 0.0));
    let params = SceneParams::default().background(Color::rgb(0.05, 0.05, 0.15));

    dom! {
        <div>
            <p>
                "DPT: Dragging + Physics + Textures, the three stories in one. Wooden crates, a "
                "heavy steel ball, a wooden ball and a balloon light as air fall onto a floor of "
                "tiles, moved by rapier3d. Drag them around: held, a body pushes aside what's "
                "lighter and is held back by what's heavier, and let go while moving, it flies on."
            </p>
            <p>{weights}</p>
            <p>
                <button on_click={again}>"Drop again"</button>
                " " {status} " " {held}
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

    use super::{BALLOON, BODIES, STEEL, WOOD, bodies};
    use crate::world::{
        Body, FLOOR, Meshes, Shape, World,
        tests::{rest, run},
    };

    /// The index of the body of `name`.
    fn index(name: &str) -> usize {
        BODIES
            .iter()
            .position(|(body, _)| body.name == name)
            .unwrap_or_else(|| panic!("no {name}"))
    }

    #[test]
    fn the_bodies_fall_and_come_to_rest_on_the_floor() {
        let mut world = World::new(&Meshes::default(), bodies());
        assert!(rest(&mut world, 0, 8).is_some(), "at rest in 8 seconds");

        let shown = world.shown();
        for (index, (body, _)) in BODIES.iter().enumerate() {
            let at = shown.place(index).translation;
            assert!(
                at.x.abs() < FLOOR && at.z.abs() < FLOOR && at.y > 0.0 && at.y < 3.0,
                "{} on the floor, within the walls: {at}",
                body.name
            );
        }
        // The steel ball, alone, where it fell.
        let steel = shown.place(index("steel ball")).translation;
        assert!(steel.abs_diff_eq(Vec3::new(3.0, 0.4, 2.5), 0.05), "{steel}");
    }

    #[test]
    fn the_steel_ball_is_the_heaviest_and_the_balloon_the_lightest() {
        let world = World::new(&Meshes::default(), bodies());
        let mass = |name| world.mass(index(name));
        let (steel, wood, balloon) = (mass("steel ball"), mass("wooden ball"), mass("balloon"));
        for (body, _) in &BODIES {
            let other = mass(body.name);
            assert!(other <= steel && other >= balloon, "{}: {other}", body.name);
        }
        // Of the balls, the wooden one in the middle, far from both.
        assert!(
            steel > 5.0 * wood && wood > 5.0 * balloon,
            "{steel}, {wood}, {balloon}"
        );
    }

    #[test]
    fn a_body_held_goes_where_it_is_held_and_falls_when_let_go() {
        let mut world = World::new(&Meshes::default(), bodies());
        let Some(rested) = rest(&mut world, 0, 8) else {
            panic!("at rest in 8 seconds");
        };
        let ball = index("steel ball");
        let to = Vec3::new(0.0, 0.7, 4.0);

        world.hold(ball, to);
        let held = run(&mut world, rested, 1.5);
        let at = world.shown().place(ball).translation;
        assert!(at.abs_diff_eq(to, 0.05), "held at {to}: {at}");

        world.let_go();
        assert!(rest(&mut world, held, 6).is_some(), "at rest again");
        let at = world.shown().place(ball).translation;
        assert!(
            at.abs_diff_eq(Vec3::new(to.x, 0.4, to.z), 0.1),
            "fallen down to the floor: {at}"
        );
    }

    /// A crate and `pusher` side by side, the pusher held and drawn through the crate: how far
    /// the crate went.
    fn crate_pushed_by(pusher: Body) -> f32 {
        let crate_ = Body {
            name: "crate",
            shape: Shape::Box,
            size: 1.0,
            at: [0.0, 0.5, 0.0],
            turn: [0.0; 3],
            stuff: WOOD,
        };
        let mut world = World::new(&Meshes::default(), [&crate_, &pusher]);
        let Some(rested) = rest(&mut world, 0, 8) else {
            panic!("at rest in 8 seconds");
        };
        let at = world.shown().place(1).translation;
        world.hold(1, Vec3::new(-4.0, at.y, 0.0));
        run(&mut world, rested, 2.0);
        world
            .shown()
            .place(0)
            .translation
            .distance(Vec3::new(0.0, 0.5, 0.0))
    }

    #[test]
    fn a_body_held_pushes_aside_what_is_lighter_only() {
        let ball = |stuff| Body {
            name: "ball",
            shape: Shape::Ball,
            size: 0.8,
            at: [2.0, 0.4, 0.0],
            turn: [0.0; 3],
            stuff,
        };
        let by_steel = crate_pushed_by(ball(STEEL));
        let by_balloon = crate_pushed_by(ball(BALLOON));
        assert!(
            by_steel > 1.0,
            "the steel ball sweeps the crate aside: {by_steel}"
        );
        assert!(by_balloon < 0.05, "the balloon can't move it: {by_balloon}");
    }
}
