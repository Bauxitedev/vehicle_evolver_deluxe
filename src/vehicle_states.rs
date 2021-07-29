use crate::vehicle::Vehicle;
use bevy::prelude::Color;
use std::fmt::*;

use log::{debug, error, info, trace, warn}; //IMPORTANT or you won't get any output during tests!

#[derive(Clone)]
pub struct VehicleStates(Vec<VehicleState>);

const FITNESS_FINISH_THRESHOLD: i64 = 14400; //If fitness goes above this value, the vehicle reached the finish flag

impl VehicleStates {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let mut v = vec![];

        for _ in 0..10 {
            v.push(VehicleState::new());
        }
        VehicleStates(v)
    }

    pub fn from(pop: Vec<Vehicle>) -> Self {
        VehicleStates(pop.into_iter().map(VehicleState::from).collect())
    }

    pub fn all_done(&self) -> bool {
        self.0
            .iter()
            .all(|state| matches!(state.status, VehicleStatus::Done))
    }

    pub fn get_vehicle_states(&self) -> &Vec<VehicleState> {
        &self.0
    }

    pub fn get_vehicle_states_mut(&mut self) -> &mut Vec<VehicleState> {
        &mut self.0
    }

    pub fn pop_vehicles(&mut self, limit: usize) -> Vec<(Vehicle, VehicleID, Color)> {
        //Finds all vehicle state that is Pending, turns it to Running, and returns its inner Vehicle.
        //Returns nothing if no pending vehicles exist

        let gradient = colorous::TURBO;

        self.0
            .iter_mut()
            .enumerate()
            .filter(|(_, state)| state.status == VehicleStatus::Pending)
            .take(limit)
            .enumerate()
            .map(|(color_idx, (i, state))| {
                state.status = VehicleStatus::Running; //Set first n Pending vehicles to Running

                let color = gradient.eval_rational(color_idx, limit);
                let mut color = Color::rgb(
                    color.r as f32 / 255.,
                    color.g as f32 / 255.,
                    color.b as f32 / 255.,
                );

                if limit == 1 {
                    //If only 1 vehicle popped, use white
                    color = Color::WHITE;
                }

                (state.vehicle.clone(), VehicleID(i), color) //And return them
            })
            .collect()
    }

    pub fn set_fitness(&mut self, VehicleID(i): VehicleID, new_fitness: i64, fell_apart: bool) {
        let vehicle = &mut self.0[i];
        vehicle.fitness = new_fitness;

        if new_fitness >= FITNESS_FINISH_THRESHOLD {
            vehicle.reached_finish = true;
        }

        vehicle.fell_apart = fell_apart;
    }

    pub fn finalize_vehicle(&mut self, VehicleID(i): VehicleID) -> (Vehicle, i64) {
        let mut new_status = None;
        let state = &mut self.0[i];

        match state.status {
            VehicleStatus::Running => {
                new_status = Some(VehicleStatus::Done);
            }
            VehicleStatus::Pending => {
                panic!("tried to finalize vehicle that hasn't been run yet");
            }
            VehicleStatus::Done => {
                warn!("vehicle already finalized");
            }
        }

        if let Some(new_status) = new_status {
            state.status = new_status;
        }

        let fitness = state.fitness;

        (state.vehicle.clone(), fitness)
    }
}

#[derive(PartialEq, Debug, Clone)]

pub struct VehicleState {
    pub vehicle: Vehicle,
    pub fitness: i64,
    pub status: VehicleStatus,
    pub reached_finish: bool,
    pub is_camera_target: bool,
    pub fell_apart: bool,
}

impl VehicleState {
    pub fn new() -> Self {
        VehicleState {
            vehicle: Vehicle::new(),
            fitness: i64::MIN,
            status: VehicleStatus::Pending,
            reached_finish: false,
            is_camera_target: false,
            fell_apart: false,
        }
    }

    pub fn from(v: Vehicle) -> Self {
        VehicleState {
            vehicle: v,
            fitness: i64::MIN,
            status: VehicleStatus::Pending,
            reached_finish: false,
            is_camera_target: false,
            fell_apart: false,
        }
    }
}
#[derive(PartialEq, Debug, Clone)]

pub enum VehicleStatus {
    Pending,
    Running,
    Done,
}

#[derive(PartialEq, Debug, Clone, Copy)]

pub struct VehicleID(pub usize);

impl Display for VehicleState {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let fitness = self.fitness;

        let finish_icon = if self.reached_finish { "🏁" } else { " " };
        let camera_icon = if self.is_camera_target { "🔆" } else { " " };
        let fell_apart_icon = if self.fell_apart { "❌" } else { " " }; //💀

        match self.status {
            VehicleStatus::Pending => {
                write!(f, "🕒")
            }
            VehicleStatus::Running => {
                write!(
                    f,
                    "🔄 fitness = {:#5} {} {} {}",
                    fitness, finish_icon, fell_apart_icon, camera_icon
                )
            }
            VehicleStatus::Done => {
                write!(
                    f,
                    "✅ fitness = {:#5} {} {} {}",
                    fitness, finish_icon, fell_apart_icon, camera_icon
                )
            }
        }
    }
}
