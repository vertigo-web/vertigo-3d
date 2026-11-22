//! A lit cube turning with each frame, by `frame_time`.

use vertigo::{DomNode, Value, dom};
use vertigo_3d::{
    Camera, Color, Material, Mesh, Object, Scene, SceneParams, Transform, frame_time,
    math::{Quat, Vec3},
};

use crate::background::css_frame;

const ORANGE: Color = Color::rgb(1.0, 0.6, 0.2);

pub fn render() -> DomNode {
    // Half a turn a second, around the up axis.
    let turning = frame_time()
        .map(|time| Transform::from_rotation(Quat::from_rotation_y(time * std::f32::consts::PI)));
    let cube = Object::new(Mesh::cube())
        .transform(turning)
        .material(Material::lit(ORANGE));

    let camera =
        Camera::perspective(45_f32.to_radians()).looking_at(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO);
    let params = SceneParams::default().background(Color::rgb(0.05, 0.05, 0.15));

    dom! {
        <div>
            <p>"A cube turning half a turn a second, drawn with each frame while it's shown."</p>
            <div css={css_frame()}>
                <Scene
                    camera={Value::new(camera).to_computed()}
                    objects={Value::new(vec![cube]).to_computed()}
                    {params}
                />
            </div>
        </div>
    }
}
