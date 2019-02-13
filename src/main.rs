//! Demonstrates how to use the fly camera
#[macro_use]
extern crate amethyst_derive;
#[macro_use]
extern crate log;

use crate::{
    component::{ActiveClipmap, Clipmap},
    renderer::DrawClipmap,
    system::ClipmapSystem,
};
use amethyst::{
    assets::{PrefabLoader, PrefabLoaderSystem, RonFormat},
    controls::FlyControlBundle,
    core::{
        nalgebra::{Point3, Vector3},
        transform::{Transform, TransformBundle},
    },
    ecs::prelude::*,
    input::{is_key_down, InputBundle},
    prelude::*,
    renderer::*,
    utils::{application_root_dir, scene::BasicScenePrefab},
    winit::VirtualKeyCode,
    Error, Logger,
};

mod component;
mod renderer;
mod system;

struct Example;

impl SimpleState for Example {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let prefab_handle = data.world.exec(
            |loader: PrefabLoader<'_, BasicScenePrefab<Vec<PosNormTex>>>| {
                loader.load("prefab/test.ron", RonFormat, (), ())
            },
        );
        data.world.register::<Transform>();
        data.world
            .create_entity()
            .named("Test")
            .with(prefab_handle)
            .build();

        let mut clipmap_transform = Transform::default();
        clipmap_transform.pitch_local(1.5708);
        let clipmap_entity = data
            .world
            .create_entity()
            .named("Clipmap")
            .with(Clipmap::new(127))
            .with(clipmap_transform)
            .build();
        data.world.add_resource(ActiveClipmap {
            entity: Some(clipmap_entity),
        });

        // Configure width of lines. Optional step
        data.world.add_resource(DebugLinesParams {
            line_width: 1.0 / 50.0,
        });
        let mut debug_lines_component = DebugLinesComponent::new().with_capacity(100);

        let width: u32 = 200;
        let main_color = [0.4, 0.4, 0.4, 1.0].into();
        let primary_color = [0., 1.0, 0., 1.0].into();
        // Center
        debug_lines_component.add_direction(
            [-500., 0., 0.].into(),
            [1000., 0., 0.].into(),
            [1., 0., 0., 1.0].into(),
        );
        debug_lines_component.add_direction(
            [0., -500., 0.].into(),
            [0., 1000., 0.].into(),
            [1., 0., 0., 1.0].into(),
        );
        debug_lines_component.add_direction(
            [0., 0., -500.].into(),
            [0., 0., 1000.].into(),
            [1., 0., 0., 1.0].into(),
        );

        // Grid lines in X-axis
        for x in 0..=width {
            let (x, width) = (x as f32, width as f32);

            let position = Point3::new(x, 0.0, 0.);

            let direction = Vector3::new(0.0, 0.0, width);
            debug_lines_component.add_direction(position, direction, primary_color);
            let direction = Vector3::new(0.0, 0.0, -width);
            debug_lines_component.add_direction(position, direction, main_color);

            let position = Point3::new(-x, 0.0, 0.);

            let direction = Vector3::new(0.0, 0.0, width);
            debug_lines_component.add_direction(position, direction, main_color);
            let direction = Vector3::new(0.0, 0.0, -width);
            debug_lines_component.add_direction(position, direction, main_color);

            let position = Point3::new(0., 0.0, x);
            let direction = Vector3::new(width, 0.0, 0.0);
            debug_lines_component.add_direction(position, direction, primary_color);
            let direction = Vector3::new(-width, 0.0, 0.0);
            debug_lines_component.add_direction(position, direction, main_color);

            let position = Point3::new(0.0, 0.0, -x);
            let direction = Vector3::new(width, 0.0, 0.0);
            debug_lines_component.add_direction(position, direction, main_color);
            let direction = Vector3::new(-width, 0.0, 0.0);
            debug_lines_component.add_direction(position, direction, main_color);
        }
        data.world.register::<DebugLinesComponent>();
        data.world
            .create_entity()
            .with(debug_lines_component)
            .build();
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
                            transform
                                .set_position(Vector3::new(0., 15., 10.))
                                .face_towards(Vector3::new(0., 5., 0.), Vector3::new(0., 1., 0.));
                        }
                    },
                );
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
    amethyst::Logger::from_config(Default::default())
        .level_for("gfx_device_gl", log::LevelFilter::Warn)
        .level_for("amethyst_assets", log::LevelFilter::Warn)
        .start();
    // amethyst::Logger::from_config(Default::default()).start();

    let app_root = application_root_dir()?;
    println!("Application Root: {:?}", app_root);

    let resources = app_root.join("resources");
    let display_config = resources.join("display.ron");

    let config = DisplayConfig::load(&display_config);

    let key_bindings_path = resources.join("input.ron");

    let pipe = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target([0.0235, 0.8, 0.8, 1.0], 1.0)
            .with_pass(DrawClipmap::new())
            .with_pass(DrawShaded::<PosNormTex>::new())
            .with_pass(DrawDebugLines::<PosColorNorm>::new()),
    );

    let game_data = GameDataBuilder::default()
        .with(
            PrefabLoaderSystem::<BasicScenePrefab<Vec<PosNormTex>>>::default(),
            "prefab",
            &[],
        )
        // .with(AutoFovSystem, "auto_fov", &["prefab"]) // This makes the system adjust the camera right after it has been loaded (in the same frame), preventing any flickering
        // .with(ShowFovSystem, "show_fov", &["auto_fov"])
        .with(ClipmapSystem::default(), "clipmap_system", &["prefab"])
        .with_bundle(
            FlyControlBundle::<String, String>::new(
                Some(String::from("move_x")),
                Some(String::from("move_y")),
                Some(String::from("move_z")),
            )
            .with_sensitivity(0.1, 0.1)
            .with_speed(10.),
        )?
        .with_bundle(TransformBundle::new().with_dep(&["fly_movement"]))?
        .with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?,
        )?
        .with_bundle(RenderBundle::new(pipe, Some(config)))?;

    let mut game = Application::new(resources, Example, game_data)?;

    game.run();

    Ok(())
}
