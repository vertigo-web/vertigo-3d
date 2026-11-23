use std::fmt;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Route {
    Background,
    Cubes,
    Dpt,
    Dragging,
    Glass,
    Layers,
    Light,
    Physics,
    Picking,
    Relief,
    Shaders,
    Spinning,
    Textures,
    Thousands,
}

impl From<String> for Route {
    fn from(path: String) -> Self {
        match path.as_str() {
            "/background" => Self::Background,
            "/cubes" => Self::Cubes,
            "/dpt" => Self::Dpt,
            "/dragging" => Self::Dragging,
            "/glass" => Self::Glass,
            "/layers" => Self::Layers,
            "/light" => Self::Light,
            "/physics" => Self::Physics,
            "/picking" => Self::Picking,
            "/relief" => Self::Relief,
            "/shaders" => Self::Shaders,
            "/spinning" => Self::Spinning,
            "/textures" => Self::Textures,
            "/thousands" => Self::Thousands,
            _ => Self::Cubes,
        }
    }
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Background => write!(f, "/background"),
            Self::Cubes => write!(f, "/cubes"),
            Self::Dpt => write!(f, "/dpt"),
            Self::Dragging => write!(f, "/dragging"),
            Self::Glass => write!(f, "/glass"),
            Self::Layers => write!(f, "/layers"),
            Self::Light => write!(f, "/light"),
            Self::Physics => write!(f, "/physics"),
            Self::Picking => write!(f, "/picking"),
            Self::Relief => write!(f, "/relief"),
            Self::Shaders => write!(f, "/shaders"),
            Self::Spinning => write!(f, "/spinning"),
            Self::Textures => write!(f, "/textures"),
            Self::Thousands => write!(f, "/thousands"),
        }
    }
}
