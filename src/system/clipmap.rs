use amethyst::{
    assets::{AssetStorage, Completion, Handle, Loader, PrefabData, ProgressCounter},
    core::{
        specs::{
            Component, Entity, HashMapStorage, Join, Read, ReadExpect, ReadStorage, System, Write,
            WriteStorage,
        },
        transform::GlobalTransform,
    },
    renderer::{
        build_mesh_with_combo, ActiveCamera, Attributes, Camera, ComboMeshCreator, Encoder,
        Factory, FilterMethod, Mesh, MeshCreator, MeshData, PngFormat, PosTex, Position, Rgba,
        SamplerInfo, Separate, Shape, ShapeUpload, SurfaceType, TexCoord, Texture, TextureMetadata,
        VertexFormat, WrapMode,
    },
    Error,
};
use gfx::format::ChannelType;

use crate::{ActiveClipmap, Clipmap};

#[derive(Default)]
pub struct ClipmapSystem {
    progress: Option<ProgressCounter>,
}

impl<'a> System<'a> for ClipmapSystem {
    type SystemData = (
        Read<'a, ActiveClipmap>,
        WriteStorage<'a, Clipmap>,
        ReadExpect<'a, Loader>,
        Read<'a, AssetStorage<Mesh>>,
        Read<'a, AssetStorage<Texture>>,
    );

    fn run(
        &mut self,
        (active, mut clipmaps, loader, mesh_storage, texture_storage): Self::SystemData,
    ) {
        if let Some(active_clipmap) = active.entity {
            let clipmap = clipmaps.get_mut(active_clipmap).unwrap();
            if let Some(progress) = &self.progress {
                match progress.complete() {
                    Completion::Complete => {
                        clipmap.initialized = true;
                        debug!("Clipmap generation completed");
                        self.progress = None;
                    }
                    _ => {
                        dbg!(progress.errors());
                    }
                }
            }

            if !clipmap.initialized && self.progress.is_none() {
                debug!(
                    "Creating clipmap with size {}x{}",
                    clipmap.size, clipmap.size
                );
                self.progress = Some(ProgressCounter::default());
                let block_size = ((clipmap.size + 1) / 4) as usize;
                let one_offset: f32 = ((clipmap.size + 1) / 4) as f32 - 1.;
                let half_offset: f32 = one_offset / 2.;
                let ring_fixup_offset = 1. + half_offset + one_offset;
                // Generate block mesh with m-1 x m-1 faces (ergo m x m vertices) and scale it by m/2.
                let block_mesh_vert = Shape::Plane(Some((block_size - 1, block_size - 1)))
                    .generate_vertices::<ComboMeshCreator>(Some((
                    (block_size - 1) as f32 / 2.,
                    (block_size - 1) as f32 / 2.,
                    0.,
                )));
                let block_mesh_data = ComboMeshCreator::from(block_mesh_vert).into();

                clipmap.block_mesh = Some(loader.load_from_data(
                    block_mesh_data,
                    self.progress.as_mut().unwrap(),
                    &mesh_storage,
                ));

                let interior_mesh_vert =
                    Shape::Plane(Some((2 * (block_size - 1) + 1, 2 * (block_size - 1) + 1)))
                        .generate_vertices::<ComboMeshCreator>(Some((
                        (2 * (block_size - 1) + 1) as f32 / 2.,
                        (2 * (block_size - 1) + 1) as f32 / 2.,
                        0.,
                    )));
                let interior_mesh_data = ComboMeshCreator::from(interior_mesh_vert).into();

                clipmap.interior_mesh = Some(loader.load_from_data(
                    interior_mesh_data,
                    self.progress.as_mut().unwrap(),
                    &mesh_storage,
                ));

                let fixup_mesh_horizontal =
                    Shape::Plane(Some((block_size - 1, 2))).generate_vertices::<ComboMeshCreator>(
                        Some(((block_size - 1) as f32 / 2., 1., 0.)),
                    );
                let fixup_mesh_vertical =
                    Shape::Plane(Some((2, block_size - 1))).generate_vertices::<ComboMeshCreator>(
                        Some((1., (block_size - 1) as f32 / 2., 0.)),
                    );

                let mut fixup_mesh_vert_north: Vec<Separate<Position>> = fixup_mesh_vertical
                    .vertices()
                    .into_iter()
                    .map(|Separate(x)| {
                        Separate::<Position>::new([x[0], x[1] - ring_fixup_offset, x[2]])
                    })
                    .collect();
                let mut fixup_mesh_vert_south: Vec<Separate<Position>> = fixup_mesh_vertical
                    .vertices()
                    .into_iter()
                    .map(|Separate(x)| {
                        Separate::<Position>::new([x[0], x[1] + ring_fixup_offset, x[2]])
                    })
                    .collect();

                let mut fixup_mesh_vert_west: Vec<Separate<Position>> = fixup_mesh_horizontal
                    .vertices()
                    .into_iter()
                    .map(|Separate(x)| {
                        Separate::<Position>::new([x[0] + ring_fixup_offset, x[1], x[2]])
                    })
                    .collect();
                let mut fixup_mesh_vert_east: Vec<Separate<Position>> = fixup_mesh_horizontal
                    .vertices()
                    .into_iter()
                    .map(|Separate(x)| {
                        Separate::<Position>::new([x[0] - ring_fixup_offset, x[1], x[2]])
                    })
                    .collect();

                let mut fixup_mesh_vertices: Vec<Separate<Position>> = Vec::new();
                fixup_mesh_vertices.append(&mut fixup_mesh_vert_north);
                fixup_mesh_vertices.append(&mut fixup_mesh_vert_west);
                fixup_mesh_vertices.append(&mut fixup_mesh_vert_east);
                fixup_mesh_vertices.append(&mut fixup_mesh_vert_south);
                let fixup_mesh_data = ComboMeshCreator::from(ComboMeshCreator::new((
                    fixup_mesh_vertices,
                    None,
                    None,
                    None,
                    None,
                )))
                .into();

                clipmap.ring_fixup_mesh = Some(loader.load_from_data(
                    fixup_mesh_data,
                    self.progress.as_mut().unwrap(),
                    &mesh_storage,
                ));

                // TODO: Generate the remaining 3 shapes. (NorthWest, SouthEast and SouthWest)
                let l_shape_mesh_horizontal = Shape::Plane(Some((2 * (block_size - 1) + 2, 1)))
                    .generate_vertices::<ComboMeshCreator>(Some((
                    (2 * (block_size - 1) + 2) as f32 / 2.,
                    0.5,
                    0.,
                )));
                let l_shape_mesh_vertical = Shape::Plane(Some((1, 2 * (block_size - 1) + 1)))
                    .generate_vertices::<ComboMeshCreator>(Some((
                    0.5,
                    (2 * (block_size - 1) + 1) as f32 / 2.,
                    0.,
                )));

                let mut l_shape_west_verts: Vec<Separate<Position>> = l_shape_mesh_vertical
                    .vertices()
                    .into_iter()
                    .map(|Separate(x)| {
                        Separate::<Position>::new([
                            x[0] + (block_size as f32) - 0.5,
                            x[1] - 0.5,
                            x[2],
                        ])
                    })
                    .collect();
                let mut l_shape_north_east_verts: Vec<Separate<Position>> = l_shape_mesh_horizontal
                    .vertices()
                    .into_iter()
                    .map(|Separate(x)| {
                        Separate::<Position>::new([x[0], x[1] + (block_size as f32) - 0.5, x[2]])
                    })
                    .collect();
                l_shape_north_east_verts.append(&mut l_shape_west_verts);

                let l_shape_mesh_data = ComboMeshCreator::from(ComboMeshCreator::new((
                    l_shape_north_east_verts,
                    None,
                    None,
                    None,
                    None,
                )))
                .into();
                clipmap.l_shape_mesh = Some(loader.load_from_data(
                    l_shape_mesh_data,
                    self.progress.as_mut().unwrap(),
                    &mesh_storage,
                ));

                // let height_metedata = TextureMetadata {
                //     sampler: SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile),
                //     mip_levels: 1,
                //     dynamic: true,
                //     format: SurfaceType::R8_G8_B8_A8,
                //     size: None,
                //     channel: ChannelType::Srgb,
                // };
                // let elevetion_map_handle =  loader.load(
                //     "texture/elevation.png",
                //     PngFormat,
                //     height_metedata,
                //     self.progress.as_mut().unwrap(),
                //     &texture_storage,
                // );
                let elevetion_map_handle = loader.load(
                    "texture/elevation_1.png",
                    PngFormat,
                    TextureMetadata {
                        sampler: SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile),
                        mip_levels: 1,
                        dynamic: true,
                        format: SurfaceType::R8_G8_B8_A8,
                        size: None,
                        channel: ChannelType::Srgb,
                    },
                    self.progress.as_mut().unwrap(),
                    &texture_storage,
                );

                let elevetion_map_handle_2 = loader.load(
                    "texture/elevation_2.png",
                    PngFormat,
                    TextureMetadata {
                        sampler: SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile),
                        mip_levels: 1,
                        dynamic: true,
                        format: SurfaceType::R8_G8_B8_A8,
                        size: None,
                        channel: ChannelType::Srgb,
                    },
                    self.progress.as_mut().unwrap(),
                    &texture_storage,
                );
                clipmap.elevation = Some(vec![elevetion_map_handle, elevetion_map_handle_2]);
                let normal_map_handle = loader.load(
                    "texture/normal.png",
                    PngFormat,
                    TextureMetadata::unorm(),
                    self.progress.as_mut().unwrap(),
                    &texture_storage,
                );
                clipmap.normal = Some(normal_map_handle);

                let z_color_handle = loader.load(
                    "texture/z_color.png",
                    PngFormat,
                    TextureMetadata::srgb(),
                    self.progress.as_mut().unwrap(),
                    &texture_storage,
                );
                clipmap.z_color = Some(z_color_handle);
            }
        }
    }
}
