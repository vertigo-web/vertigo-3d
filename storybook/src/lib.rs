use std::rc::Rc;

use vertigo::{DomNode, css, dom, main, router::Router};
use vertigo_forms::{Tab, Tabs, TabsParams};

mod background;
mod cubes;
mod dpt;
mod dragging;
mod glass;
mod layers;
mod light;
mod physics;
mod picking;
mod relief;
mod route;
mod shaders;
mod spinning;
mod textures;
mod thousands;
mod world;

use route::Route;

#[main]
fn app() -> DomNode {
    let current_tab = Router::<Route>::new_history_router();

    let tabs = vec![
        Tab {
            key: Route::Cubes,
            name: "Cubes".to_string(),
            render: Rc::new(|_: &Route| cubes::render()),
        },
        Tab {
            key: Route::Light,
            name: "Light".to_string(),
            render: Rc::new(|_: &Route| light::render()),
        },
        Tab {
            key: Route::Picking,
            name: "Picking".to_string(),
            render: Rc::new(|_: &Route| picking::render()),
        },
        Tab {
            key: Route::Dragging,
            name: "Dragging".to_string(),
            render: Rc::new(|_: &Route| dragging::render()),
        },
        Tab {
            key: Route::Physics,
            name: "Physics".to_string(),
            render: Rc::new(|_: &Route| physics::render()),
        },
        Tab {
            key: Route::Dpt,
            name: "DPT".to_string(),
            render: Rc::new(|_: &Route| dpt::render()),
        },
        Tab {
            key: Route::Textures,
            name: "Textures".to_string(),
            render: Rc::new(|_: &Route| textures::render()),
        },
        Tab {
            key: Route::Relief,
            name: "Relief".to_string(),
            render: Rc::new(|_: &Route| relief::render()),
        },
        Tab {
            key: Route::Shaders,
            name: "Shaders".to_string(),
            render: Rc::new(|_: &Route| shaders::render()),
        },
        Tab {
            key: Route::Glass,
            name: "Glass".to_string(),
            render: Rc::new(|_: &Route| glass::render()),
        },
        Tab {
            key: Route::Layers,
            name: "Layers".to_string(),
            render: Rc::new(|_: &Route| layers::render()),
        },
        Tab {
            key: Route::Spinning,
            name: "Spinning".to_string(),
            render: Rc::new(|_: &Route| spinning::render()),
        },
        Tab {
            key: Route::Thousands,
            name: "Thousands".to_string(),
            render: Rc::new(|_: &Route| thousands::render()),
        },
        Tab {
            key: Route::Background,
            name: "Background".to_string(),
            render: Rc::new(|_: &Route| background::render()),
        },
    ];

    dom! {
        <html>
            <head />
            <body>
                <Tabs
                    {&current_tab}
                    {tabs}
                    params={bordered_tabs()}
                />
            </body>
        </html>
    }
}

pub fn bordered_tabs() -> TabsParams {
    TabsParams {
        header_item_add_css: css! {"
            border: 1px solid black;
            padding: 0px 10px;
        "},
        header_active_item_add_css: css! {"
            background-color: lightgray;
        "},
        content_css: css! {"
            border: solid 1px black;
            padding: 5px 10px;
        "},
        ..Default::default()
    }
}
