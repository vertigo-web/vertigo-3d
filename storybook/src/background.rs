//! A scene's background, chosen with buttons: the scene is drawn again when it changes.

use vertigo::{Css, DomNode, Value, bind, css, dom};
use vertigo_3d::{Camera, Color, Scene, SceneParams};

const COLORS: [(&str, Color); 3] = [
    ("Night", Color::rgb(0.05, 0.05, 0.15)),
    ("Sky", Color::rgb(0.53, 0.81, 0.92)),
    ("Sunset", Color::rgb(0.98, 0.55, 0.24)),
];

pub fn render() -> DomNode {
    let background = Value::new(COLORS[0].1);

    let buttons: Vec<DomNode> = COLORS
        .iter()
        .map(|(name, color)| {
            let on_click = bind!(background, color, |_| background.set(color));
            dom! { <button on_click={on_click}>{*name}</button> }
        })
        .collect();

    let params = SceneParams::default().background(background);

    dom! {
        <div>
            <p>{..buttons}</p>
            <div css={css_frame()}>
                <Scene
                    camera={Value::new(Camera::default()).to_computed()}
                    objects={Value::new(Vec::new()).to_computed()}
                    {params}
                />
            </div>
        </div>
    }
}

pub fn css_frame() -> Css {
    css! {"
        width: 1280px;
        height: 800px;
        border: 1px solid #ccc;
    "}
}
