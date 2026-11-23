//! rapier3d's world under the stories "Physics" and "DPT": bodies of boxes and balls falling onto a
//! floor walled round, stepped with each frame, the scene's objects taking their places. The
//! shape of each body is its object's mesh's box (`Mesh::bounds`), scaled as the object.
//!
//! rapier3d works with a glam of its own version, so its vectors and rotations go to and from
//! vertigo-3d's as arrays.

use std::{cell::RefCell, rc::Rc};

use rapier3d::prelude::{
    ColliderBuilder, PhysicsWorld, RigidBody, RigidBodyBuilder, RigidBodyHandle, Rotation, Vector,
};
use vertigo::{Computed, Value, get_driver};
use vertigo_3d::{
    Color, Material, Mesh, Object, Transform,
    math::{Quat, Vec3},
};

/// Half of the floor's side: it goes from -6 to 6 along X and Z.
pub const FLOOR: f32 = 6.0;

/// The most of the time since the last frame that the world catches up with: after a pause, f.
/// ex. in a tab hidden, it goes on from where it was, and with a slow GPU it goes slower.
const MOST_BEHIND: f32 = 0.25;

/// How fast a body held is drawn to where it's held, for each unit away, a second.
const PULL: f32 = 10.0;

/// The fastest a body held is drawn, in units a second.
const FASTEST: f32 = 8.0;

/// The most a body held speeds up or slows down, in units a second, each second: all bodies
/// follow the pointer alike, and push what's in the way as hard as they are heavy.
const GRIP: f32 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Box,
    Ball,
}

/// What a body is made of, as it moves.
#[derive(Clone, Copy, Debug)]
pub struct Stuff {
    /// How heavy it is for its size: water's is 1.
    pub density: f32,
    /// How much of its speed it keeps bouncing off something, from 0 to 1.
    pub bounce: f32,
    /// How much the air slows it down.
    pub drag: f32,
    /// How much of the weight of it pulls it down: less than 1 for what the air holds up.
    pub gravity: f32,
}

/// A body: its name, shape, size, where its middle starts, how it's turned there - an axis as
/// long as the angle - and what it's made of.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub name: &'static str,
    pub shape: Shape,
    pub size: f32,
    pub at: [f32; 3],
    pub turn: [f32; 3],
    pub stuff: Stuff,
}

/// The floor, a slab under Y = 0, and the walls round it: the cube's transform for each.
fn fixed() -> [Transform; 5] {
    let (long, thick, high) = (2.0 * FLOOR + 0.4, 0.2, 0.6);
    let wall = |x: f32, z: f32, scale: Vec3| {
        Transform::from_translation(Vec3::new(x, high / 2.0, z)).with_scale(scale)
    };
    [
        Transform::from_translation(Vec3::new(0.0, -0.1, 0.0)).with_scale(Vec3::new(
            2.0 * FLOOR,
            0.2,
            2.0 * FLOOR,
        )),
        wall(0.0, -FLOOR - 0.1, Vec3::new(long, high, thick)),
        wall(0.0, FLOOR + 0.1, Vec3::new(long, high, thick)),
        wall(-FLOOR - 0.1, 0.0, Vec3::new(thick, high, long)),
        wall(FLOOR + 0.1, 0.0, Vec3::new(thick, high, long)),
    ]
}

/// The floor, of `floor`, and the walls round it.
pub fn ground(meshes: &Meshes, floor: Material) -> impl Iterator<Item = Object> {
    let walls = Material::lit(Color::rgb(0.45, 0.35, 0.3));
    let cube = meshes.cube.clone();
    fixed()
        .into_iter()
        .enumerate()
        .map(move |(index, transform)| {
            Object::new(cube.clone())
                .transform(transform)
                .material(if index == 0 {
                    floor.clone()
                } else {
                    walls.clone()
                })
        })
}

/// The meshes of the shapes, shared by the objects and the bodies made of them.
#[derive(Clone)]
pub struct Meshes {
    pub cube: Mesh,
    pub ball: Mesh,
}

impl Default for Meshes {
    fn default() -> Self {
        Self {
            cube: Mesh::cube(),
            ball: Mesh::sphere(24),
        }
    }
}

impl Meshes {
    pub fn of(&self, shape: Shape) -> &Mesh {
        match shape {
            Shape::Box => &self.cube,
            Shape::Ball => &self.ball,
        }
    }
}

/// What the scene shows of the world: where each body is, and how many of them move.
#[derive(Clone, PartialEq)]
pub struct Shown {
    pub places: Vec<Transform>,
    pub moving: usize,
}

impl Shown {
    /// Where the body at `index` is.
    pub fn place(&self, index: usize) -> Transform {
        self.places.get(index).copied().unwrap_or_default()
    }

    /// Whether the bodies are all at rest, or how many of them move.
    pub fn status(&self) -> String {
        match self.moving {
            0 => "All at rest.".to_string(),
            moving => format!("Moving: {moving} of {}", self.places.len()),
        }
    }
}

/// A body in the world: its handle, its size, and how much of its weight pulls it down.
struct Placed {
    handle: RigidBodyHandle,
    size: f32,
    gravity: f32,
}

/// rapier3d's world, with its bodies in the order given.
pub struct World {
    pub physics: PhysicsWorld,
    bodies: Vec<Placed>,
    /// The body held, by its index, and where its middle is drawn to.
    held: Option<(usize, Vec3)>,
    /// The time of the frame the world was stepped to, in milliseconds.
    last_frame: Option<f64>,
    /// The time since then not stepped yet, less than a step, in seconds.
    behind: f32,
}

impl World {
    /// `bodies` where they start, still, above the floor.
    pub fn new<'a>(meshes: &Meshes, bodies: impl IntoIterator<Item = &'a Body>) -> Self {
        let mut physics = PhysicsWorld::new();

        for transform in fixed() {
            physics.insert(
                RigidBodyBuilder::fixed().translation(to_rapier(transform.translation)),
                shape_of(&meshes.cube, Shape::Box, transform.scale),
            );
        }

        let bodies = bodies
            .into_iter()
            .map(|body| {
                // rapier3d has no resistance to rolling: without more of a damping, a ball
                // rolls on slowly for long, and the world comes to rest late.
                let turning_damping = match body.shape {
                    Shape::Box => 0.5,
                    Shape::Ball => 6.0,
                };
                let rigid = RigidBodyBuilder::dynamic()
                    .translation(Vector::from_array(body.at))
                    .rotation(Vector::from_array(body.turn))
                    .linear_damping(body.stuff.drag)
                    .angular_damping(turning_damping)
                    .gravity_scale(body.stuff.gravity);
                let shape = shape_of(meshes.of(body.shape), body.shape, Vec3::splat(body.size))
                    .density(body.stuff.density)
                    .restitution(body.stuff.bounce);
                Placed {
                    handle: physics.insert(rigid, shape).0,
                    size: body.size,
                    gravity: body.stuff.gravity,
                }
            })
            .collect();

        Self {
            physics,
            bodies,
            held: None,
            last_frame: None,
            behind: 0.0,
        }
    }

    /// Steps the world on to the frame at `time`, in milliseconds, by steps of a 60th of a
    /// second: the same however often the browser draws.
    pub fn step_to(&mut self, time: f64) {
        let since = self
            .last_frame
            .map_or(0.0, |last| ((time - last) / 1000.0) as f32);
        self.last_frame = Some(time);

        let step = self.physics.integration_parameters.dt;
        self.behind = (self.behind + since).min(MOST_BEHIND);
        while self.behind >= step {
            self.steer(step);
            self.physics.step();
            self.behind -= step;
        }
    }

    /// Where the bodies are, as the scene shows them.
    pub fn shown(&self) -> Shown {
        let rigid = self.bodies.iter().filter_map(|placed| {
            self.physics
                .bodies
                .get(placed.handle)
                .map(|rigid| (rigid, placed.size))
        });
        Shown {
            places: rigid
                .clone()
                .map(|(rigid, size)| {
                    Transform::from_translation(to_scene(rigid.translation()))
                        .with_rotation(rotation_to_scene(rigid.rotation()))
                        .with_scale(Vec3::splat(size))
                })
                .collect(),
            moving: rigid.filter(|(rigid, _)| !rigid.is_sleeping()).count(),
        }
    }

    /// The middle of the body at `index`, in the scene.
    pub fn middle(&self, index: usize) -> Option<Vec3> {
        self.rigid(index).map(|rigid| to_scene(rigid.translation()))
    }

    /// How heavy the body at `index` is.
    pub fn mass(&self, index: usize) -> f32 {
        self.rigid(index).map_or(0.0, RigidBody::mass)
    }

    /// Throws the body at `index` up, by `speed` in units a second, from `point` of it, in the
    /// scene: it spins unless the point is right above or below its middle.
    pub fn throw(&mut self, index: usize, point: Vec3, speed: f32) {
        if let Some(rigid) = self.rigid_mut(index) {
            let impulse = Vector::Y * rigid.mass() * speed;
            rigid.apply_impulse_at_point(impulse, to_rapier(point), true);
        }
    }

    /// Holds the body at `index`, its middle drawn to `to` as it can, weightless while held.
    pub fn hold(&mut self, index: usize, to: Vec3) {
        if let Some(rigid) = self.rigid_mut(index) {
            rigid.set_gravity_scale(0.0, true);
            self.held = Some((index, to));
        }
    }

    /// Lets go of the body held: it falls again, flying on as it moved.
    pub fn let_go(&mut self) {
        let Some((index, _)) = self.held.take() else {
            return;
        };
        let gravity = self.bodies.get(index).map_or(1.0, |placed| placed.gravity);
        if let Some(rigid) = self.rigid_mut(index) {
            rigid.set_gravity_scale(gravity, true);
        }
    }

    /// Draws the body held towards where it's held, for a step of `step` seconds: as fast as
    /// it's away, but speeding up by at most [`GRIP`], so it pushes only as hard as it's heavy.
    fn steer(&mut self, step: f32) {
        let Some((index, to)) = self.held else {
            return;
        };
        let Some(rigid) = self.rigid_mut(index) else {
            return;
        };
        let at = to_scene(rigid.translation());
        let now = to_scene(rigid.linvel());
        let wanted = ((to - at) * PULL).clamp_length_max(FASTEST);
        let change = (wanted - now).clamp_length_max(GRIP * step);
        rigid.set_linvel(to_rapier(now + change), true);
        // Held, it stops spinning.
        let spin = rigid.angvel() * 0.9;
        rigid.set_angvel(spin, true);
    }

    fn rigid(&self, index: usize) -> Option<&RigidBody> {
        self.bodies
            .get(index)
            .and_then(|placed| self.physics.bodies.get(placed.handle))
    }

    fn rigid_mut(&mut self, index: usize) -> Option<&mut RigidBody> {
        self.bodies
            .get(index)
            .and_then(|placed| self.physics.bodies.get_mut(placed.handle))
    }
}

/// Where the bodies of `world` are, stepped with each frame while the scene showing them is.
/// Once all the bodies are at rest, the places stay the same, and the scene isn't drawn again.
pub fn shown_with_frames(world: &Rc<RefCell<World>>) -> Computed<Shown> {
    Value::with_connect(world.borrow().shown(), {
        let world = world.clone();
        move |shown| {
            let (world, shown) = (world.clone(), shown.clone());
            get_driver().on_animation_frame(move |time| {
                let mut world = world.borrow_mut();
                world.step_to(time);
                shown.set(world.shown());
            })
        }
    })
}

/// The shape of a body of `mesh`, scaled by `scale`, as big as its object: half its box each
/// way from the middle for a box, half its width round for a ball.
fn shape_of(mesh: &Mesh, shape: Shape, scale: Vec3) -> ColliderBuilder {
    let half = mesh.bounds().size() * scale / 2.0;
    match shape {
        Shape::Box => ColliderBuilder::cuboid(half.x, half.y, half.z),
        Shape::Ball => ColliderBuilder::ball(half.x),
    }
}

fn to_rapier(vector: Vec3) -> Vector {
    Vector::from_array(vector.to_array())
}

fn to_scene(vector: Vector) -> Vec3 {
    Vec3::from_array(vector.to_array())
}

fn rotation_to_scene(rotation: &Rotation) -> Quat {
    Quat::from_array(rotation.to_array())
}

#[cfg(test)]
pub mod tests {
    use super::World;

    /// A 60th of a second, in milliseconds: the frames of a browser drawing 60 a second.
    pub const FRAME: f64 = 1000.0 / 60.0;

    /// Steps `world` on frame after frame from `frame`, for at most `seconds`, until all its
    /// bodies are at rest: the frame they are, if they come to be.
    pub fn rest(world: &mut World, frame: u32, seconds: u32) -> Option<u32> {
        (frame + 1..=frame + seconds * 60).find(|frame| {
            world.step_to(f64::from(*frame) * FRAME);
            world.shown().moving == 0
        })
    }

    /// Steps `world` on frame after frame from `frame`, for `seconds`: the frame it's at then.
    pub fn run(world: &mut World, frame: u32, seconds: f32) -> u32 {
        let last = frame + (seconds * 60.0) as u32;
        (frame + 1..=last).for_each(|frame| world.step_to(f64::from(frame) * FRAME));
        last
    }
}
