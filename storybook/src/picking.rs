//! Cubes told of clicks and of the pointer over them, and a ball in front of one, which isn't but
//! is in the way.

use vertigo::{Computed, DomNode, Value, dom};
use vertigo_3d::{
    Camera, Color, Material, Mesh, Object, Pick, Scene, SceneParams, Transform, math::Vec3,
};

use crate::background::css_frame;

pub fn render() -> DomNode {
    let over = Value::new("nothing");
    let clicked = Value::new("Clicked: nothing".to_string());

    let cube = Mesh::cube();
    let cubes = [("left", -2.0), ("middle", 0.0), ("right", 2.0)].map(|(name, x)| {
        let picked = Value::new(false);
        let hovered = Value::new(false);
        let material = Computed::from({
            let (picked, hovered) = (picked.clone(), hovered.clone());
            move |context| {
                Material::lit(match (picked.get(context), hovered.get(context)) {
                    (_, true) => Color::rgb(1.0, 0.6, 0.2),
                    (true, false) => Color::rgb(0.3, 0.8, 0.4),
                    (false, false) => Color::rgb(0.5, 0.6, 0.9),
                })
            }
        });

        Object::new(cube.clone())
            .transform(Transform::from_translation(Vec3::new(x, 0.5, 0.0)))
            .material(material)
            .on_click({
                let clicked = clicked.clone();
                move |Pick { point, .. }| {
                    picked.change(|picked| *picked = !*picked);
                    clicked.set(format!(
                        "Clicked: {name} at ({:.2}, {:.2}, {:.2})",
                        point.x, point.y, point.z
                    ));
                }
            })
            .on_hover({
                let over = over.clone();
                move |now| {
                    hovered.set(now);
                    over.change(|over| match now {
                        true => *over = name,
                        // Unless the pointer went from this cube straight onto another.
                        false if *over == name => *over = "nothing",
                        false => {}
                    });
                }
            })
    });

    let floor = Object::new(Mesh::plane())
        .transform(Transform::from_scale(Vec3::new(12.0, 1.0, 12.0)))
        .material(Material::lit(Color::rgb(0.6, 0.6, 0.6)));
    // On the way from the camera to the right cube's middle.
    let ball = Object::new(Mesh::sphere(32))
        .transform(
            Transform::from_translation(Vec3::new(4.0, 3.0, 6.0) / 3.0)
                .with_scale(Vec3::splat(0.4)),
        )
        .material(Material::lit(Color::rgb(0.9, 0.9, 0.9)));

    let objects: Vec<Object> = [floor, ball].into_iter().chain(cubes).collect();
    let camera = Camera::perspective(45_f32.to_radians())
        .looking_at(Vec3::new(0.0, 2.0, 6.0), Vec3::new(0.0, 0.5, 0.0));
    let params = SceneParams::default().background(Color::rgb(0.05, 0.05, 0.15));

    let over = over.map(|over| format!("Over: {over}"));

    dom! {
        <div>
            <p>{over}</p>
            <p>{clicked}</p>
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
