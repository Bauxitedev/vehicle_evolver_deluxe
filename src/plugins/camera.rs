use crate::{
    plugins::genetics::SimulationParams,
    plugins::vehicle_manager::BlockComponent,
    utility::window_to_world,
    vehicle_states::{VehicleID, VehicleStates, VehicleStatus},
};
use bevy::{input::mouse::MouseWheel, prelude::*, render::camera::Camera};
use bevy_egui::EguiContext;

use log::{debug, error, info, trace, warn}; //IMPORTANT or you won't get any output during tests!

use super::genetics::GeneticsGuiState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let camera_controls_stage = SystemStage::single_threaded()
            .with_system(camera_controls.system())
            .with_system(camera_lock.system());

        app.add_stage_after(
            CoreStage::Last,
            "CAMERA_CONTROLS_STAGE",
            camera_controls_stage,
        );
    }
}

fn camera_controls(
    mut ev_cursor: EventReader<MouseWheel>,
    windows: Res<Windows>,
    egui_ctx: ResMut<EguiContext>,
    mut query_cam_transform: Query<&mut Transform, With<Camera>>,
) {
    if egui_ctx.ctx().wants_pointer_input() {
        return; //Don't consume input events that egui already consumed
    }

    let window = windows.get_primary().unwrap();
    let mouse_pos = window.cursor_position();
    let cam_transform = *query_cam_transform.iter_mut().next().unwrap();

    for ev in ev_cursor.iter() {
        if ev.y.abs() > 0.0 {
            if let Some(p_window) = mouse_pos {
                let win_size = Vec2::new(window.width(), window.height());
                let mouse_window_ratio = p_window / win_size;
                let mouse_half_ratio = Vec2::new(0.5, 0.5);

                let zoom_fac = if ev.y < 0.0 { 1.2 } else { 0.8 };
                let lerp_factor = 1.0 - (zoom_fac); //Allowed to go outside 0..1

                let new_cam_pos_window_ratio =
                    mouse_half_ratio.lerp(mouse_window_ratio, lerp_factor);
                let new_cam_pos_window = new_cam_pos_window_ratio * win_size;
                let new_cam_pos_world = window_to_world(new_cam_pos_window, window, &cam_transform);

                query_cam_transform.for_each_mut(|mut t| {
                    let scale_z = t.scale.z;

                    t.scale *= zoom_fac;
                    t.scale.z = scale_z;

                    t.translation = new_cam_pos_world;
                });
            }
        }
    }
}

fn camera_lock(
    params: Res<SimulationParams>,
    gui_state: Res<GeneticsGuiState>,
    mut vehicle_states: ResMut<VehicleStates>,
    query_blocks: Query<(&Transform, &BlockComponent)>,
    query_cam_transform: Query<&mut Transform, (With<Camera>, Without<BlockComponent>)>,
    time: Res<Time>,
) {
    //Hide camera lock icon on all states except one
    for state in vehicle_states.get_vehicle_states_mut().iter_mut() {
        state.is_camera_target = false;
    }

    let camera_lerp_speed = 5.0;

    //Lock to fittest vehicle
    let best_vehicle = vehicle_states
        .get_vehicle_states_mut()
        .iter_mut()
        .enumerate()
        .filter(|(_, vehicle)| vehicle.status == VehicleStatus::Running)
        .max_by_key(|(_, vehicle)| vehicle.fitness);

    if let Some((best_vehicle_i, best_vehicle_state)) = best_vehicle {
        best_vehicle_state.is_camera_target = true;

        if !params.camera_lock {
            return;
        }

        let mut target_vehicle_id = VehicleID(best_vehicle_i);

        //If hovering over a vehicle, lock to that vehicle instead of the fittest vehicle
        if let Some(hovered_id) = gui_state.hovered_id {
            target_vehicle_id = hovered_id;
        }

        let blocks = query_blocks
            .iter()
            .filter(|(_, block_comp)| block_comp.belongs_to == target_vehicle_id)
            .map(|(transform, _)| transform.translation.x)
            .collect::<Vec<_>>();

        if blocks.is_empty() {
            warn!("no blocks found whatsoever, can't lock camera");
            return;
        }
        let block_count = blocks.len();
        let avg = blocks.into_iter().sum::<f32>() / block_count as f32;
        let target_x = avg.round() as i64;

        query_cam_transform.for_each_mut(|mut t| {
            let mut target_pos = t.translation;
            target_pos.x = target_x as f32;

            t.translation = t.translation.lerp(
                target_pos,
                (time.delta_seconds() * camera_lerp_speed).clamp(0., 1.),
            );
        });
    } else {
        warn!("no vehicle ids found whatsoever, can't lock camera");
    }
}
