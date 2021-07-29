use crate::{
    genetics_simulator::{GenerationalStatistics, GeneticsSimulator},
    plugins::vehicle_manager::{BlockComponent, SpawnTimerState, VehicleIDs},
    vehicle::Vehicle,
    vehicle_states::VehicleID,
};
use crate::{
    utility::invlerp,
    vehicle_states::{VehicleStates, VehicleStatus},
};
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Align, Pos2, ScrollArea, Ui},
    EguiContext,
};
use bevy_inspector_egui::Inspectable;
use bevy_inspector_egui::InspectorPlugin;
use dashmap::DashMap;
use egui::{Color32, Label};
use std::sync::Arc;

use log::{debug, error, info, trace, warn}; //IMPORTANT or you won't get any output during tests!

pub struct GeneticsPlugin;
impl Plugin for GeneticsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<GeneticsGuiState>();

        app.add_startup_system(setup_genetics.exclusive_system());
        app.add_system(calculate_fitness_of_current_vehicles.system());
        app.add_system(make_gui.system());
        app.add_system(evolve_if_finished.system());
        app.add_plugin(InspectorPlugin::<SimulationParams>::new());
    }
}

#[derive(Inspectable)]
pub struct SimulationParams {
    #[inspectable(min = 1, max = 30)]
    pub max_simultaneous_vehicles: u32,

    #[inspectable(min = 2, max = 24)]
    pub tournament_k: u32,

    #[inspectable(min = 0, max = 20)]
    pub mutation_amount: u32, //NOTE: keep mutation amount very low (<3)

    #[inspectable(min = 4., max = 60.)]
    pub max_generation_duration: f32, //Seconds

    #[inspectable(min = 0., max = 0.9)]
    pub unhovered_alpha: f32,

    pub camera_lock: bool,
    pub place_only_best_vehicle: bool,
    pub show_green_screen: bool,
}

impl Default for SimulationParams {
    fn default() -> Self {
        SimulationParams {
            max_simultaneous_vehicles: if cfg!(target_arch = "wasm32") { 8 } else { 30 }, //WASM is slow so run only 8 at once
            tournament_k: 10,
            mutation_amount: 1,
            max_generation_duration: 24.0,
            unhovered_alpha: 0.1,
            camera_lock: true,
            place_only_best_vehicle: false,
            show_green_screen: false,
        }
    }
}

pub type GlobalFitnessMap = Arc<DashMap<Vehicle, i64>>;

#[derive(Default)]
pub struct GeneticsGuiState {
    pub hovered_id: Option<VehicleID>,
}

fn make_gui(
    egui_context: ResMut<EguiContext>,
    vehicle_states: Res<VehicleStates>,
    sim: NonSend<GeneticsSimulator>,
    spawn_state: Res<SpawnTimerState>,
    mut gui_state: ResMut<GeneticsGuiState>,
) {
    let gradient = colorous::WARM;

    let fitness_to_color = |fitness: f64| {
        let t = invlerp(0., 9000., fitness as f32).clamp(0., 1.);
        let col = gradient.eval_continuous(t as f64);
        Color32::from_rgb(col.r, col.g, col.b)
    };

    gui_state.hovered_id = None;

    let generations_changed = true; //TODO only scroll when a new entry appears in the list, not every frame

    egui::Window::new("Genetics GUI")
        .default_pos(Pos2::new(20., 300.))
        .show(egui_context.ctx(), |ui| {
            ui.style_mut().body_text_style = egui::TextStyle::Monospace;

            let stats = sim.get_generational_statistics();

            let scroll_area = ScrollArea::from_max_height(100.0);

            scroll_area.show(ui, |ui| {
                ui.vertical(|ui| {
                    for (i, stat) in stats.iter().enumerate() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("Generation {:#3}. Avg=", i + 1));
                            ui.colored_label(
                                fitness_to_color(stat.avg_fitness),
                                format!("{:#5}", stat.avg_fitness.round()),
                            );
                            ui.label(" Max=");
                            ui.colored_label(
                                fitness_to_color(stat.max_fitness),
                                format!("{:#5}", stat.max_fitness.round()),
                            );
                        });
                    }
                });

                if generations_changed {
                    ui.scroll_to_cursor(Align::BOTTOM);
                }
            });
            ui.separator();
            make_fitness_plot(ui, stats);
            ui.separator();

            let progress_bar_len = 18;
            ui.label(format!(
                "Time: {:>4.1}/{:.1} {}",
                spawn_state.timer.elapsed().as_secs_f32(),
                spawn_state.timer.duration().as_secs_f32(),
                (0..progress_bar_len)
                    .map(|i| {
                        let percent = i as f32 / progress_bar_len as f32;
                        if percent < spawn_state.timer.percent() {
                            '◼'
                        } else {
                            '◻'
                        }
                    })
                    .collect::<String>()
            ));

            ui.separator();

            ui.label("Population:");

            for (i, state) in vehicle_states.get_vehicle_states().iter().enumerate() {
                let mut l = Label::new(format!("{:02}. {}", i + 1, state));

                if state.status != VehicleStatus::Pending {
                    l = l.text_color(fitness_to_color(state.fitness as f64));
                }
                ui.add(l).on_hover_ui(|ui: &mut Ui| {
                    ui.heading("Vehicle:");

                    if state.status == VehicleStatus::Running {
                        gui_state.hovered_id = Some(VehicleID(i));
                    }

                    ui.monospace(format!("{}", state.vehicle));
                });
            }
        });
}

fn make_fitness_plot(ui: &mut Ui, gen: &[GenerationalStatistics]) {
    use egui::plot::{Curve, Plot, Value};

    let avg_iter = gen
        .iter()
        .enumerate()
        .map(|(i, stats)| Value::new(i as f64, stats.avg_fitness));
    let avg_curve = Curve::from_values_iter(avg_iter).name("Average fitness");

    let max_iter = gen
        .iter()
        .enumerate()
        .map(|(i, stats)| Value::new(i as f64, stats.max_fitness));
    let max_curve = Curve::from_values_iter(max_iter).name("Max fitness");

    ui.add(
        Plot::new("Avg Fitness Plot")
            .curve(avg_curve)
            .curve(max_curve)
            .include_x(0)
            .include_x(60)
            .include_y(0)
            .include_y(14900)
            .view_aspect(4.0)
            .show_legend(true),
    );
}

fn calculate_fitness_of_current_vehicles(
    query: Query<(&Transform, &BlockComponent)>,
    mut vehicle_states: ResMut<VehicleStates>,
    vehicle_ids: Res<VehicleIDs>,
) {
    let max_diff = 1000; //How far min/max can be apart in X coordinates before we start punishment

    if !vehicle_ids.is_empty() {
        for id in vehicle_ids.iter() {
            let blocks = query
                .iter()
                .filter(|(_, block_comp)| block_comp.belongs_to == *id)
                .map(|(transform, _)| transform.translation.x.round() as i64)
                .collect::<Vec<_>>();
            //Need to collect here since you can't use the same iterator twice

            let len = blocks.len();

            let mut fitness = if len == 0 {
                0.0
            } else {
                let sum = blocks.iter().sum::<i64>();
                sum as f64 / (len as f64)
            };

            let mut fitness_punishment_multiplier = 1.0;

            let mut fell_apart = false;
            let (min, max) = (blocks.iter().min(), blocks.iter().max());
            if let (Some(min), Some(max)) = (min, max) {
                let diff = (max - min).abs();
                if diff > max_diff {
                    fitness_punishment_multiplier *= 0.1;
                    fell_apart = true;
                }
            }

            fitness *= fitness_punishment_multiplier;

            let fitness = fitness.round() as i64;

            vehicle_states.set_fitness(*id, fitness, fell_apart);
        }
    } else {
        warn!("can't update fitness (no active vehicles)");
    }
}

fn setup_genetics(world: &mut World) {
    let (sim, population, map) = initialize_vehicle_sim();
    let states = VehicleStates::from(population);
    world.insert_resource(states);
    world.insert_resource(sim);
    world.insert_resource(map);
}

fn initialize_vehicle_sim() -> (GeneticsSimulator, Vec<Vehicle>, GlobalFitnessMap) {
    let vehicles_sim = GeneticsSimulator::new(24);
    let initial_population = vehicles_sim
        .get_population()
        .clone()
        .into_iter()
        .map(|(v, _)| v)
        .collect();
    let fitness_map = Arc::new(DashMap::default());

    (vehicles_sim, initial_population, fitness_map)
}
fn evolve_if_finished(
    mut sim: ResMut<GeneticsSimulator>,
    mut vehicle_states: ResMut<VehicleStates>,
    map: Res<GlobalFitnessMap>,
    params: Res<SimulationParams>,
) {
    if !vehicle_states.all_done() {
        return;
    }

    info!("Evolving...");
    sim.fill_in_fitness(&map);
    sim.step(&params);

    if params.place_only_best_vehicle {
        let best = map.iter().max_by_key(|x| *x.value()).unwrap();
        info!(
            "Replacing all vehicles with best vehicle with fitness {}: \n{}",
            best.value(),
            best.key()
        );

        let pop_size = sim.get_population().len();
        sim.overwrite_population(vec![best.key().clone(); pop_size]); //Make x copies of the best vehicle
    }

    info!("Simulation stepped");
    let vehicles = sim.get_population_vehicles();

    //Then, reset all vehicle states
    *vehicle_states = VehicleStates::from(vehicles);
}
