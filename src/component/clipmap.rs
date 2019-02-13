use amethyst::{
    assets::{AssetStorage, Completion, Handle, Loader, PrefabData, ProgressCounter},
    core::{
        specs::{
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
        Texture, VertexFormat,
    },
};

type ClipmapMeshHandle = Handle<Mesh>;

#[derive(Clone, PrefabData)]
#[prefab(Component)]
// #[serde(default)]
// TODO: Doc
pub struct Clipmap {
    pub initialized: bool,
    pub block_mesh: Option<ClipmapMeshHandle>,
    pub ring_fixup_mesh: Option<ClipmapMeshHandle>,
    pub l_shape_mesh: Option<ClipmapMeshHandle>,
    pub interior_mesh: Option<ClipmapMeshHandle>,
    pub elevation: Option<Vec<Handle<Texture>>>,
    pub normal: Option<Handle<Texture>>,
    pub z_color: Option<Handle<Texture>>,
    pub size: u32,
    pub texture_size: Option<u32>,
    pub alpha_offset: [f32; 2],
    pub one_over_width: [f32; 2],
}
impl Clipmap {
    /// Creates a new instance with the default values for all fields
    pub fn new(size: u32) -> Self {
        // Check that size is 2^k-1
        assert!((size + 1) & size == 0);
        let transition_width = size as f32 / 10.;

        Clipmap {
            block_mesh: None,
            ring_fixup_mesh: None,
            l_shape_mesh: None,
            interior_mesh: None,
            elevation: None,
            normal: None,
            z_color: None,
            size: size,
            initialized: false,
            texture_size: Some(255),
            // Per forumla this hould be: (n-1)/2-w-1 with w = transition width (n/10)
            alpha_offset: [((size as f32 - 1.) / 2.) - transition_width - 1.; 2],
            // alpha_offset: [transition_width - 1.; 2],
            one_over_width: [1. / transition_width; 2],
        }
    }
}
impl Component for Clipmap {
    type Storage = HashMapStorage<Self>;
}
impl Default for Clipmap {
    fn default() -> Self {
        Clipmap::new(15)
    }
}

/// Active clipmap resource, used by the renderer to choose which camera to get the view matrix from.
/// If no active camera is found, the first camera will be used as a fallback.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ActiveClipmap {
    /// Camera entity
    pub entity: Option<Entity>,
}

/// Active camera prefab
pub struct ActiveClipmapPrefab(usize);

impl<'a> PrefabData<'a> for ActiveClipmapPrefab {
    type SystemData = (Write<'a, ActiveClipmap>,);
    type Result = ();

    fn add_to_entity(
        &self,
        _: Entity,
        system_data: &mut Self::SystemData,
        entities: &[Entity],
    ) -> Result<(), Error> {
        system_data.0.entity = Some(entities[self.0]);
        // TODO: if no `ActiveClipmap` insert using `LazyUpdate`, require changes to `specs`
        Ok(())
    }
}
