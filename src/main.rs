#[macro_use]
extern crate derive_new;
use bevy::winit::WinitWindows;
use bevy::{
    prelude::*,
    render::texture::{AddressMode, FilterMode},
};
use bevy_egui::EguiPlugin;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;
use plugins::*;
use utility::setup_test_logger;

#[cfg(not(target_arch = "wasm32"))]
use bevy_inspector_egui::WorldInspectorPlugin;

use log::{debug, error, info, trace, warn}; //IMPORTANT or you won't get any output during tests!

mod genetics_simulator;
mod plugins;
mod utility;
mod vehicle;
mod vehicle_states;

fn main() {
    setup_test_logger();

    let mut app = App::build();
    app.insert_resource(Msaa { samples: 4 }); //TODO disable Msaa in wasm?
    app.insert_resource(WindowDescriptor {
        title: "Vehicle Evolver Deluxe".to_string(),
        width: if cfg!(target_arch = "wasm32") {
            1600.
        } else {
            1920.
        },
        height: if cfg!(target_arch = "wasm32") {
            900.
        } else {
            1080.
        },
        ..Default::default()
    });

    //TODO change this to true if you wanna disable the log plugin from bevy
    //however that means bevy's internals will no longer log stuff
    let disable_bevy_logger = false;
    if disable_bevy_logger {
        app.add_plugins_with(DefaultPlugins, |group| {
            group.disable::<bevy::log::LogPlugin>()
        });
    } else {
        app.add_plugins(DefaultPlugins);
    }

    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let show_debug_inspector = false; //Disabled because it's really slow
        if show_debug_inspector {
            app.add_plugin(WorldInspectorPlugin::new());
        }
    }

    app.add_plugin(ShapePlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(camera::CameraPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(vehicle_manager::VehicleSpawnerPlugin)
        .add_plugin(genetics::GeneticsPlugin)
        .add_plugin(RapierRenderPlugin)
        .add_plugin(terrain_mesh::TerrainMeshPlugin)
        .add_plugin(background::BackgroundPlugin)
        .add_plugin(spawn_indicator::SpawnIndicatorPlugin)
        .add_startup_system(setup.system())
        .add_system(fix_textures.system())
        .add_startup_system(hide_loading_text.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut cam = OrthographicCameraBundle::new_2d();
    cam.transform = Transform::from_translation(Vec3::new(0., 0., 900.));
    cam.transform.scale *= 1.5;
    commands.spawn_bundle(cam);

    commands.spawn_scene(asset_server.load("models/TerrainRoad.glb#Scene0"));
}

#[allow(dead_code)]
fn hide_loading_text(#[cfg(target_arch = "wasm32")] windows: Res<WinitWindows>) {
    //WASM only code
    #[cfg(target_arch = "wasm32")]
    {
        //see https://rustwasm.github.io/wasm-bindgen/examples/dom.html
        let (_, window) = windows.windows.iter().next().expect("no windows are open");
        use winit::platform::web::WindowExtWebSys;
        let canvas = window.canvas();
        let parent = canvas.parent_node().expect("canvas has no parent");
        let document = parent.owner_document().expect("canvas' parent has no doc");

        if let Some(loading) = document.get_element_by_id("loading") {
            loading.remove();
        } else {
            warn!("can't find loading element");
        }
        return;
    }
}

fn fix_textures(
    mut events: EventReader<AssetEvent<Texture>>,
    asset_server: Res<AssetServer>,
    mut assets: ResMut<Assets<Texture>>,
) {
    for event in events.iter() {
        if let AssetEvent::Created { handle } = event {
            if let Some(tex) = assets.get_mut(handle) {
                tex.sampler.min_filter = FilterMode::Linear;
                tex.sampler.mag_filter = FilterMode::Linear;

                let bg_tex = asset_server.load("textures/bg.png");

                if *handle == bg_tex {
                    //TODO this doesn't work in WASM
                    tex.sampler.address_mode_u = AddressMode::Repeat; //Background repeats horizontally
                }
            } else {
                warn!("couldn't find texture to fix, did you load it?");
            }
        }
    }
}

//Setup logger, this runs only when testing
#[cfg(test)]
#[ctor::ctor]
fn init() {
    crate::utility::setup_test_logger();
}
