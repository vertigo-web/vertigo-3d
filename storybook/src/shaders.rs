//! A ball with code of its own in its shaders, going on with the time of the frames: plain,
//! rippling, running in the colors of a rainbow, or both.

use vertigo::{DomNode, Value, bind, dom};
use vertigo_3d::{
    Camera, Color, Material, Mesh, Object, Scene, SceneParams, Shader, Transform, frame_time,
    math::Vec3,
};

use crate::background::css_frame;

/// Ripples running up the ball.
const RIPPLES: &str = "position += normal * 0.06 * sin(position.y * 14.0 + time * 5.0);";
/// Colors running down the ball, each part of its own.
const RAINBOW: &str = "pixel.rgb *= 0.5 + 0.5 * vec3(
    sin(time * 3.0 + world.y * 4.0),
    sin(time * 4.0 + world.y * 4.0 + 2.1),
    sin(time * 5.0 + world.y * 4.0 + 4.2));";
/// The ripples for WebGPU, in WGSL, which reads alike here.
const RIPPLES_WGSL: &str = RIPPLES;
/// The colors for WebGPU, in WGSL, which sets the colors of a pixel with its alpha.
const RAINBOW_WGSL: &str = "pixel = vec4f(pixel.rgb * (0.5 + 0.5 * vec3f(
    sin(time * 3.0 + world.y * 4.0),
    sin(time * 4.0 + world.y * 4.0 + 2.1),
    sin(time * 5.0 + world.y * 4.0 + 4.2))), pixel.a);";

pub fn render() -> DomNode {
    let chosen = Value::new("Plain");
    let ripples = || Shader::new().vertex(RIPPLES).vertex_wgsl(RIPPLES_WGSL);
    let rainbow = |shader: Shader| shader.fragment(RAINBOW).fragment_wgsl(RAINBOW_WGSL);
    let (rippling, rainbow, both) = (ripples(), rainbow(Shader::new()), rainbow(ripples()));
    let material = chosen.map(move |chosen| {
        let ball = Material::lit(Color::rgb(0.9, 0.9, 0.9)).specular(Color::WHITE, 40.0);
        match chosen {
            "Rippling" => ball.shader(rippling.clone()),
            "Rainbow" => ball.shader(rainbow.clone()),
            "Both" => ball.shader(both.clone()),
            _ => ball,
        }
    });

    let buttons: Vec<DomNode> = ["Plain", "Rippling", "Rainbow", "Both"]
        .into_iter()
        .map(|name| {
            let on_click = bind!(chosen, name, |_| chosen.set(name));
            dom! { <button on_click={on_click}>{name}</button> }
        })
        .collect();

    let ball = Object::new(Mesh::sphere(96))
        .transform(Transform::from_scale(Vec3::splat(2.0)))
        .material(material);
    let camera =
        Camera::perspective(45_f32.to_radians()).looking_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO);
    let params = SceneParams::default()
        .background(Color::rgb(0.05, 0.05, 0.15))
        .time(frame_time());

    dom! {
        <div>
            <p>{..buttons}</p>
            <div css={css_frame()}>
                <Scene
                    camera={Value::new(camera).to_computed()}
                    objects={Value::new(vec![ball]).to_computed()}
                    {params}
                />
            </div>
        </div>
    }
}
