use amethyst::{
    assets::{AssetStorage, Completion, Handle, Loader, PrefabData, ProgressCounter, Format},
    core::{
        ecs::{
            Component, Entity, HashMapStorage, Join, Read, ReadExpect, ReadStorage, System, Write,
            WriteStorage,
        },
        transform::GlobalTransform,
    },
    error::Error,
    renderer::{
        build_mesh_with_combo,
        pipe::{
            pass::{Pass, PassData},
            DepthMode, Effect, NewEffect,
        },
        ActiveCamera, Attributes, Camera, ComboMeshCreator, Encoder, Factory, Light, Mesh,
        MeshCreator, MeshData, PosTex, Position, Rgba, Separate, Shape, ShapeUpload, TexCoord,
        VertexFormat, TexturePrefab,TextureMetadata, MaterialDefaults
    },
};
use amethyst_rendy::{
    rendy::hal::Backend,
    types::Texture,
    rendy::texture::image::{load_from_image, ImageTextureConfig},
    rendy::texture::TextureBuilder,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, PrefabData)]
#[prefab(Component)]
// TODO: Doc
pub struct Terrain<B: Backend> {
    pub size: u32,
    pub max_level: u8,
    pub heightmap: Handle<Texture<B>>,
    pub normal: Handle<Texture<B>>,
    pub albedo: Handle<Texture<B>>,
    pub heightmap_offset: f32,
    pub heightmap_scale: f32,
}
impl<B: Backend> Component for Terrain<B> {
    type Storage = HashMapStorage<Self>;
}

// #[derive(Deserialize, Serialize)]
// pub struct TerrainPrefab<F> 
// where
//     F: Format<Texture, Options = TextureMetadata>,
// {
//     size: u32,
//     max_level: u8,
//     heightmap: Option<TexturePrefab<F>>,
//     normal: Option<TexturePrefab<F>>,
//     albedo: Option<TexturePrefab<F>>,
// }



// impl<F> Default for TerrainPrefab<F>
// where
//     F: Format<Texture, Options = TextureMetadata>,
// {
//     fn default() -> Self {
//         TerrainPrefab {
//             size: 128,
//             max_level: 4,
//             heightmap: None,
//             normal: None,
//             albedo: None,
//         }
//     }
// }

// fn load_handle<F>(
//     entity: Entity,
//     prefab: &Option<TexturePrefab<F>>,
//     tp_data: &mut <TexturePrefab<F> as PrefabData<'_>>::SystemData,
//     def: &Handle<Texture>,
// ) -> Handle<Texture>
// where
//     F: Format<Texture, Options = TextureMetadata> + Sync + Clone,
// {
//     prefab
//         .as_ref()
//         .and_then(|tp| tp.add_to_entity(entity, tp_data, &[]).ok())
//         .unwrap_or_else(|| def.clone())
// }

// impl<'a, F> PrefabData<'a> for TerrainPrefab<F>
// where
//     F: Format<Texture, Options = TextureMetadata> + Sync + Clone,
// {
//     type SystemData = (
//         WriteStorage<'a, Terrain>,
//         ReadExpect<'a, MaterialDefaults>,
//         <TexturePrefab<F> as PrefabData<'a>>::SystemData,
//     );
//     type Result = ();

//     fn add_to_entity(
//         &self,
//         entity: Entity,
//         system_data: &mut Self::SystemData,
//         _: &[Entity],
//     ) -> Result<(), Error> {
//         let &mut (ref mut terrain, ref mat_default, ref mut tp_data) =
//             system_data;
//         let trn = Terrain {
//             size: self.size,
//             max_level: self.max_level,
//             heightmap: load_handle(entity, &self.heightmap, tp_data, &mat_default.0.ambient_occlusion),
//             normal: load_handle(entity, &self.normal, tp_data, &mat_default.0.normal),
//             albedo: load_handle(entity, &self.albedo, tp_data, &mat_default.0.albedo),
//         };
//         terrain.insert(entity, trn)?;
//         Ok(())
//     }

//     fn load_sub_assets(
//         &mut self,
//         progress: &mut ProgressCounter,
//         system_data: &mut Self::SystemData,
//     ) -> Result<bool, Error> {
//         let &mut (_, _, ref mut tp_data) = system_data;
//         let mut ret = false;
//         if let Some(ref mut texture) = self.heightmap {
//             if texture.load_sub_assets(progress, tp_data)? {
//                 ret = true;
//             }
//         }
//         if let Some(ref mut texture) = self.albedo {
//             if texture.load_sub_assets(progress, tp_data)? {
//                 ret = true;
//             }
//         }
//         if let Some(ref mut texture) = self.normal {
//             if texture.load_sub_assets(progress, tp_data)? {
//                 ret = true;
//             }
//         }
//         Ok(ret)
//     }

// }

/// Active clipmap resource, used by the renderer to choose which camera to get the view matrix from.
/// If no active camera is found, the first camera will be used as a fallback.
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveTerrain {
    /// Camera entity
    pub entity: Entity,
}

/// Active camera prefab
pub struct ActiveTerrainPrefab(usize);

impl<'a> PrefabData<'a> for ActiveTerrainPrefab {
    type SystemData = (Option<Write<'a, ActiveTerrain>>,);
    type Result = ();

    fn add_to_entity(
        &self,
        _entity: Entity,
        system_data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<(), Error> {
        if let Some(ref mut terrain) = system_data.0 {
            terrain.entity = entities[self.0];
        }
        // TODO: if no `ActiveTerrain` insert using `LazyUpdate`, require changes to `specs`
        Ok(())
    }
}
