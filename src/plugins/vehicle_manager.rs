use crate::{plugins::genetics::GlobalFitnessMap, vehicle::Vehicle};
use crate::{
    plugins::genetics::SimulationParams,
    vehicle::Block,
    vehicle_states::{VehicleID, VehicleStates},
};
use bevy::prelude::*;
use bevy_rapier2d::physics::TimestepMode;
use bevy_rapier2d::prelude::*;
use ndarray::{Array, Array2};

use log::{debug, error, info, trace, warn};

use super::genetics::GeneticsGuiState; //IMPORTANT or you won't get any output during tests!

pub struct VehicleSpawnerPlugin;

impl Plugin for VehicleSpawnerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        //Warning: race condition here! setup_physics must run BEFORE ALL OTHER STARTUP SYSTEMS

        app.init_resource::<VehicleIDs>();
        app.init_resource::<SpawnTimerState>();

        app.add_startup_system_to_stage(StartupStage::PreStartup, setup_physics.system()); //TODO move this to another system?

        app.add_system(maybe_spawn_vehicle.system());
        app.add_system(hide_unhovered_vehicles.system());
        //note: do NOT add the spawn_vehicle system to CoreStage::First or the physics break entirely
    }
}

fn hide_unhovered_vehicles(
    query: Query<(&Handle<ColorMaterial>, &mut Transform, &BlockComponent)>,
    gui_state: Res<GeneticsGuiState>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    params: Res<SimulationParams>,
) {
    let default_z = 0.0;
    let default_alpha = 1.0;

    if let Some(hovered_id) = gui_state.hovered_id {
        query.for_each_mut(|(handle, mut transform, block_comp)| {
            let hovered = block_comp.belongs_to == hovered_id;

            let alpha = if hovered {
                default_alpha
            } else {
                params.unhovered_alpha
            };

            materials
                .get_mut(handle)
                .expect("material not found")
                .color
                .set_a(alpha);

            transform.translation.z = if hovered { 5. } else { default_z }; //Draw on top of every other vehicle if hovered
        })
    } else {
        query.for_each_mut(|(handle, mut transform, _)| {
            transform.translation.z = default_z; //Reset ordering
            materials
                .get_mut(handle)
                .expect("material not found")
                .color
                .set_a(default_alpha);
        })
    }
}

#[derive(new)]
pub struct BlockComponent {
    pub belongs_to: VehicleID,
}

//TODO move this to its own plugin or main
fn setup_physics(mut configuration: ResMut<RapierConfiguration>) {
    configuration.scale = 100.0; //pixels per meter
    configuration.timestep_mode = TimestepMode::FixedTimestep; //TODO this seems to speed up the simulation when performance is fast

    //(*configuration).physics_pipeline_active = false;
}

pub struct SpawnTimerState {
    pub timer: Timer,
}

impl Default for SpawnTimerState {
    fn default() -> Self {
        SpawnTimerState {
            timer: Timer::from_seconds(0.1, false), //small timer here so we have time to setup VehicleStates
        }
    }
}

pub type VehicleIDs = Vec<VehicleID>;

fn maybe_spawn_vehicle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    configuration: Res<RapierConfiguration>,
    mut spawner_state: ResMut<SpawnTimerState>,
    mut query_blocks: Query<Entity, With<BlockComponent>>,
    time: Res<Time>,
    mut vehicle_states: ResMut<VehicleStates>,
    mut prev_vehicle_ids: ResMut<VehicleIDs>,
    fitness_map: ResMut<GlobalFitnessMap>,
    params: Res<SimulationParams>,
) {
    spawner_state.timer.tick(time.delta());
    if !spawner_state.timer.finished() {
        //Wait for timer to finish
        return;
    }

    //finalize vehicle and remove them from VehicleIDs
    for id in prev_vehicle_ids.drain(..) {
        info!("finalized vehicle {:?}", id);
        let (vehicle, final_fitness) = vehicle_states.finalize_vehicle(id);
        if fitness_map.insert(vehicle, final_fitness).is_some() {
            warn!("vehicle override another's fitness, vehicle may have been simulated twice");
        }
    }

    let popped_vehicles = vehicle_states.pop_vehicles(params.max_simultaneous_vehicles as usize);
    if !popped_vehicles.is_empty() {
        //vehicle spawned, set timer so we wait to simulate it
        spawner_state.timer = Timer::from_seconds(params.max_generation_duration, false);

        //cleanup any previous vehicles
        for e in query_blocks.iter_mut() {
            commands.entity(e).despawn();
        }

        let vehicle_ids = popped_vehicles
            .iter()
            .map(|(_, id, _)| *id)
            .collect::<Vec<_>>();
        commands.insert_resource(vehicle_ids);

        for (vehicle, vehicle_id, color) in popped_vehicles {
            info!("popped and spawned vehicle [id={:?}]:", vehicle_id);

            //let mut rng = rand::thread_rng();

            let entities = setup_panels(
                &vehicle,
                vehicle_id,
                color,
                &configuration,
                &asset_server,
                &mut commands,
                &mut materials,
            );
            setup_joints(&mut commands, entities);
        }
    } else {
        warn!("ran out of vehicles");
    }
}

#[derive(new, Clone)]
struct EntityCell {
    pub ent: Entity,
    pub pos: Vec2,
    pub block_type: Block,
}

fn setup_panels(
    vehicle: &Vehicle,
    vehicle_id: VehicleID,
    color: Color,
    configuration: &RapierConfiguration,
    asset_server: &AssetServer,
    cmd: &mut Commands,
    materials: &mut Assets<ColorMaterial>,
) -> Array2<Option<EntityCell>> {
    let VehicleID(collider_group_index) = vehicle_id;

    assert!(collider_group_index < 32);
    let collider_group = 1 << collider_group_index;

    let mut entities = Array::from_shape_simple_fn(vehicle.blocks.raw_dim(), || None);

    let grid_cell_size = (60, 60); //how big every cell should be in pixels
    let grid_size = vehicle.blocks.shape();
    let sim_scale = configuration.scale;

    let panel_texture = (asset_server.load("textures/metalPanel.png"), (100, 100));
    let wheel_texture = (asset_server.load("textures/saw.png"), (128, 128));

    let spawn_offset = Vec2::new(0.0, 0.0);
    for ((y, x), block) in vehicle.blocks.indexed_iter() {
        let texture;
        let collider_shape;
        let scale;
        let friction;
        match block {
            Block::Air => continue,
            Block::Panel => {
                texture = panel_texture.clone();
                let texture_size = texture.1;

                scale = grid_cell_size.0 as f32 / texture_size.0 as f32;
                collider_shape = ColliderShape::ball(scale / 2.); //TODO bug in rapier, can't use cubes here

                friction = 0.1;
            }
            Block::Wheel => {
                texture = wheel_texture.clone();
                let texture_size = texture.1;

                scale = grid_cell_size.0 as f32 / texture_size.0 as f32;
                collider_shape = ColliderShape::ball(scale / 2.);

                friction = 0.6;
            }
        }
        let pos = Vec2::new(
            (x as f32 - grid_size[0] as f32 / 2.) * (grid_cell_size.0 as f32) / sim_scale,
            -(y as f32 - grid_size[1] as f32 / 2.) * (grid_cell_size.1 as f32) / sim_scale,
        ) + spawn_offset;

        let rigid_body = RigidBodyBundle {
            position: pos.into(),
            ..Default::default()
        };

        let collider = ColliderBundle {
            shape: collider_shape,
            material: ColliderMaterial::new(friction, 0.1),
            flags: ColliderFlags {
                collision_groups: InteractionGroups::new(collider_group, collider_group),
                ..Default::default()
            },
            ..Default::default()
        };

        let material = materials.add(ColorMaterial {
            texture: texture.0.into(),
            color,
        }); //NOTE - every vehicle needs its own material, right? Otherwise hovering doesn't work

        let transform = Transform::from_scale(Vec3::ONE * scale);
        let sprite = SpriteBundle {
            material,
            transform,
            ..Default::default()
        };

        let entity = cmd
            .spawn_bundle(rigid_body)
            .insert_bundle(collider)
            .insert_bundle(sprite)
            .insert(RigidBodyPositionSync::Discrete)
            .insert(Name::new(format!("Block @ {}, {}", x, y)))
            .insert(BlockComponent::new(vehicle_id))
            .id();

        entities[(y, x)] = Some(EntityCell::new(entity, pos, *block));
    }

    entities
}

fn setup_joints(commands: &mut Commands, entities: Array2<Option<EntityCell>>) {
    //Takes two world positions and gets two local isometries
    fn get_pair_isometries_fixedjoint(pos1: Vec2, pos2: Vec2) -> (Isometry<Real>, Isometry<Real>) {
        let a = Isometry::identity();
        let b = Isometry::from(pos1 - pos2);

        (a, b)
    }

    //This cannot be a closure since it's generic
    fn spawn_joint<J: Into<JointParams>>(
        joint: J,
        e1: Entity,
        e2: Entity,
        commands: &mut Commands,
    ) {
        commands
            .spawn()
            .insert(JointBuilderComponent::new(joint, e1, e2));
    }

    //Note: b is assumed to be the wheel, a is fixed
    fn setup_ball_joint(a: &EntityCell, b: &EntityCell, commands: &mut Commands) {
        let (anchor_a, anchor_b) = (b.pos - a.pos, Vec2::ZERO);
        let mut joint = BallJoint::new(anchor_a.into(), anchor_b.into());
        joint.configure_motor_velocity(-60.0, 0.005);
        spawn_joint(joint, a.ent, b.ent, commands)
    }

    fn setup_fixed_joint(a: &EntityCell, b: &EntityCell, commands: &mut Commands) {
        let (iso_a, iso_b) = get_pair_isometries_fixedjoint(a.pos, b.pos);
        let joint = FixedJoint::new(iso_a, iso_b);
        spawn_joint(joint, a.ent, b.ent, commands);
    }

    //Connects two entities if they are both present
    let mut maybe_connect_entities = |ent1: &Option<EntityCell>, ent2: &Option<EntityCell>| {
        if let (Some(a), Some(b)) = (ent1, ent2) {
            match (&a.block_type, &b.block_type) {
                (Block::Panel, Block::Panel) => {
                    setup_fixed_joint(a, b, commands); //Panels are glued together
                }
                (Block::Panel, Block::Wheel) => {
                    setup_ball_joint(a, b, commands); //Panels and wheels roll together
                }
                (Block::Wheel, Block::Panel) => {
                    setup_ball_joint(b, a, commands); //Panels and wheels roll together
                }
                _ => {} //Else no joint
            }
        }
    };

    //Horizontal joints
    for win in entities.windows((2, 1)) {
        let ent_left = &win[[0, 0]];
        let ent_right = &win[[1, 0]];

        maybe_connect_entities(ent_left, ent_right);
    }

    //Vertical joints
    for win in entities.windows((1, 2)) {
        let ent_top = &win[[0, 0]];
        let ent_bottom = &win[[0, 1]];

        maybe_connect_entities(ent_top, ent_bottom);
    }
}
