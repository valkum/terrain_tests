//! Terrain pass
//!
#[allow(unused_imports)]
use amethyst::{
    assets::{AssetStorage, Handle},
    core::{
        math as na,
        math::base::coordinates::XYZW,
        math::Vector3,
        math::Vector4,
        ecs::{Entity, Join, Read, ReadExpect, ReadStorage, Resources, SystemData},
        transform::GlobalTransform,
    },
    error::Error,
    // renderer::{
    //     build_mesh_with_combo, get_camera,
    //     pipe::{
    //         pass::{Pass, PassData},
    //         DepthMode, Effect, NewEffect,
    //     },
    //     ScreenDimensions,
    //     AmbientColor, Attributes,
    //     ComboMeshCreator, Encoder, Factory, Light, Mesh, MeshCreator, PosTex, Position, Rgba,
    //     Separate, Shape, TexCoord, Texture, VertexFormat,
    // },
    window::ScreenDimensions,
};
use amethyst_rendy::{
    camera::{ActiveCamera, Camera},
    mtl::{Material, MaterialDefaults},
    light::Light,
    types::{Mesh, Texture},
    visibility::Visibility,
    resources::{AmbientColor},
    palette,
    pass::util
};
use glsl_layout::AsStd140;

use std::io::Write;
use amethyst_rendy::rendy::{
    command::{QueueId, RenderPassEncoder},
    factory::Factory,
    graph::{
        render::{Layout, SetLayout, PrepareResult, SimpleGraphicsPipeline, SimpleGraphicsPipelineDesc},
        GraphContext, NodeBuffer, NodeImage,
    },
    hal::{
        self,
        device::Device,
        format::Format,
        pso::{
            self,
            BlendState, ColorBlendDesc, ColorMask, DepthStencilDesc, Descriptor, DescriptorPool,
            DescriptorRangeDesc, DescriptorSetLayoutBinding, DescriptorSetWrite, DescriptorType,
            ElemStride, Element, EntryPoint, GraphicsShaderSet, VertexInputRate, ShaderStageFlags,
            Specialization, DescriptorPoolCreateFlags
        },
        Backend,
        Primitive,
    },
    mesh::{AsVertex, PosNormTex, MeshBuilder, VertexFormat},
    resource::{DescriptorSet, DescriptorSetLayout, Escape, Handle as RendyHandle},
    shader::Shader,
};
use shred_derive::SystemData;

use super::{TerrainConfig, TerrainViewMode};
use cnquadtree::{TerrainQuadtree, TerrainQuadtreeNode, Direction};
use crate::{
    component::{ActiveTerrain, Terrain}
};

use smallvec::{smallvec, SmallVec};



/// Draw mesh without lighting
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawTerrainDesc();

impl DrawTerrainDesc {
    /// Create instance of `DrawFlat` pass
    pub fn new() -> Self {
        Default::default()
    }
}

const MAX_POINT_LIGHTS: usize = 128;
const MAX_DIR_LIGHTS: usize = 16;
const MAX_SPOT_LIGHTS: usize = 128;


impl<B: Backend> SimpleGraphicsPipelineDesc<B, Resources> for DrawTerrainDesc {
    type Pipeline = DrawTerrain<B>;

    fn vertices(&self) -> Vec<(Vec<Element<Format>>, ElemStride, VertexInputRate,)> {
        log::trace!("Set vertex format");
        vec![
            PosNormTex::VERTEX.gfx_vertex_input_desc(VertexInputRate::Vertex),
            pod::InstancedPatchArgs::VERTEX.gfx_vertex_input_desc(VertexInputRate::Instance(1)),
        ]
    }

    fn layout(&self) -> Layout {
        let mut sets = Vec::with_capacity(4);

        // Set 0 - vertex args
        sets.push(SetLayout {
            bindings: vec![
                // VertexArgs
                DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: DescriptorType::UniformBuffer,
                    count: 1,
                    stage_flags: ShaderStageFlags::GRAPHICS,
                    immutable_samplers: false,
                },
                // // TessArgs
                // DescriptorSetLayoutBinding {
                //     binding: 1,
                //     ty: DescriptorType::UniformBuffer,
                //     count: 1,
                //     stage_flags: ShaderStageFlags::GRAPHICS,
                //     immutable_samplers: false,
                // },
            ]
        });

        // Set 1 - heightmap, normal, albedo
        let mut bindings = Vec::with_capacity(3);
        for i in 0..3 {
            bindings.push(
                DescriptorSetLayoutBinding {
                    binding: i,
                    ty: DescriptorType::CombinedImageSampler,
                    count: 1,
                    stage_flags: ShaderStageFlags::GRAPHICS,
                    immutable_samplers: false,
                }
            );
        }
        sets.push(SetLayout { bindings });

        
        // Set 2 - environment
        let mut bindings = Vec::with_capacity(4);
        for i in 0..4 {
            bindings.push(DescriptorSetLayoutBinding {
                binding: i,
                ty: DescriptorType::UniformBuffer,
                count: 1,
                stage_flags: ShaderStageFlags::FRAGMENT,
                immutable_samplers: false,
            })
        }
        sets.push(SetLayout { bindings });


        Layout {
            sets,
            push_constants: Vec::new(),
        }
    }

    fn load_shader_set<'a>(
        &self,
        storage: &'a mut Vec<B::ShaderModule>,
        factory: &mut Factory<B>,
        _aux: &Resources,
    ) -> GraphicsShaderSet<'a, B> {
        log::trace!("Create shader");
        storage.clear();

        log::trace!("Loading shader module '{:#?}'", *super::TERRAIN_VERTEX);
        storage.push(unsafe { super::TERRAIN_VERTEX.module(factory).unwrap() });

        log::trace!("Loading shader module '{:#?}'", *super::TERRAIN_FRAGMEN);
        storage.push(unsafe { super::TERRAIN_FRAGMEN.module(factory).unwrap()});
        
        log::trace!("Loading shader module '{:#?}'", *super::TERRAIN_CONTROL);
        storage.push(unsafe { super::TERRAIN_CONTROL.module(factory).unwrap()});

        log::trace!("Loading shader module '{:#?}'", *super::TERRAIN_EVAL);
        storage.push(unsafe { super::TERRAIN_EVAL.module(factory).unwrap()});

        log::trace!("Loading shader module '{:#?}'", *super::TERRAIN_GEOM);
        storage.push(unsafe { super::TERRAIN_GEOM.module(factory).unwrap()});


        GraphicsShaderSet {
            vertex: EntryPoint {
                entry: "main",
                module: &storage[0],
                specialization: Specialization::default(),
            },
            fragment: Some(EntryPoint {
                entry: "main",
                module: &storage[1],
                specialization: Specialization::default(),
            }),
            hull: Some(EntryPoint {
                entry: "main",
                module: &storage[2],
                specialization: Specialization::default(),
            }),
            domain: Some(EntryPoint {
                entry: "main",
                module: &storage[3],
                specialization: Specialization::default(),
            }),
            geometry: Some(EntryPoint {
                entry: "main",
                module: &storage[4],
                specialization: Specialization::default(),
            }),
        }
    }

    fn input_assembler(&self) -> amethyst_rendy::rendy::hal::pso::InputAssemblerDesc {
        amethyst_rendy::rendy::hal::pso::InputAssemblerDesc {
            primitive: amethyst_rendy::rendy::hal::Primitive::PatchList(4),
            primitive_restart: amethyst_rendy::rendy::hal::pso::PrimitiveRestart::Disabled
        }
    }

    fn build<'a>(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        _resources: &Resources,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
        set_layouts: &[RendyHandle<DescriptorSetLayout<B>>],
    ) -> Result<Self::Pipeline, failure::Error> {
        use amethyst_rendy::rendy::hal::PhysicalDevice;
        let limits = factory.physical().limits();


        let verts = vec![
            PosNormTex {
                position: [-1.0, 0.0, -1.0].into(),
                normal: [0.0, 1.0, 0.0].into(),
                tex_coord: [0.0, 1.0].into()
            },
            PosNormTex {
                position: [1.0, 0.0, -1.0].into(),
                normal: [0.0, 1.0, 0.0].into(),
                tex_coord: [1.0, 1.0].into()
            },
            PosNormTex {
                position: [1.0, 0.0, 1.0].into(),
                normal: [0.0, 1.0, 0.0].into(),
                tex_coord: [1.0, 0.0].into()
            },
            PosNormTex {
                position: [-1.0, 0.0, 1.0].into(),
                normal: [0.0, 1.0, 0.0].into(),
                tex_coord: [0.0, 0.0].into()
            },
        ];
        let basic_mesh = MeshBuilder::new().with_vertices(verts).with_prim_type(Primitive::PatchList(4)).build(queue, factory)?;

        Ok(DrawTerrain {
            per_image: Vec::with_capacity(4),
            terrain_data: TerrainData{desc_set: SmallVec::<[Escape<DescriptorSet<B>>; 3]>::new()},
            ubo_offset_align: limits.min_uniform_buffer_offset_alignment,
            basic_mesh
        })
    }
}



/// Draw a terrain
#[derive(Debug)]
pub struct DrawTerrain<B: Backend> {
    per_image: Vec<PerImage<B>>,
    // materials_data: FnvHashMap<u32, MaterialData<B>>,
    terrain_data: TerrainData<B>,
    ubo_offset_align: u64,
    basic_mesh: amethyst_rendy::rendy::mesh::Mesh<B>,
}


#[derive(Debug)]
struct PerImage<B: Backend> {
    num_patches: usize,
    environment_buffer: Option<Escape<amethyst_rendy::rendy::resource::Buffer<B>>>,
    patch_buffer: Option<Escape<amethyst_rendy::rendy::resource::Buffer<B>>>,
    // material_buffer: Option<amethyst_rendy::rendy::resource::Buffer<B>>,
    terrain_buffer: Option<Escape<amethyst_rendy::rendy::resource::Buffer<B>>>,
    environment_set: Option<Escape<DescriptorSet<B>>>,
    objects_set: Option<Escape<DescriptorSet<B>>>,
}

impl<B: Backend> PerImage<B> {
    fn new() -> Self {
        Self {
            num_patches: 0,
            environment_buffer: None,
            patch_buffer: None,
            terrain_buffer: None,
            environment_set: None,
            objects_set: None,
        }
    }
}

#[derive(Default, Debug)]
struct TerrainData<B: Backend> {
    // usually given material will have just one mesh
    // batches: SmallVec<[InstancedBatchData; 1]>,
    desc_set: SmallVec<[Escape<DescriptorSet<B>>; 3]>,
}

// #[derive(Debug)]
// struct InstancedBatchData {
//     mesh_id: u32,
//     models: SmallVec<[Transform; 4]>,
// }

#[derive(Debug)]
struct ObjectData<B: Backend> {
    desc_set: DescriptorSet<B>,
}

impl<B: Backend> DrawTerrain<B> {
    #[inline]
    fn texture_descriptor<'a>(
        handle: &Handle<Texture<B>>,
        fallback: &Handle<Texture<B>>,
        storage: &'a AssetStorage<Texture<B>>,
    ) -> pso::Descriptor<'a, B> {
        let Texture(texture) = storage
            .get(handle)
            .or_else(|| storage.get(fallback))
            .unwrap();
        pso::Descriptor::CombinedImageSampler(
            texture.view().raw(),
            hal::image::Layout::ShaderReadOnlyOptimal,
            texture.sampler().raw(),
        )
    }

    #[inline]
    fn desc_write<'a>(
        set: &'a B::DescriptorSet,
        binding: u32,
        descriptor: pso::Descriptor<'a, B>,
    ) -> pso::DescriptorSetWrite<'a, B, Option<pso::Descriptor<'a, B>>> {
        pso::DescriptorSetWrite {
            set,
            binding,
            array_offset: 0,
            descriptors: Some(descriptor),
        }
    }
}




#[derive(SystemData)]
struct TerrainPassData<'a, B: Backend> {
    // entities: Entities<'a>,
    active_camera: Option<Read<'a, ActiveCamera>>,
    cameras: ReadStorage<'a, Camera>,
    // screen_dimensions: ReadExpect<'a, ScreenDimensions>,
    // ambient_color: Option<Read<'a, AmbientColor>>,
    global_transforms: ReadStorage<'a, GlobalTransform>,
    lights: ReadStorage<'a, Light>,
    materials: ReadStorage<'a, Handle<Material<B>>>,
    active_terrain: Option<Read<'a, ActiveTerrain>>,
    terrains: ReadStorage<'a, Terrain<B>>,
    terrain_config: ReadExpect<'a, TerrainConfig>,
    // mesh: ReadExpect<'a, TerrainMesh>,
    texture_storage: Read<'a, AssetStorage<Texture<B>>>,
    material_defaults: ReadExpect<'a, MaterialDefaults<B>>,
    // tint: ReadStorage<'a, Rgba>,
}


impl<B: Backend> SimpleGraphicsPipeline<B, Resources> for DrawTerrain<B> {
    type Desc = DrawTerrainDesc;

    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        set_layouts: &[RendyHandle<DescriptorSetLayout<B>>],
        index: usize,
        resources: &Resources,
    ) -> PrepareResult {
        log::trace!("prepare draw");
        let TerrainPassData{
            active_camera,
            cameras,
            // screen_dimensions,
            // ambient_color,
            global_transforms,
            lights,
            active_terrain,
            terrains,
            terrain_config,
            texture_storage,
            material_defaults,
            ..
        } = TerrainPassData::<B>::fetch(resources);


        // ensure resources for this image are available
        let this_image = {
            while self.per_image.len() <= index {
                self.per_image.push(PerImage::new());
            }
            &mut self.per_image[index]
        };

        // Prepare camera
        let defcam = Camera::standard_3d(16., 9.);
        let identity = GlobalTransform::default();
        let (camera_position, camera) = {

            let camera = active_camera
                .and_then(|ac| {
                    cameras.get(ac.entity).map(|camera| {
                        (
                            camera,
                            global_transforms.get(ac.entity).unwrap_or(&identity),
                        )
                    })
                })
                .unwrap_or_else(|| {
                    (&cameras, &global_transforms)
                        .join()
                        .next()
                        .unwrap_or((&defcam, &identity))
                });

            let camera_position = pod_vec((camera.1).0.column(3).xyz());

            // let proj: [[f32; 4]; 4] = camera.0.proj.into();
            // let view: [[f32; 4]; 4] = (*camera.1).0.into();

            (camera_position, camera)
        };

        if let Some((terrain, terrain_transform)) = active_terrain
            .and_then(|at| {


                terrains.get(at.entity).map(|terrain| {
                    (
                        terrain,
                        global_transforms.get(at.entity).unwrap_or(&identity),
                    )
                })
            })
            .or_else(|| {
                (&terrains, &global_transforms)
                    .join()
                    .next()
            })
        {
            let mut wireframe = 0.0;

            match terrain_config.view_mode {
                TerrainViewMode::Wireframe => {
                    wireframe = 1.0;               
                }
                TerrainViewMode::Color => {
                    wireframe = 0.0;
                    // Same as Above but with a color texture.
                }
                TerrainViewMode::LOD => {

                }
            }

    
            let vertex_args = pod::VertexArgs::from(
                camera, 
                &terrain_transform, 
                [terrain.size as f32, terrain.size as f32],
                terrain.heightmap_scale,
                terrain.heightmap_offset,
                wireframe
            );
            // Prepare lights
            {
                let env_buf_size = util::align_size::<pod::Environment>(self.ubo_offset_align, 1);
                let plight_buf_size =
                    util::align_size::<pod::PointLight>(self.ubo_offset_align, MAX_POINT_LIGHTS);
                let dlight_buf_size =
                    util::align_size::<pod::DirectionalLight>(self.ubo_offset_align, MAX_DIR_LIGHTS);
                let slight_buf_size =
                    util::align_size::<pod::SpotLight>(self.ubo_offset_align, MAX_SPOT_LIGHTS);
                let vertex_args_size = util::align_size::<pod::VertexArgs>(self.ubo_offset_align, 1);

                let env_range = Some(0)..Some(env_buf_size);
                let plight_range = env_range.end..env_range.end.map(|e| e + plight_buf_size);
                let dlight_range = plight_range.end..plight_range.end.map(|e| e + dlight_buf_size);
                let slight_range = dlight_range.end..dlight_range.end.map(|e| e + slight_buf_size);
                let vertex_args_range = slight_range.end..slight_range.end.map(|e| e + vertex_args_size);

                if util::ensure_buffer(
                    &factory,
                    &mut this_image.environment_buffer,
                    hal::buffer::Usage::UNIFORM,
                    amethyst_rendy::rendy::memory::Dynamic,
                    vertex_args_range.end.unwrap(),
                )
                .unwrap()
                {
                    let buffer = this_image.environment_buffer.as_ref().unwrap().raw();
                    let env_set = this_image
                        .environment_set
                        .get_or_insert_with(|| factory.create_descriptor_set(set_layouts[2].clone()).unwrap())
                        .raw();
                    let obj_set = this_image
                        .objects_set
                        .get_or_insert_with(|| factory.create_descriptor_set(set_layouts[0].clone()).unwrap())
                        .raw();

                    unsafe {
                        factory.write_descriptor_sets(vec![
                            DescriptorSetWrite {
                                set: env_set,
                                binding: 0,
                                array_offset: 0,
                                descriptors: Some(Descriptor::Buffer(buffer, env_range.clone())),
                            },
                            DescriptorSetWrite {
                                set: env_set,
                                binding: 1,
                                array_offset: 0,
                                descriptors: Some(Descriptor::Buffer(buffer, plight_range.clone())),
                            },
                            DescriptorSetWrite {
                                set: env_set,
                                binding: 2,
                                array_offset: 0,
                                descriptors: Some(Descriptor::Buffer(buffer, dlight_range.clone())),
                            },
                            DescriptorSetWrite {
                                set: env_set,
                                binding: 3,
                                array_offset: 0,
                                descriptors: Some(Descriptor::Buffer(buffer, slight_range.clone())),
                            },
                            DescriptorSetWrite {
                                set: obj_set,
                                binding: 0,
                                array_offset: 0,
                                descriptors: Some(Descriptor::Buffer(buffer, vertex_args_range.clone())),
                            },
                        ]);
                    }
                }

                let point_lights: Vec<_> = (&lights, &global_transforms)
                    .join()
                    .filter_map(|(light, transform)| {
                        if let Light::Point(ref light) = *light {
                            Some(
                                pod::PointLight {
                                    position: pod_vec(transform.0.column(3).xyz()),
                                    color: pod_srgb(light.color),
                                    intensity: light.intensity,
                                }
                                .std140(),
                            )
                        } else {
                            None
                        }
                    })
                    .take(MAX_POINT_LIGHTS)
                    .collect();

                let dir_lights: Vec<_> = lights
                    .join()
                    .filter_map(|light| {
                        if let Light::Directional(ref light) = *light {
                            Some(
                                pod::DirectionalLight {
                                    color: pod_srgb(light.color),
                                    direction: pod_vec(light.direction),
                                }
                                .std140(),
                            )
                        } else {
                            None
                        }
                    })
                    .take(MAX_DIR_LIGHTS)
                    .collect();

                let spot_lights: Vec<_> = (&lights, &global_transforms)
                    .join()
                    .filter_map(|(light, transform)| {
                        if let Light::Spot(ref light) = *light {
                            Some(
                                pod::SpotLight {
                                    position: pod_vec(transform.0.column(3).xyz()),
                                    color: pod_srgb(light.color),
                                    direction: pod_vec(light.direction),
                                    angle: light.angle.cos(),
                                    intensity: light.intensity,
                                    range: light.range,
                                    smoothness: light.smoothness,
                                }
                                .std140(),
                            )
                        } else {
                            None
                        }
                    })
                    .take(MAX_SPOT_LIGHTS)
                    .collect();

                let pod = pod::Environment {
                    ambient_color: [0.0, 0.0, 0.0].into(), // TODO: ambient
                    camera_position,
                    point_light_count: point_lights.len() as _,
                    directional_light_count: dir_lights.len() as _,
                    spot_light_count: spot_lights.len() as _,
                }
                .std140();

                unsafe {
                    let buffer = this_image.environment_buffer.as_mut().unwrap();
                    factory
                        .upload_visible_buffer(buffer, env_range.start.unwrap(), &[pod])
                        .unwrap();
                    if point_lights.len() > 0 {
                        factory
                            .upload_visible_buffer(buffer, plight_range.start.unwrap(), &point_lights)
                            .unwrap();
                    }
                    if dir_lights.len() > 0 {
                        factory
                            .upload_visible_buffer(buffer, dlight_range.start.unwrap(), &dir_lights)
                            .unwrap();
                    }
                    if spot_lights.len() > 0 {
                        factory
                            .upload_visible_buffer(buffer, slight_range.start.unwrap(), &spot_lights)
                            .unwrap();
                    }

                    factory
                        .upload_visible_buffer(buffer, vertex_args_range.start.unwrap(), &[vertex_args])
                        .unwrap();
                }
            }

            while self.terrain_data.desc_set.len() <= index {
                self.terrain_data.desc_set
                    .push(factory.create_descriptor_set(set_layouts[1].clone()).unwrap());
            }

            let set = self.terrain_data.desc_set[index].raw();
            let def = &material_defaults.0;
            let storage = &texture_storage;
            
            let desc_heightmap = Self::texture_descriptor(&terrain.heightmap, &def.normal, storage);
            let desc_normal = Self::texture_descriptor(&terrain.normal, &def.normal, storage);
            let desc_albedo = Self::texture_descriptor(&terrain.albedo, &def.albedo, storage);
           unsafe {
                factory.write_descriptor_sets(vec![
                    // Todo: heightmap default
                    Self::desc_write(set, 0, desc_heightmap),
                    Self::desc_write(set, 1, desc_normal),
                    Self::desc_write(set, 2, desc_albedo),
                ]);
            }


            // Todo: use x,z from camera_position
            // let camera_pos_2d = [512.0, 512.0];
            let camera_pos_2d = [(camera.1).0.column(3).x, (camera.1).0.column(3).z];
            let camera_direction : [f32; 2] = ((camera.1).0 * Vector4::<f32>::new(0., 0., 1., 0.)).xz().into();
            // Todo: base the AABB on the transform
            // let bounds = ncollide2d::bounding_volume::AABB::<f32>::new(
            //     [0.0, 0.0].into(),
            //     [(terrain.size) as f32, (terrain.size) as f32,].into()
            // );


            let mut obj_idx = 0;
            let quadtree = TerrainQuadtree::new(camera_pos_2d, [0.0, 0.0, (terrain.size) as f32, (terrain.size) as f32].into(), terrain.max_level);
            let leaves = quadtree.leaves();
            
            
            let mut patches: Vec<pod::InstancedPatchArgs> = Vec::with_capacity(leaves.len());
            // dbg!(camera_pos_2d);
            // dbg!(camera_direction);
            for patch in leaves {
                if patch.check_visibility( camera_direction, camera_pos_2d) {
                    obj_idx += 1;

                    let mut neighbour_scales = [64, 64, 64, 64];
                    let neighbours = patch.get_neighbours();
                    
                    if neighbours[Direction::North] != TerrainQuadtreeNode::None && quadtree.get_level(neighbours[Direction::North]) < patch.level() {
                        let diff = patch.level() - quadtree.get_level(neighbours[Direction::North]);
                        neighbour_scales[0] = neighbour_scales[0] >> diff;
                    } 
                    if neighbours[Direction::East] != TerrainQuadtreeNode::None && quadtree.get_level(neighbours[Direction::East]) < patch.level() {
                        let diff = patch.level() - quadtree.get_level(neighbours[Direction::East]);
                        neighbour_scales[1] = neighbour_scales[1] >> diff;
                    }
                    if neighbours[Direction::South] != TerrainQuadtreeNode::None && quadtree.get_level(neighbours[Direction::South]) < patch.level() {
                        let diff = patch.level() - quadtree.get_level(neighbours[Direction::South]);
                        neighbour_scales[2] = neighbour_scales[2] >> diff;
                    }
                    if neighbours[Direction::West] != TerrainQuadtreeNode::None && quadtree.get_level(neighbours[Direction::West]) < patch.level() {
                        let diff = patch.level() - quadtree.get_level(neighbours[Direction::West]);
                        neighbour_scales[3] = neighbour_scales[3] >> diff;
                    }
                    patches.push(pod::InstancedPatchArgs{
                        patch_scale: patch.half_extents()[0],
                        patch_origin: [patch.origin()[0], 0., patch.origin()[1]].into(),
                        neighbour_scales: neighbour_scales.into()
                    });
                }
            }
            this_image.num_patches = obj_idx;

            util::ensure_buffer(
                &factory,
                &mut this_image.patch_buffer,
                amethyst_rendy::rendy::hal::buffer::Usage::VERTEX,
                amethyst_rendy::rendy::memory::Dynamic,
                (patches.len() * std::mem::size_of::<pod::InstancedPatchArgs>()) as _,
            )
            .unwrap();

            if let Some(mut buffer) = this_image.patch_buffer.as_mut() {
                unsafe {
                    factory
                        .upload_visible_buffer(&mut buffer, 0, &patches)
                        .unwrap();
                }
            }
        }
        



        PrepareResult::DrawRecord
    }

    fn draw(
        &mut self,
        layout: &B::PipelineLayout,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        aux: &Resources,
    ) {
        let this_image = &self.per_image[index];
        if this_image.num_patches == 0 {
            return;
        }
        // let TerrainPassData {
        //     mesh_storage,
        //     meshes,
        //     ..
        // } = TerrainPassData::<B>::fetch(resources);
        if let Some(objects_set) = this_image.objects_set.as_ref() {

            encoder.bind_graphics_descriptor_sets(
                layout,
                0,
                Some(objects_set.raw()),
                std::iter::empty(),
            );

            encoder.bind_graphics_descriptor_sets(
                layout,
                1,
                Some(self.terrain_data.desc_set[index].raw()),
                std::iter::empty(),
            );

            encoder.bind_graphics_descriptor_sets(
                layout,
                2,
                Some(this_image.environment_set.as_ref().unwrap().raw()),
                std::iter::empty(),
            );

            self.basic_mesh.bind(&[PosNormTex::VERTEX], &mut encoder).unwrap();
            encoder.bind_vertex_buffers(
                1,
                Some((
                    this_image.patch_buffer.as_ref().unwrap().raw(),
                    0,
                )),
            );
            encoder.draw(0..self.basic_mesh.len(), 0..this_image.num_patches as u32);
        }
    }

    fn dispose(self, factory: &mut Factory<B>, _aux: &Resources) {}
}

fn pod_srgb(srgb: palette::Srgb) -> glsl_layout::vec3 {
    let (r, g, b) = srgb.into_components();
    [r, g, b].into()
}

fn pod_vec(vec: amethyst::core::math::Vector3<f32>) -> glsl_layout::vec3 {
    let arr: [f32; 3] = vec.into();
    arr.into()
}

fn byte_size<T>(slice: &[T]) -> usize {
    slice.len() * std::mem::size_of::<T>()
}

mod pod {
    use glsl_layout::*;
    use amethyst_rendy::{camera::Camera, rendy::mesh::{AsVertex, VertexFormat, Attribute}};
    use amethyst::core::transform::GlobalTransform;
    use std::borrow::Cow;

    pub(crate) fn array_size<T: AsStd140>(elems: usize) -> usize
    where
        T::Std140: Sized,
    {
        std::mem::size_of::<T::Std140>() * elems
    }

    #[repr(C, align(16))]
    #[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
    pub(crate) struct InstancedPatchArgs {
        pub patch_scale: float,
        pub patch_origin: vec3,
        pub neighbour_scales: ivec4
    }

    impl AsVertex for InstancedPatchArgs {
        const VERTEX: VertexFormat<'static> = VertexFormat {
            attributes: Cow::Borrowed(&[
                // Patch Scale (4 Byte)
                Attribute {
                    format: amethyst_rendy::rendy::hal::format::Format::Rgba32Sfloat,
                    offset: 0,
                },
                // Patch Origin (12 Byte)
                Attribute {
                    format: amethyst_rendy::rendy::hal::format::Format::Rgba32Sfloat,
                    offset: 4,
                },
                // Neighbour Scale (16 Byte)
                Attribute {
                    format: amethyst_rendy::rendy::hal::format::Format::Rgba32Sint,
                    offset: 16,
                },
            ]),
            stride: 32,
        };
    }

    // For all shader stages (binding = 0)
    #[derive(Clone, Copy, Debug, AsStd140)]
    #[repr(C, align(16))]
    pub(crate) struct VertexArgs {
        proj: mat4,
        view: mat4,
        model: mat4,
        terrain_size: vec2,
        terrain_height_scale: float,
        terrain_height_offset: float,
        wireframe: float
    }

    impl VertexArgs {
        pub fn from(
            camera: (&Camera, &GlobalTransform),
            object: &GlobalTransform,
            terrain_size: [f32; 2],
            terrain_height_scale: f32,
            terrain_height_offset: f32,
            wireframe: f32,
        ) -> Self {
            
            let proj: [[f32; 4]; 4] = camera.0.proj.into();
            let view: [[f32; 4]; 4] = camera.1
                .0
                .try_inverse()
                .expect("Unable to get inverse of camera transform")
                .into();
            let model: [[f32; 4]; 4] = object.0.into();
            VertexArgs {
                proj: proj.into(),
                view: view.into(),
                model: model.into(),
                terrain_size: terrain_size.into(),
                terrain_height_scale: terrain_height_scale.into(),
                terrain_height_offset: terrain_height_offset.into(),
                wireframe: wireframe.into()
            }
            
        }
    }
    // // For Tess shader (binding = 2)
    // #[derive(Clone, Copy, AsStd140)]
    // #[repr(C, align(16))]
    // pub(crate) struct TessArgs {
    //     // Screen dimensions
    //     viewport: vec2,

    //     terrain_height_scale: float,
    //     terrain_height_offset: float,
    //     //   0
    //     // 3 x 1
    //     //   2
    //     neighbour_scales: vec4
    // }
    // impl TessArgs {
    //     pub fn from(
    //         viewport: [f32; 2],
    //         terrain_height_scale: f32,
    //         terrain_height_offset: f32,
    //         neighbour_scales: [f32; 4]
    //     ) -> Self {
    //         TessArgs {
    //             viewport: viewport.into(),
    //             terrain_height_scale: terrain_height_scale.into(),
    //             terrain_height_offset: terrain_height_offset.into(),
    //             neighbour_scales: neighbour_scales.into()
    //         }
    //     }
    // }

    #[derive(Clone, Copy, Debug, AsStd140)]
    pub(crate) struct PointLight {
        pub position: vec3,
        pub color: vec3,
        pub intensity: float,
    }

    #[derive(Clone, Copy, Debug, AsStd140)]
    pub(crate) struct DirectionalLight {
        pub color: vec3,
        pub direction: vec3,
    }

    #[derive(Clone, Copy, Debug, AsStd140)]
    pub(crate) struct SpotLight {
        pub position: vec3,
        pub color: vec3,
        pub direction: vec3,
        pub angle: float,
        pub intensity: float,
        pub range: float,
        pub smoothness: float,
    }

    #[derive(Clone, Copy, Debug, AsStd140)]
    pub(crate) struct Environment {
        pub ambient_color: vec3,
        pub camera_position: vec3,
        pub point_light_count: int,
        pub directional_light_count: int,
        pub spot_light_count: int,
    }
}