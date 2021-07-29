use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
use log::Level;

use log::{debug, error, info, trace, warn}; //IMPORTANT or you won't get any output during tests!

//https://stackoverflow.com/a/65633668
pub fn window_to_world(position: Vec2, window: &Window, camera: &Transform) -> Vec3 {
    // Center in screen space
    let norm = Vec3::new(
        position.x - window.width() / 2.,
        position.y - window.height() / 2.,
        0.,
    );

    // Apply camera transform
    *camera * norm
}

pub fn setup_test_logger() {
    //If on WASM, use console_log, else use fern

    #[cfg(target_arch = "wasm32")]
    console_log::init_with_level(Level::Info)
        .unwrap_or_else(|_| println!("couldn't init web logger"));

    #[cfg(not(target_arch = "wasm32"))]
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}:{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.line().map_or("???".into(), |x| x.to_string()),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Warn)
        .level_for("vehicle_evolver_deluxe", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()
        .unwrap();
}

pub fn invlerp(a: f32, b: f32, t: f32) -> f32 {
    (t - a) / (b - a)
}
