//! A scene's frame as its renderer draws it: its opaque instances in batches of a mesh and
//! surface, its see-through ones the farthest first, and its textures with what became of their
//! images. It is drawn with WebGL, see `gl`, or with WebGPU, see `gpu`.

mod gl;
mod gpu;

use vertigo::webgl::ImageLoad;

pub(crate) use self::{gl::Renderer as GlRenderer, gpu::Renderer as GpuRenderer};
use crate::{
    Camera, Color, Light, Loading, Material, Mesh, Object, Texture, Transform, layer::Drawn,
    material::Surface, texture::Image,
};

/// All a scene's drawing depends on: when it changes, the scene is drawn.
#[derive(Clone, PartialEq)]
pub(crate) struct Frame {
    pub camera: Camera,
    pub background: Color,
    pub lights: Vec<Light>,
    /// What shaders read as `time`.
    pub time: f32,
    /// Whether normal and height maps show.
    pub relief: bool,
    /// Textures loaded though no object may show them yet.
    pub preload: Vec<Texture>,
    pub instances: Vec<Instance>,
    pub layers: Vec<Drawn>,
}

impl Frame {
    /// Whether only WebGL draws it as it is: a shader of it has no code of WGSL for its code of
    /// GLSL, or a layer has no way of WebGPU.
    pub fn needs_webgl(&self) -> bool {
        self.instances.iter().any(|instance| {
            instance
                .material
                .surface()
                .shader
                .as_ref()
                .is_some_and(|shader| !shader.has_wgsl())
        }) || self.layers.iter().any(|drawn| !drawn.layer.has_webgpu())
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct Instance {
    pub object: Object,
    pub transform: Transform,
    pub material: Material,
}

/// Floats an instance has in its group's buffer: its matrix, its color, 1 if it's lit, how
/// many times its texture repeats across and down, the color and shininess of its highlights,
/// and the depth of its height map.
pub(crate) const INSTANCE: usize = 28;

impl Instance {
    fn put(&self, data: &mut Vec<f32>) {
        let color = self.material.base_color();
        let repeat = self.material.repeated();
        data.extend(self.transform.to_matrix().to_cols_array());
        data.extend([color.red, color.green, color.blue, color.alpha]);
        let (specular, shininess) = self.material.highlights();
        data.push(if self.material.is_lit() { 1.0 } else { 0.0 });
        data.extend([repeat.x, repeat.y]);
        data.extend([specular.red, specular.green, specular.blue, shininess]);
        data.push(self.material.depth());
    }

    fn mesh(&self) -> &Mesh {
        self.object.mesh()
    }
}

/// The opaque instances of a mesh and textures, as their buffer holds them.
#[derive(Debug, PartialEq)]
pub(crate) struct Batch {
    pub mesh: Mesh,
    pub surface: Surface,
    pub data: Vec<f32>,
}

/// The opaque instances of each mesh and textures in `instances`, in the order they first come.
pub(crate) fn batches(instances: &[Instance]) -> Vec<Batch> {
    let mut batches: Vec<Batch> = Vec::new();

    for instance in instances
        .iter()
        .filter(|instance| !instance.material.is_see_through())
    {
        let surface = instance.material.surface();
        let index = batches
            .iter()
            .position(|batch| batch.mesh == *instance.mesh() && batch.surface == *surface);
        let index = index.unwrap_or_else(|| {
            batches.push(Batch {
                mesh: instance.mesh().clone(),
                surface: surface.clone(),
                data: Vec::new(),
            });
            batches.len() - 1
        });
        instance.put(&mut batches[index].data);
    }

    batches
}

/// See-through instances drawn with a call, one after another in their mesh's buffer.
#[derive(Debug, PartialEq)]
pub(crate) struct Run {
    /// Of the meshes [`see_through`] gives.
    pub mesh: usize,
    pub surface: Surface,
    pub first: u32,
    pub count: u32,
}

/// The see-through instances of `instances`, the farthest from `camera` first: the instances of
/// each mesh as their buffer holds them, and the runs drawing them in that order, each of one
/// mesh and textures.
pub(crate) fn see_through(
    instances: &[Instance],
    camera: &Camera,
) -> (Vec<(Mesh, Vec<f32>)>, Vec<Run>) {
    let forward = (camera.target - camera.eye).normalize_or_zero();
    let depth = |instance: &Instance| {
        let center = instance.mesh().data().bounds.center();
        let center = instance.transform.to_matrix().transform_point3(center);
        (center - camera.eye).dot(forward)
    };
    let mut sorted: Vec<(f32, &Instance)> = instances
        .iter()
        .filter(|instance| instance.material.is_see_through())
        .map(|instance| (depth(instance), instance))
        .collect();
    sorted.sort_by(|(a, _), (b, _)| b.total_cmp(a));

    let mut meshes: Vec<(Mesh, Vec<f32>)> = Vec::new();
    let mut runs: Vec<Run> = Vec::new();
    for (_, instance) in sorted {
        let mesh = match meshes.iter().position(|(mesh, _)| mesh == instance.mesh()) {
            Some(mesh) => mesh,
            None => {
                meshes.push((instance.mesh().clone(), Vec::new()));
                meshes.len() - 1
            }
        };
        let data = &mut meshes[mesh].1;
        let first = (data.len() / INSTANCE) as u32;
        instance.put(data);

        let surface = instance.material.surface();
        match runs.last_mut() {
            Some(run) if run.mesh == mesh && run.surface == *surface => run.count += 1,
            _ => runs.push(Run {
                mesh,
                surface: surface.clone(),
                first,
                count: 1,
            }),
        }
    }

    (meshes, runs)
}

/// A texture on a context, and what became of its image.
pub(crate) struct Made<T> {
    pub texture: Texture,
    pub made: T,
    pub image: Arrival,
}

/// What became of the image of a texture.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Arrival {
    /// Of pixels, in at once.
    Pixels,
    /// From a URL, coming.
    Coming,
    Came,
    Failed,
}

impl<T> Made<T> {
    pub fn new(texture: Texture, made: T) -> Self {
        let image = match texture.data().image {
            Image::Url(_) => Arrival::Coming,
            Image::Pixels { .. } => Arrival::Pixels,
        };
        Self {
            texture,
            made,
            image,
        }
    }

    fn is_coming_from(&self, url: &str) -> bool {
        self.image == Arrival::Coming
            && matches!(&self.texture.data().image, Image::Url(from) if from == url)
    }
}

/// An image came or failed: a texture of `textures` of its URL waiting for it has it, or stays
/// white.
pub(crate) fn arrived<T>(textures: &mut [Made<T>], load: &ImageLoad) {
    let (url, arrival) = match load {
        ImageLoad::Loaded { url, .. } => (url, Arrival::Came),
        ImageLoad::Failed { url } => (url, Arrival::Failed),
    };
    if let Some(made) = textures.iter_mut().find(|made| made.is_coming_from(url)) {
        made.image = arrival;
    }
}

/// How far the images of `textures` from URLs are loaded.
pub(crate) fn loading_of<T>(textures: &[Made<T>]) -> Loading {
    loading(textures.iter().map(|made| made.image))
}

/// How far the images of `arrivals` are loaded.
fn loading(arrivals: impl Iterator<Item = Arrival>) -> Loading {
    let mut loading = Loading::default();
    for arrival in arrivals {
        match arrival {
            Arrival::Pixels => continue,
            Arrival::Coming => {}
            Arrival::Came => loading.loaded += 1,
            Arrival::Failed => loading.failed += 1,
        }
        loading.all += 1;
    }
    loading
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::{Arrival, Batch, INSTANCE, Instance, Run, batches, loading, see_through};
    use crate::material::Surface;
    use crate::{Camera, Color, Loading, Material, Mesh, Object, Texture, Transform};

    fn instance(mesh: &Mesh, x: f32, material: Material) -> Instance {
        Instance {
            object: Object::new(mesh.clone()),
            transform: Transform::from_translation(Vec3::new(x, 0.0, 0.0)),
            material,
        }
    }

    fn violet() -> Material {
        Material::color(Color::rgb(0.5, 0.25, 1.0))
    }

    #[test]
    fn opaque_instances_of_a_mesh_and_texture_are_drawn_together_in_the_order_they_come() {
        let (cube, ball) = (Mesh::cube(), Mesh::sphere(8));
        let bricks = Texture::url("/bricks.png");
        let batches = batches(&[
            instance(&cube, 1.0, violet()),
            instance(&ball, 2.0, violet()),
            instance(&cube, 3.0, violet().repeat(2.0, 4.0)),
            instance(&cube, 4.0, violet().texture(bricks.clone())),
            instance(&cube, 5.0, Material::color(Color::rgba(1.0, 1.0, 1.0, 0.5))),
        ]);

        let kinds: Vec<_> = batches
            .iter()
            .map(
                |Batch {
                     mesh,
                     surface,
                     data,
                 }| {
                    (mesh.clone(), surface.color.clone(), data.len() / INSTANCE)
                },
            )
            .collect();
        assert_eq!(
            kinds,
            vec![
                (cube.clone(), None, 2),
                (ball, None, 1),
                (cube, Some(bricks), 1),
            ],
            "the see-through one is drawn apart"
        );

        // Each instance: its matrix, column after column, then its color, if it's lit, how many
        // times its texture repeats, and its highlights: none, of the shininess there is at first.
        let second_cube = &batches[0].data[INSTANCE..];
        assert_eq!(second_cube[12], 3.0, "the translation, in the last column");
        assert_eq!(
            &second_cube[16..],
            &[0.5, 0.25, 1.0, 1.0, 0.0, 2.0, 4.0, 0.0, 0.0, 0.0, 32.0, 0.0]
        );
        let shiny = super::batches(&[instance(
            &Mesh::cube(),
            0.0,
            Material::lit(Color::WHITE).specular(Color::rgb(0.2, 0.2, 0.2), 64.0),
        )]);
        assert_eq!(
            &shiny[0].data[20..],
            &[1.0, 1.0, 1.0, 0.2, 0.2, 0.2, 64.0, 0.0]
        );
    }

    #[test]
    fn see_through_instances_are_drawn_the_farthest_first() {
        let (cube, ball) = (Mesh::cube(), Mesh::sphere(8));
        let glass = || Material::color(Color::rgba(0.2, 0.4, 1.0, 0.5));
        let frosted = Texture::url("/frosted.png");
        // Looking along -X from far right: the higher x, the nearer.
        let camera = Camera::default().looking_at(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);

        let (meshes, runs) = see_through(
            &[
                instance(&cube, 3.0, glass()),
                instance(&cube, -2.0, glass()),
                instance(&ball, 0.0, glass()),
                instance(&cube, -1.0, glass()),
                instance(&cube, 1.0, glass().texture(frosted.clone())),
                instance(&cube, 9.0, violet()),
            ],
            &camera,
        );

        assert_eq!(meshes.len(), 2);
        assert_eq!((meshes[0].0 == cube, meshes[1].0 == ball), (true, true));
        let xs: Vec<f32> = meshes[0]
            .1
            .chunks(INSTANCE)
            .map(|instance| instance[12])
            .collect();
        assert_eq!(xs, vec![-2.0, -1.0, 1.0, 3.0], "the cubes, farthest first");
        assert_eq!(
            runs,
            vec![
                Run {
                    mesh: 0,
                    surface: Surface::default(),
                    first: 0,
                    count: 2
                },
                Run {
                    mesh: 1,
                    surface: Surface::default(),
                    first: 0,
                    count: 1
                },
                Run {
                    mesh: 0,
                    surface: Surface {
                        color: Some(frosted),
                        ..Surface::default()
                    },
                    first: 2,
                    count: 1
                },
                Run {
                    mesh: 0,
                    surface: Surface::default(),
                    first: 3,
                    count: 1
                },
            ]
        );
    }

    #[test]
    fn instances_of_other_maps_are_drawn_apart_and_each_has_the_depth_of_its_height_map() {
        let wall = Mesh::cube();
        let (bricks, bumps, heights) = (
            Texture::url("/bricks.png"),
            Texture::url("/bumps.png"),
            Texture::url("/heights.png"),
        );
        let lit = Material::lit(Color::WHITE).texture(bricks.clone());
        let at = |x: f32, material: Material| Instance {
            object: Object::new(wall.clone()),
            transform: Transform::from_translation(Vec3::new(x, 0.0, 0.0)),
            material,
        };

        let batches = batches(&[
            at(0.0, lit.clone()),
            at(1.0, lit.clone().normal_map(bumps.clone())),
            at(
                2.0,
                lit.clone()
                    .normal_map(bumps.clone())
                    .height_map(heights.clone(), 0.1),
            ),
            at(
                3.0,
                lit.clone()
                    .normal_map(bumps.clone())
                    .height_map(heights, 0.05),
            ),
            // A depth without a height map is none
            at(4.0, lit.normal_map(bumps)),
        ]);

        let counts: Vec<usize> = batches
            .iter()
            .map(|batch| batch.data.len() / INSTANCE)
            .collect();
        assert_eq!(counts, vec![1, 2, 2]);
        let depths: Vec<f32> = batches[2]
            .data
            .chunks(INSTANCE)
            .map(|instance| instance[INSTANCE - 1])
            .collect();
        assert_eq!(depths, vec![0.1, 0.05]);
        assert_eq!(batches[1].data[INSTANCE - 1], 0.0);
        assert_eq!(batches[0].surface.color, Some(bricks));
    }

    #[test]
    fn the_images_from_urls_are_counted_and_not_the_pixels() {
        let arrivals = [
            Arrival::Pixels,
            Arrival::Coming,
            Arrival::Came,
            Arrival::Came,
            Arrival::Failed,
        ];
        let counted = loading(arrivals.into_iter());
        assert_eq!(
            counted,
            Loading {
                loaded: 2,
                failed: 1,
                all: 4
            }
        );
        assert!(!counted.is_done(), "one is coming");
        assert!(loading([Arrival::Pixels].into_iter()).is_done());
    }
}
