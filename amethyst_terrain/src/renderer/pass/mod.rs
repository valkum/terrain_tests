//! Render pass.

// pub use self::quadtree::TerrainQuadtree;
pub use self::terrain::DrawTerrain;

use serde::{Deserialize, Serialize};

use crate::{ActiveTerrain, Terrain};
use amethyst::core::{
    ecs::prelude::{Join, Read, ReadStorage},
    transform::GlobalTransform,
};

use amethyst_rendy::rendy::shader::{ShaderKind, SourceLanguage, StaticShaderInfo};

// mod quadtree;
mod terrain;


lazy_static::lazy_static! {
    static ref TERRAIN_VERTEX: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/shader/vertex/terrain.glsl"),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    );

    static ref TERRAIN_CONTROL: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/shader/tesselation/terrain.tcs.glsl"),
        ShaderKind::TessControl,
        SourceLanguage::GLSL,
        "main",
    );

    static ref TERRAIN_EVAL: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/shader/tesselation/terrain.tes.glsl"),
        ShaderKind::TessEvaluation,
        SourceLanguage::GLSL,
        "main",
    );

    static ref TERRAIN_GEOM: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/shader/geometry/terrain.glsl"),
        ShaderKind::Geometry,
        SourceLanguage::GLSL,
        "main",
    );

    static ref TERRAIN_FRAGMEN: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/shader/fragment/terrain.glsl"),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    );
}

// pub fn get_terrain<'a>(
//     active: Read<'a, ActiveTerrain>,
//     terrains: &'a ReadStorage<'a, Terrain>,
//     globals: &'a ReadStorage<'a, GlobalTransform>,
// ) -> Option<(&'a Terrain, &'a GlobalTransform)> {
//     active
//         .entity
//         .and_then(|entity| {
//             let terrain = terrains.get(entity);
//             let transform = globals.get(entity);
//             terrain.into_iter().zip(transform.into_iter()).next()
//         })
//         .or_else(|| (terrains, globals).join().next())
// }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TerrainViewMode {
    Color, // Normal view.
    Wireframe,
    LOD,
}
impl Default for TerrainViewMode {
    fn default() -> Self {
        TerrainViewMode::Color
    }
}

/// Colors used for the gradient skybox
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerrainConfig {
    /// The color directly above the viewer
    pub view_mode: TerrainViewMode,
}

impl Default for TerrainConfig {
    fn default() -> TerrainConfig {
        TerrainConfig {
            view_mode: Default::default(),
        }
    }
}

