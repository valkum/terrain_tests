//! Demonstrates how to use the fly camera
#[macro_use]
extern crate amethyst_derive;
#[macro_use]
extern crate log;




use amethyst::{
    assets::{
        AssetLoaderSystemData, Completion, Handle, Loader, PrefabData,
        PrefabLoader, PrefabLoaderSystem, ProgressCounter,
    },
    controls::{
        ArcBallControlBundle, ArcBallControlTag, ControlTagPrefab, FlyControlBundle, FlyControlTag,
    },
    core::{
        math::{Unit, UnitQuaternion, Vector3},
        transform::{Transform, TransformBundle},
        Time,
    },
    ecs::prelude::*,
    ecs::Resources,
    input::{is_key_down, InputBundle},
    prelude::*,
    utils::{application_root_dir, scene::BasicScenePrefab},
    window::{EventsLoopSystem, ScreenDimensions, WindowSystem},
    winit::{EventsLoop, VirtualKeyCode, Window},
};
use amethyst_rendy::{
    camera::{ActiveCamera, Camera, Projection},
    formats::texture::ImageFormat,
    light::{DirectionalLight, Light},
    palette::{Srgb},
    rendy::{
        factory::Factory,
        graph::GraphBuilder,
        hal::Backend,
        texture::{
            image::{ImageTextureConfig},
        },
    },
    system::{GraphCreator, RendererSystem},
    types::{DefaultBackend, Texture},
};
use amethyst_terrain::*;
use std::marker::PhantomData;
use std::sync::Arc;
// use gfx_core::format::ChannelType;

#[derive(Default)]
struct Example<B: Backend> {
    progress: Option<ProgressCounter>,
    _p: PhantomData<B>,
    assets: Option<(Handle<Texture<B>>, Handle<Texture<B>>, Handle<Texture<B>>)>,
    // heightmap: Option<Handle<Texture>>,
    // albedo: Option<Handle<Texture>>,
    // normalmap: Option<Handle<Texture>>,
}
impl<B: Backend> Example<B> {
    pub fn new() -> Self {
        Self {
            progress: None,
            assets: None,
            _p: PhantomData,
        }
    }
}

struct Orbit {
    axis: Unit<Vector3<f32>>,
    time_scale: f32,
    center: Vector3<f32>,
    radius: f32,
    height: f32,
}

impl Component for Orbit {
    type Storage = DenseVecStorage<Self>;
}

struct OrbitSystem;

impl<'a> System<'a> for OrbitSystem {
    type SystemData = (
        Read<'a, Time>,
        ReadStorage<'a, Orbit>,
        WriteStorage<'a, Transform>,
    );

    fn run(&mut self, (time, orbits, mut transforms): Self::SystemData) {
        for (orbit, transform) in (&orbits, &mut transforms).join() {
            let angle = time.absolute_time_seconds() as f32 * orbit.time_scale;
            let cross = orbit.axis.cross(&Vector3::z()).normalize() * orbit.radius;
            let rot = UnitQuaternion::from_axis_angle(&orbit.axis, angle);
            let final_pos = (rot * cross) + orbit.center;
            transform.set_translation(final_pos);
            transform.set_translation_y(orbit.height);
            transform.face_towards(orbit.center, [0., 1., 0.].into());
        }
    }
}

struct CameraCorrectionSystem {
    last_aspect: f32,
}

impl CameraCorrectionSystem {
    pub fn new() -> Self {
        Self { last_aspect: 0.0 }
    }
}

impl<'a> System<'a> for CameraCorrectionSystem {
    type SystemData = (
        ReadExpect<'a, ScreenDimensions>,
        ReadExpect<'a, ActiveCamera>,
        WriteStorage<'a, Camera>,
    );

    fn run(&mut self, (dimensions, active_cam, mut cameras): Self::SystemData) {
        let current_aspect = dimensions.aspect_ratio();

        if current_aspect != self.last_aspect {
            self.last_aspect = current_aspect;

            let camera = cameras.get_mut(active_cam.entity).unwrap();
            *camera = Camera::from(Projection::perspective(
                current_aspect,
                std::f32::consts::FRAC_PI_3,
                0.1,
                1000.0,
            ));
        }
    }
}

// #[derive(Deserialize, Serialize, PrefabData)]
// #[serde(default)]
// #[serde(deny_unknown_fields)]
// pub struct BasicTerrainScenePrefab<V, M = ObjFormat>
// where
//     M: Format<Mesh> + Clone,
//     M::Options: DeserializeOwned + Serialize + Clone,
//     V: From<InternalShape> + Into<MeshData>,
// {
//     graphics: Option<GraphicsPrefab<V, M, TextureFormat>>,
//     transform: Option<Transform>,
//     light: Option<LightPrefab>,
//     camera: Option<CameraPrefab>,
//     control_tag: Option<ControlTagPrefab>,
//     terrain: Option<TerrainPrefab<TextureFormat>>
// }
// impl<V, M> Default for BasicTerrainScenePrefab<V, M>
// where
//     M: Format<Mesh> + Clone,
//     M::Options: DeserializeOwned + Serialize + Clone,
//     V: From<InternalShape> + Into<MeshData>,
// {
//     fn default() -> Self {
//         BasicTerrainScenePrefab {
//             graphics: None,
//             transform: None,
//             light: None,
//             camera: None,
//             control_tag: None,
//             terrain: None,
//         }
//     }
// }
impl<B: Backend> SimpleState for Example<B> {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let StateData { world, .. } = data;
        world.add_resource(TerrainConfig {
            view_mode: TerrainViewMode::Wireframe,
            ..Default::default()
        });
        world.register::<Terrain<B>>();
        world.register::<Transform>();
        world.register::<Light>();
        world.register::<Orbit>();

        // let prefab_handle = data.world.exec(
        //     |loader: PrefabLoader<'_, BasicTerrainScenePrefab<Vec<PosNormTex>>>| {
        //         loader.load("prefab/test.ron", RonFormat, (), ())
        //     },
        // );
        // data.world
        //     .create_entity()
        //     .named("Test")
        //     .with(prefab_handle)
        //     .build();

        self.progress = Some(ProgressCounter::new());

        let (heightmap, normalmap, albedo) =
            world.exec(|loader: AssetLoaderSystemData<'_, Texture<B>>| {
                // use palette::gradient;

                // .with_format(SurfaceType::R16)
                (
                    loader.load(
                        "texture/test4_1024x1024.png",
                        ImageFormat,
                        ImageTextureConfig::unorm(),
                        self.progress.as_mut().unwrap(),
                    ),
                    // loader.load_from_data(
                    //     load_from_linear_rgba(Gradient::new(vec![
                    //             LinSrgb::new(1.0, 0.1, 0.1),
                    //             LinSrgb::new(0.1, 0.1, 1.0),
                    //             LinSrgb::new(0.1, 1.0, 0.1),
                    //     ]).into(),
                    //     (),
                    // ),
                    // loader.load_from_data(
                    //     load_from_linear_rgba(LinSrgb::new(1.0, 1.0, 1.0).into()),
                    //     (),
                    // ),
                    loader.load(
                        "texture/test4_1024x1024_normal.png",
                        ImageFormat,
                        ImageTextureConfig::unorm(),
                        self.progress.as_mut().unwrap(),
                    ),
                    loader.load(
                        "texture/test4_1024x1024_albedo.png",
                        ImageFormat,
                        ImageTextureConfig::default(),
                        self.progress.as_mut().unwrap(),
                    ),
                )
            });
        self.assets = Some((heightmap, normalmap, albedo));

        // Configure width of lines. Optional step
        // data.world.add_resource(DebugLinesParams {
        //     line_width: 1.0 / 50.0,
        // });
        // let mut debug_lines_component = DebugLinesComponent::new().with_capacity(100);

        // let width: u32 = 200;
        // let main_color = [0.4, 0.4, 0.4, 1.0].into();
        // let primary_color = [0., 1.0, 0., 1.0].into();
        // // Center
        // debug_lines_component.add_direction(
        //     [-500., 0., 0.].into(),
        //     [1000., 0., 0.].into(),
        //     [1., 0., 0., 1.0].into(),
        // );
        // debug_lines_component.add_direction(
        //     [0., -500., 0.].into(),
        //     [0., 1000., 0.].into(),
        //     [1., 0., 0., 1.0].into(),
        // );
        // debug_lines_component.add_direction(
        //     [0., 0., -500.].into(),
        //     [0., 0., 1000.].into(),
        //     [1., 0., 0., 1.0].into(),
        // );

        // // Grid lines in X-axis
        // for x in 0..=width {
        //     let (x, width) = (x as f32, width as f32);

        //     let position = Point3::new(x, 0.0, 0.);

        //     let direction = Vector3::new(0.0, 0.0, width);
        //     debug_lines_component.add_direction(position, direction, primary_color);
        //     let direction = Vector3::new(0.0, 0.0, -width);
        //     debug_lines_component.add_direction(position, direction, main_color);

        //     let position = Point3::new(-x, 0.0, 0.);

        //     let direction = Vector3::new(0.0, 0.0, width);
        //     debug_lines_component.add_direction(position, direction, main_color);
        //     let direction = Vector3::new(0.0, 0.0, -width);
        //     debug_lines_component.add_direction(position, direction, main_color);

        //     let position = Point3::new(0., 0.0, x);
        //     let direction = Vector3::new(width, 0.0, 0.0);
        //     debug_lines_component.add_direction(position, direction, primary_color);
        //     let direction = Vector3::new(-width, 0.0, 0.0);
        //     debug_lines_component.add_direction(position, direction, main_color);

        //     let position = Point3::new(0.0, 0.0, -x);
        //     let direction = Vector3::new(width, 0.0, 0.0);
        //     debug_lines_component.add_direction(position, direction, main_color);
        //     let direction = Vector3::new(-width, 0.0, 0.0);
        //     debug_lines_component.add_direction(position, direction, main_color);
        // }
        // data.world.register::<DebugLinesComponent>();
        // data.world
        //     .create_entity()
        //     .with(debug_lines_component)
        //     .build();

        println!("Create lights");
        let dlight: Light = DirectionalLight {
            direction: [512.0, 0., 512.0].into(),
            color: Srgb::new(
                0.78823529411764705882352941176471,
                0.88235294117647058823529411764706,
                1.0,
            ),
            ..DirectionalLight::default()
        }
        .into();

        let mut dlight_trans = Transform::default();
        dlight_trans.set_translation_xyz(0.0, 512.0, 0.0);

        world
            .create_entity()
            .with(dlight)
            .with(dlight_trans)
            .build();

        let mut transform = Transform::default();
        transform.set_translation_xyz(512.0, 50.0, 512.0);

        let _center_entity = world.create_entity().with(transform).build();
        let center = Vector3::new(512.0, 50., 512.);

        let mut pos = Transform::default();
        pos.set_translation_xyz(212., 200., 512.);

        let camera = world
            .create_entity()
            .with(Camera::from(Projection::perspective(
                1.3,
                std::f32::consts::FRAC_PI_3,
                0.1,
                1000.0,
            )))
            .with(pos)
            // .with(FlyControlTag)
            // .with(ArcBallControlTag {target: center_entity, distance: 100.})
            .with(Orbit {
                axis: Unit::new_normalize(Vector3::y()),
                time_scale: 0.5,
                center,
                radius: 300.,
                height: 150.,
            })
            .build();

        world.add_resource(ActiveCamera { entity: camera })
    }
    fn update(&mut self, data: &mut StateData<GameData>) -> SimpleTrans {
        if let Some(progress) = &self.progress {
            match progress.complete() {
                Completion::Complete => {
                    dbg!("Loading Complete");

                    if let Some((heightmap, normalmap, albedo)) = self.assets.clone() {
                        let terrain = Terrain {
                            size: 1024,
                            max_level: 5,
                            heightmap: heightmap,
                            normal: normalmap,
                            albedo: albedo,
                            heightmap_offset: 0.,
                            heightmap_scale: 150.,
                        };
                        let pos = Transform::default();
                        let terrain_entity =
                            data.world.create_entity().with(terrain).with(pos).build();
                        data.world.add_resource(ActiveTerrain {
                            entity: terrain_entity,
                        });
                    }
                    self.progress = None;

                    Trans::None
                }
                Completion::Failed => {
                    dbg!("Loading failed");
                    Trans::None
                }
                _ => Trans::None,
            }
        } else {
            Trans::None
        }
    }

    fn handle_event(
        &mut self,
        data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
        if let StateEvent::Window(event) = event {
            if is_key_down(&event, VirtualKeyCode::Escape) {
                Trans::Quit
            } else if is_key_down(&event, VirtualKeyCode::R) {
                data.world.exec(
                    |(mut transforms, camera): (WriteStorage<Transform>, ReadStorage<Camera>)| {
                        for (transform, _camera) in (&mut transforms, &camera).join() {
                            let _curr = transform.translation().clone();
                            transform
                                .set_translation_xyz(512., 512., 512.)
                                .face_towards(
                                    Vector3::new(512., 0., 512.),
                                    Vector3::new(1., 0., 0.),
                                );
                        }
                    },
                );
                Trans::None
            } else if is_key_down(&event, VirtualKeyCode::F11)
                || is_key_down(&event, VirtualKeyCode::Return)
                    && (is_key_down(&event, VirtualKeyCode::LAlt)
                        || is_key_down(&event, VirtualKeyCode::RAlt))
            {
                debug!("Toggle Fullscreen");
                data.world
                    .exec(|mut screen_dimensions: (WriteExpect<ScreenDimensions>)| {
                        screen_dimensions.toggle_maximized()
                    });
                Trans::None
            // } else if is_key_down(&event, VirtualKeyCode::Z) {
            //     data.world.exec(
            //         |(mut transforms, camera): (WriteStorage<Transform>, ReadStorage<Camera>)| {

            //             for (transform, _camera) in (&mut transforms, &camera).join() {
            //                 transform
            //                     .set_translation_xyz(0., 200., 0.);
            //             }
            //         },
            //     );
            //     Trans::None
            } else if is_key_down(&event, VirtualKeyCode::F) {
                data.world.exec(|mut terrain_config: Write<TerrainConfig>| {
                    terrain_config.view_mode = match terrain_config.view_mode {
                        TerrainViewMode::Wireframe => TerrainViewMode::Color,
                        TerrainViewMode::Color => TerrainViewMode::LOD,
                        TerrainViewMode::LOD => TerrainViewMode::Wireframe,
                    };
                    debug!("Switching to {:?}", &terrain_config.view_mode);
                });
                Trans::None
            } else {
                Trans::None
            }
        } else {
            Trans::None
        }
    }
}

fn main() -> amethyst::Result<()> {
    // amethyst::Logger::from_config(amethyst::LoggerConfig{level_filter: log::LevelFilter::Trace, ..Default::default()})
    //     .level_for("gfx_device_vulkan", log::LevelFilter::Trace)
    //     .level_for("amethyst_assets", log::LevelFilter::Warn)
    //     .start();

    amethyst::Logger::from_config(amethyst::LoggerConfig {
        log_file: Some("rendy_example.log".into()),
        // level_filter: log::LevelFilter::Trace,
        level_filter: log::LevelFilter::Error,
        ..Default::default()
    })
    .level_for("amethyst_utils::fps_counter", log::LevelFilter::Debug)
    .level_for("gfx_backend_vulkan", log::LevelFilter::Warn)
    // .level_for("rendy_factory", log::LevelFilter::Trace)
    // .level_for("rendy_resource", log::LevelFilter::Trace)
    // .level_for("rendy_descriptor", log::LevelFilter::Trace)
    .start();

    let app_root = application_root_dir()?;
    println!("Application Root: {:?}", app_root);

    let resources = app_root.join("resources");
    let display_config = resources.join("display.ron");

    // let config = DisplayConfig::load(&display_config);

    let key_bindings_path = resources.join("input.ron");

    // let pipe = Pipeline::build().with_stage(
    //     Stage::with_backbuffer()
    //         .clear_target([0.0, 0.0, 0.0, 1.0], 1.0)
    //         .with_pass(DrawTerrain::new())
    //         .with_pass(DrawShaded::<PosNormTex>::new())
    //         .with_pass(DrawDebugLines::<PosColorNorm>::new())
    //         .with_pass(DrawSkybox::new()),
    // );
    let event_loop = EventsLoop::new();

    let game_data = GameDataBuilder::default()
        // .with(
        //     PrefabLoaderSystem::<BasicScenePrefab<Vec<PosNormTex>>>::default(),
        //     "prefab",
        //     &[],
        // )
        // .with(AutoFovSystem, "auto_fov", &["prefab"]) // This makes the system adjust the camera right after it has been loaded (in the same frame), preventing any flickering
        // .with(ShowFovSystem, "show_fov", &["auto_fov"])
        // .with(TerrainSystem::default(), "terrain_system", &["prefab"])
        .with(OrbitSystem, "orbit", &[])
        .with(CameraCorrectionSystem::new(), "cam", &[])
        // .with_bundle(ArcBallControlBundle::<String, String>::new())?
        .with_bundle(TransformBundle::new().with_dep(&["orbit"]))?
        // .with_bundle(FPSCounterBundle::default())?
        .with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?,
        )?
        .with_thread_local(WindowSystem::from_config_path(&event_loop, display_config))
        .with_thread_local(EventsLoopSystem::new(event_loop))
        .with_thread_local(RendererSystem::<DefaultBackend, _>::new(ExampleGraph::new()));

    let mut game = Application::build(resources, Example::<DefaultBackend>::new())
        .expect("Failed to initialize")
        .build(game_data)?;

    game.run();

    Ok(())
}

struct ExampleGraph {
    last_dimensions: Option<ScreenDimensions>,
    dirty: bool,
}

impl ExampleGraph {
    pub fn new() -> Self {
        Self {
            last_dimensions: None,
            dirty: true,
        }
    }
}

impl<B: Backend> GraphCreator<B> for ExampleGraph {
    fn rebuild(&mut self, res: &Resources) -> bool {
        // Rebuild when dimensions change, but wait until at least two frames have the same.
        let new_dimensions = res.try_fetch::<ScreenDimensions>();
        use std::ops::Deref;
        if self.last_dimensions.as_ref() != new_dimensions.as_ref().map(|d| d.deref()) {
            self.dirty = true;
            self.last_dimensions = new_dimensions.map(|d| d.clone());
            return false;
        }
        return self.dirty;
    }

    fn builder(&mut self, factory: &mut Factory<B>, res: &Resources) -> GraphBuilder<B, Resources> {
        self.dirty = false;

        let window = <ReadExpect<'_, Arc<Window>>>::fetch(res);
        use amethyst_rendy::{
            rendy::{
                graph::{
                    present::PresentNode,
                    render::{RenderGroupBuilder, SimpleGraphicsPipeline},
                    GraphBuilder,
                },
                hal::{
                    command::{ClearDepthStencil, ClearValue},
                    format::Format,
                },
            },
        };

        let surface = factory.create_surface(window.clone());

        let mut graph_builder = GraphBuilder::new();

        let color = graph_builder.create_image(
            surface.kind(),
            1,
            factory.get_surface_format(&surface),
            Some(ClearValue::Color([0.34, 0.36, 0.52, 1.0].into())),
        );

        let depth = graph_builder.create_image(
            surface.kind(),
            1,
            Format::D32Sfloat,
            Some(ClearValue::DepthStencil(ClearDepthStencil(1.0, 0))),
        );

        let pass = graph_builder.add_node(
            DrawTerrain::builder()
                .into_subpass()
                .with_color(color)
                .with_depth_stencil(depth)
                .into_pass(),
        );

        let present_builder = PresentNode::builder(factory, surface, color).with_dependency(pass);

        graph_builder.add_node(present_builder);

        graph_builder
    }
}
