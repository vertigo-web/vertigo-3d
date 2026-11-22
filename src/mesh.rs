use std::{fmt, rc::Rc};

use glam::{Vec2, Vec3};

use crate::pick::bounds;

/// The shape of objects: triangles of corners, each corner with its position and its normal,
/// the way its surface faces. Shared by the objects of the shape, and sent to the browser once
/// for a scene, whatever their number.
///
/// The shapes made here are 1 across, around the origin, and the corners of their triangles go
/// round counter-clockwise seen from outside, as WebGL takes a triangle's front. Their corners
/// have texture coordinates too, where a [`Texture`](crate::Texture) goes on them, upright.
///
/// ```rust
/// use vertigo_3d::Mesh;
///
/// let cube = Mesh::cube();
/// // Objects of the same shape share it, and are drawn together.
/// let same = cube.clone();
/// assert_eq!(cube, same);
/// assert_ne!(cube, Mesh::cube());
/// ```
#[derive(Clone)]
pub struct Mesh {
    data: Rc<MeshData>,
}

#[derive(Clone)]
pub(crate) struct MeshData {
    /// x, y and z of each corner.
    pub positions: Vec<f32>,
    /// x, y and z of each corner's normal.
    pub normals: Vec<f32>,
    /// Where each corner is on a texture: across from the left, and down from the top.
    pub uvs: Vec<f32>,
    /// x, y and z of the way each corner's texture goes right on the surface, across its normal,
    /// and 1 when the texture's up is the normal crossed with it, -1 when the other way: what
    /// a normal map's colors are read along. Worked out from the texture coordinates.
    pub tangents: Vec<f32>,
    /// Corners of the triangles, three a triangle.
    pub indices: Vec<u32>,
    /// The middle and the radius of a sphere around the corners.
    pub bounds: (Vec3, f32),
}

/// Why [`Mesh::from_triangles`] didn't make a mesh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeshError {
    /// Not a normal for each position.
    Normals { positions: usize, normals: usize },
    /// The indices aren't a whole number of triangles.
    Triangles { indices: usize },
    /// An index past the last position.
    Index { index: u32, positions: usize },
    /// Not texture coordinates for each position, see [`Mesh::with_uvs`].
    Uvs { positions: usize, uvs: usize },
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normals { positions, normals } => {
                write!(f, "{normals} normals for {positions} positions")
            }
            Self::Triangles { indices } => write!(f, "{indices} indices aren't whole triangles"),
            Self::Index { index, positions } => {
                write!(f, "index {index} past the last of {positions} positions")
            }
            Self::Uvs { positions, uvs } => {
                write!(f, "{uvs} texture coordinates for {positions} positions")
            }
        }
    }
}

impl std::error::Error for MeshError {}

impl Mesh {
    /// A mesh of corners at `positions`, facing the way of `normals`, one for each position,
    /// made into triangles by `indices`, three a triangle. Their texture coordinates are 0, see
    /// [`Self::with_uvs`].
    ///
    /// ```rust
    /// use vertigo_3d::{Mesh, math::Vec3};
    ///
    /// // A triangle facing the camera, which looks along -Z.
    /// let triangle = Mesh::from_triangles(
    ///     vec![Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, -0.5, 0.0), Vec3::new(0.0, 0.5, 0.0)],
    ///     vec![Vec3::Z; 3],
    ///     vec![0, 1, 2],
    /// );
    /// assert!(triangle.is_ok());
    /// ```
    pub fn from_triangles(
        positions: Vec<Vec3>,
        normals: Vec<Vec3>,
        indices: Vec<u32>,
    ) -> Result<Self, MeshError> {
        if normals.len() != positions.len() {
            return Err(MeshError::Normals {
                positions: positions.len(),
                normals: normals.len(),
            });
        }
        if !indices.len().is_multiple_of(3) {
            return Err(MeshError::Triangles {
                indices: indices.len(),
            });
        }
        if let Some(index) = indices
            .iter()
            .find(|index| **index as usize >= positions.len())
        {
            return Err(MeshError::Index {
                index: *index,
                positions: positions.len(),
            });
        }

        let uvs = vec![Vec2::ZERO; positions.len()];
        Ok(Self::made(&positions, &normals, &uvs, indices))
    }

    /// The mesh with `uvs`, one for each position: where the corner is on a texture, from 0 to 1
    /// across it from the left, and down it from the top. A new mesh, not the same as this one.
    ///
    /// ```rust
    /// use vertigo_3d::{Mesh, math::{Vec2, Vec3}};
    ///
    /// let triangle = Mesh::from_triangles(
    ///     vec![Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, -0.5, 0.0), Vec3::new(0.0, 0.5, 0.0)],
    ///     vec![Vec3::Z; 3],
    ///     vec![0, 1, 2],
    /// )
    /// // The bottom corners of the texture, and the middle of its top.
    /// .and_then(|triangle| {
    ///     triangle.with_uvs(vec![Vec2::new(0.0, 1.0), Vec2::ONE, Vec2::new(0.5, 0.0)])
    /// });
    /// assert!(triangle.is_ok());
    /// ```
    pub fn with_uvs(self, uvs: Vec<Vec2>) -> Result<Self, MeshError> {
        let positions = self.data.positions.len() / 3;
        if uvs.len() != positions {
            return Err(MeshError::Uvs {
                positions,
                uvs: uvs.len(),
            });
        }
        let mut data = MeshData::clone(&self.data);
        data.uvs = uvs.iter().flat_map(|uv| uv.to_array()).collect();
        data.tangents = tangents(
            &vectors(&data.positions),
            &vectors(&data.normals),
            &uvs,
            &data.indices,
        );
        Ok(Self {
            data: Rc::new(data),
        })
    }

    fn made(positions: &[Vec3], normals: &[Vec3], uvs: &[Vec2], indices: Vec<u32>) -> Self {
        let tangents = tangents(positions, normals, uvs, &indices);
        let positions: Vec<f32> = positions
            .iter()
            .flat_map(|position| position.to_array())
            .collect();
        Self {
            data: Rc::new(MeshData {
                bounds: bounds(&positions),
                tangents,
                positions,
                normals: normals
                    .iter()
                    .flat_map(|normal| normal.to_array())
                    .collect(),
                uvs: uvs.iter().flat_map(|uv| uv.to_array()).collect(),
                indices,
            }),
        }
    }

    /// A cube 1 across, its faces each facing their own way, each with all of a texture,
    /// upright on the sides, and the top seen from the front.
    pub fn cube() -> Self {
        // Each face: the way it faces, and its right and up seen from outside, the right turned to
        // the up counter-clockwise.
        let faces = [
            (Vec3::X, Vec3::NEG_Z, Vec3::Y),
            (Vec3::NEG_X, Vec3::Z, Vec3::Y),
            (Vec3::Y, Vec3::X, Vec3::NEG_Z),
            (Vec3::NEG_Y, Vec3::X, Vec3::Z),
            (Vec3::Z, Vec3::X, Vec3::Y),
            (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
        ];

        let (mut positions, mut normals, mut uvs, mut indices) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for (normal, right, up) in faces {
            let first = positions.len() as u32;
            for (a, b) in [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
                positions.push(normal * 0.5 + right * a + up * b);
                normals.push(normal);
                uvs.push(Vec2::new(a + 0.5, 0.5 - b));
            }
            indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
        }

        Self::made(&positions, &normals, &uvs, indices)
    }

    /// A sphere 1 across, of `segments` slices around and half as many rings from pole to
    /// pole, smooth: each corner faces away from the middle. At least 3 segments. A texture goes
    /// round it once, its top at the top pole, as a map of the world on a globe.
    pub fn sphere(segments: u32) -> Self {
        let segments = segments.max(3);
        let rings = (segments / 2).max(2);

        let (mut positions, mut normals, mut uvs, mut indices) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for ring in 0..=rings {
            let theta = std::f32::consts::PI * ring as f32 / rings as f32;
            for segment in 0..=segments {
                let phi = std::f32::consts::TAU * segment as f32 / segments as f32;
                let normal = Vec3::new(
                    theta.sin() * phi.cos(),
                    theta.cos(),
                    theta.sin() * phi.sin(),
                );
                positions.push(normal * 0.5);
                normals.push(normal);
                // Eastwards, as seen from outside.
                uvs.push(Vec2::new(
                    1.0 - segment as f32 / segments as f32,
                    ring as f32 / rings as f32,
                ));
            }
        }
        for ring in 0..rings {
            for segment in 0..segments {
                let a = ring * (segments + 1) + segment;
                let b = a + segments + 1;
                indices.extend([a, a + 1, b, a + 1, b + 1, b]);
            }
        }

        Self::made(&positions, &normals, &uvs, indices)
    }

    /// A square 1 across, lying flat around the origin and facing up, +Y: a floor. A texture's
    /// top is at its far edge, -Z.
    pub fn plane() -> Self {
        let positions = [
            Vec3::new(-0.5, 0.0, -0.5),
            Vec3::new(0.5, 0.0, -0.5),
            Vec3::new(0.5, 0.0, 0.5),
            Vec3::new(-0.5, 0.0, 0.5),
        ];
        let uvs = [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y];
        Self::made(&positions, &[Vec3::Y; 4], &uvs, vec![0, 2, 1, 0, 3, 2])
    }

    /// How many triangles it has.
    pub fn triangles(&self) -> usize {
        self.data.indices.len() / 3
    }

    pub(crate) fn data(&self) -> &MeshData {
        &self.data
    }
}

impl PartialEq for Mesh {
    /// The same mesh, not one alike: objects of one are drawn together.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.data, &other.data)
    }
}

impl fmt::Debug for Mesh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mesh")
            .field("triangles", &self.triangles())
            .finish_non_exhaustive()
    }
}

/// Vectors of three floats in a row.
fn vectors(floats: &[f32]) -> Vec<Vec3> {
    floats
        .as_chunks::<3>()
        .0
        .iter()
        .map(|&xyz| Vec3::from(xyz))
        .collect()
}

/// Of each corner, the way its texture goes right on the surface, across its normal, and whether
/// the texture's up is the normal crossed with it (1) or the other way (-1), see
/// [`MeshData::tangents`]. Summed over the triangles of the corner, so a smooth surface has smooth
/// ones. A corner whose texture doesn't stretch over its triangles - f. ex. of no texture
/// coordinates - gets any way across its normal.
fn tangents(positions: &[Vec3], normals: &[Vec3], uvs: &[Vec2], indices: &[u32]) -> Vec<f32> {
    let mut rights = vec![Vec3::ZERO; positions.len()];
    let mut ups = vec![Vec3::ZERO; positions.len()];

    for &[a, b, c] in indices.as_chunks::<3>().0 {
        let [a, b, c] = [a as usize, b as usize, c as usize];
        let (along, across) = (positions[b] - positions[a], positions[c] - positions[a]);
        let (moved, moved_across) = (uvs[b] - uvs[a], uvs[c] - uvs[a]);
        let area = moved.x * moved_across.y - moved_across.x * moved.y;
        if area.abs() < 1e-12 {
            continue;
        }
        // How far the surface goes for a step right on the texture, and for a step down it.
        let right = (along * moved_across.y - across * moved.y) / area;
        let down = (across * moved.x - along * moved_across.x) / area;
        for corner in [a, b, c] {
            rights[corner] += right;
            ups[corner] -= down;
        }
    }

    normals
        .iter()
        .zip(rights.iter().zip(&ups))
        .flat_map(|(normal, (right, up))| {
            let normal = normal.normalize_or_zero();
            let tangent = (*right - normal * normal.dot(*right)).normalize_or_zero();
            let tangent = if tangent == Vec3::ZERO {
                normal.any_orthonormal_vector()
            } else {
                tangent
            };
            let handedness = if normal.cross(tangent).dot(*up) < 0.0 {
                -1.0
            } else {
                1.0
            };
            [tangent.x, tangent.y, tangent.z, handedness]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use glam::{Vec2, Vec3};

    use super::{Mesh, MeshError};

    /// Each triangle's corners, as positions.
    fn triangles(mesh: &Mesh) -> Vec<[Vec3; 3]> {
        let data = mesh.data();
        let at = |index: u32| Vec3::from_slice(&data.positions[index as usize * 3..][..3]);
        data.indices
            .chunks(3)
            .map(|corners| [at(corners[0]), at(corners[1]), at(corners[2])])
            .collect()
    }

    /// The way a triangle faces, by the order of its corners: counter-clockwise seen from where
    /// it points.
    fn facing([a, b, c]: [Vec3; 3]) -> Vec3 {
        (b - a).cross(c - a)
    }

    #[test]
    fn a_cube_has_its_triangles_facing_out() {
        let cube = Mesh::cube();
        assert_eq!(cube.triangles(), 12);
        for triangle in triangles(&cube) {
            let middle = (triangle[0] + triangle[1] + triangle[2]) / 3.0;
            assert!(facing(triangle).dot(middle) > 0.0, "{triangle:?}");
            assert!(
                triangle
                    .iter()
                    .all(|corner| corner.abs().max_element() == 0.5)
            );
        }
    }

    #[test]
    fn a_sphere_has_its_triangles_facing_out() {
        let sphere = Mesh::sphere(16);
        assert_eq!(sphere.triangles(), 16 * 8 * 2);
        for triangle in triangles(&sphere) {
            let middle = (triangle[0] + triangle[1] + triangle[2]) / 3.0;
            let facing = facing(triangle);
            // At the poles, two corners of a triangle are one point.
            if facing.length() > 1e-6 {
                assert!(facing.dot(middle) > 0.0, "{triangle:?}");
            }
            assert!(
                triangle
                    .iter()
                    .all(|corner| (corner.length() - 0.5).abs() < 1e-5)
            );
        }
        assert_eq!(Mesh::sphere(1).triangles(), Mesh::sphere(3).triangles());
    }

    #[test]
    fn a_plane_faces_up() {
        for triangle in triangles(&Mesh::plane()) {
            assert!(facing(triangle).normalize().abs_diff_eq(Vec3::Y, 1e-6));
        }
    }

    #[test]
    fn a_texture_is_upright_on_the_sides_of_a_cube() {
        let cube = Mesh::cube();
        let data = cube.data();
        let corners = data.positions.len() / 3;
        assert_eq!(data.uvs.len(), corners * 2);
        for corner in 0..corners {
            let position = Vec3::from_slice(&data.positions[corner * 3..][..3]);
            let normal = Vec3::from_slice(&data.normals[corner * 3..][..3]);
            let [u, v] = [data.uvs[corner * 2], data.uvs[corner * 2 + 1]];
            if normal.y == 0.0 {
                // The top of the texture up, its left on the left seen from outside.
                assert_eq!(v, 0.5 - position.y, "{position}");
                let right = Vec3::Y.cross(normal);
                assert_eq!(u, 0.5 + position.dot(right), "{position}");
            }
        }
    }

    #[test]
    fn with_uvs_takes_one_for_each_corner() {
        let triangle = || {
            Mesh::from_triangles(
                vec![Vec3::ZERO, Vec3::X, Vec3::Y],
                vec![Vec3::Z; 3],
                vec![0, 1, 2],
            )
        };
        let Ok(mesh) = triangle().and_then(|mesh| mesh.with_uvs(vec![Vec2::ONE; 3])) else {
            panic!("a mesh with uvs");
        };
        assert_eq!(mesh.data().uvs, vec![1.0; 6]);
        assert_eq!(
            triangle()
                .and_then(|mesh| mesh.with_uvs(vec![Vec2::ONE; 2]))
                .err(),
            Some(MeshError::Uvs {
                positions: 3,
                uvs: 2
            })
        );
    }

    #[test]
    fn a_mesh_of_triangles_is_checked() {
        let three = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
        assert_eq!(
            Mesh::from_triangles(three.clone(), vec![Vec3::Z; 2], vec![0, 1, 2]).err(),
            Some(MeshError::Normals {
                positions: 3,
                normals: 2
            })
        );
        assert_eq!(
            Mesh::from_triangles(three.clone(), vec![Vec3::Z; 3], vec![0, 1]).err(),
            Some(MeshError::Triangles { indices: 2 })
        );
        assert_eq!(
            Mesh::from_triangles(three, vec![Vec3::Z; 3], vec![0, 1, 3]).err(),
            Some(MeshError::Index {
                index: 3,
                positions: 3
            })
        );
    }

    /// Each corner's tangent and handedness.
    fn tangents(mesh: &Mesh) -> Vec<(Vec3, f32)> {
        mesh.data()
            .tangents
            .chunks(4)
            .map(|tangent| (Vec3::from_slice(&tangent[..3]), tangent[3]))
            .collect()
    }

    #[test]
    fn a_textures_right_and_up_go_along_the_faces_of_a_cube_and_a_plane() {
        let cube = Mesh::cube();
        let data = cube.data();
        for (corner, (tangent, handedness)) in tangents(&cube).into_iter().enumerate() {
            let normal = Vec3::from_slice(&data.normals[corner * 3..][..3]);
            // The front, facing +Z: the texture upright, its right +X, its up +Y
            if normal == Vec3::Z {
                assert!(tangent.abs_diff_eq(Vec3::X, 1e-6), "{tangent}");
                assert_eq!(normal.cross(tangent) * handedness, Vec3::Y);
            }
            assert!(tangent.dot(normal).abs() < 1e-6, "across the normal");
        }

        // The floor: its texture's right +X, its top at the far edge, -Z
        for (tangent, handedness) in tangents(&Mesh::plane()) {
            assert!(tangent.abs_diff_eq(Vec3::X, 1e-6), "{tangent}");
            assert_eq!(Vec3::Y.cross(tangent) * handedness, Vec3::NEG_Z);
        }
    }

    #[test]
    fn every_corner_has_a_tangent_across_its_normal_even_without_a_texture() {
        let flat = Mesh::from_triangles(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![Vec3::Z; 3],
            vec![0, 1, 2],
        )
        .expect("a triangle");
        for mesh in [Mesh::sphere(12), flat.clone()] {
            let data = mesh.data();
            for (corner, (tangent, handedness)) in tangents(&mesh).into_iter().enumerate() {
                let normal = Vec3::from_slice(&data.normals[corner * 3..][..3]);
                assert!((tangent.length() - 1.0).abs() < 1e-4, "{tangent}");
                assert!(tangent.dot(normal).abs() < 1e-4);
                assert!(handedness.abs() == 1.0);
            }
        }

        // Texture coordinates given later bring their own
        let textured = flat
            .with_uvs(vec![Vec2::new(0.0, 1.0), Vec2::ONE, Vec2::ZERO])
            .expect("three coordinates");
        for (tangent, _) in tangents(&textured) {
            assert!(tangent.abs_diff_eq(Vec3::X, 1e-6), "{tangent}");
        }
    }
}
