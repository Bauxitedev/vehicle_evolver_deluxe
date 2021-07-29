use ndarray::Array2;
use num_enum::TryFromPrimitive;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use rand::seq::IteratorRandom;
use std::{
    convert::TryFrom,
    fmt::{Display, Write},
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use log::{debug, error, info, trace, warn}; //IMPORTANT or you won't get any output during tests!

#[derive(TryFromPrimitive, Clone, Debug, PartialEq, EnumIter, Eq, Hash, Copy)]
#[repr(u8)]
pub enum Block {
    Air,
    Panel,
    Wheel,
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            //TODO the monospace rendering is broken! These should all be equally wide but they aren't!
            Block::Air => f.write_char('◻'),
            Block::Panel => f.write_char('◼'),
            Block::Wheel => f.write_char('⭕'),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Eq, Hash)]

pub struct Vehicle {
    pub blocks: Array2<Block>,
}

impl std::fmt::Display for Vehicle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in self.blocks.rows() {
            for x in row {
                write!(f, "{}", x)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

pub const VEHICLE_SHAPE: (usize, usize) = (6, 8); //rows, columns
impl Vehicle {
    pub fn new() -> Self {
        let rng = thread_rng();
        Vehicle::new_with_rng(rng)
    }

    pub fn new_with_rng<R>(mut rng: R) -> Self
    where
        R: Rng + Sized,
    {
        let blocks = Array2::from_shape_fn(VEHICLE_SHAPE, |(_x, _y)| {
            let dist = WeightedIndex::new(&[0.4, 1.0, 0.3]).unwrap();
            Block::try_from(dist.sample(&mut rng) as u8).unwrap()
        });
        Vehicle { blocks }
    }

    pub fn new_empty() -> Self {
        Vehicle::new_fill_with(Block::Air)
    }

    pub fn new_fill_with(b: Block) -> Self {
        let blocks = Array2::from_shape_simple_fn(VEHICLE_SHAPE, || b);
        Vehicle { blocks }
    }

    #[allow(dead_code)]
    pub fn from(blocks: Vec<Block>) -> Self {
        let blocks = Array2::from_shape_vec(VEHICLE_SHAPE, blocks)
            .expect("couldn't convert Vec<Blocks> to Array2<Block>");
        Vehicle { blocks }
    }

    pub fn mutate(&mut self, amount: usize) {
        //pick x blocks and mutate them (note: mutation may not do anything, e.g. O -> O)

        let mut rng = thread_rng();

        info!("mutating with amount {}...", amount);

        for ((x, y), block) in self
            .blocks
            .indexed_iter_mut()
            .choose_multiple(&mut rng, amount)
        {
            let new_block = Block::iter().choose(&mut rng).unwrap();
            info!("mutated at {},{} from {} to {}", x, y, block, new_block);

            *block = new_block;
        }
    }

    #[allow(dead_code)]
    pub fn uniform_crossover(&self, other: &Vehicle) -> (Vehicle, Vehicle) {
        let mut rng = rand::thread_rng();

        let mut brother = Vehicle::new_empty();
        let mut sister = Vehicle::new_empty();

        assert_eq!(brother.blocks.shape(), sister.blocks.shape());
        for (((y, x), block_sister), (_, block_brother)) in sister
            .blocks
            .indexed_iter_mut()
            .zip(brother.blocks.indexed_iter_mut())
        {
            if rng.gen() {
                *block_sister = self.blocks[(y, x)];
                *block_brother = other.blocks[(y, x)];
            } else {
                *block_sister = other.blocks[(y, x)];
                *block_brother = self.blocks[(y, x)];
            }
        }

        (brother, sister)
    }

    pub fn one_point_crossover(
        &self,
        other: &Vehicle,
        crossover_point: usize,
    ) -> (Vehicle, Vehicle) {
        assert!(crossover_point >= 1);
        assert!(crossover_point < VEHICLE_SHAPE.1);

        let blocks_brother = Array2::from_shape_fn(VEHICLE_SHAPE, |(y, x)| {
            if x >= crossover_point {
                other.blocks[(y, x)]
            } else {
                self.blocks[(y, x)]
            }
        });

        let blocks_sister = Array2::from_shape_fn(VEHICLE_SHAPE, |(y, x)| {
            if x >= crossover_point {
                self.blocks[(y, x)]
            } else {
                other.blocks[(y, x)]
            }
        });
        (
            Vehicle {
                blocks: blocks_brother,
            },
            Vehicle {
                blocks: blocks_sister,
            },
        )
    }
}

#[cfg(test)]
#[test]
fn test() {
    let v1 = Vehicle::new_fill_with(Block::Panel);
    let v2 = Vehicle::new_fill_with(Block::Wheel);

    for x in 1..8 {
        let v3 = v1.one_point_crossover(&v2, x);
        info!(
            "vehicle after one-point crossover:\n{}\n\n{}\n---",
            v3.0, v3.1
        );
    }

    for _ in 1..8 {
        let v3 = v1.uniform_crossover(&v2);
        info!(
            "vehicle after uniform crossover:\n{}\n\n{}\n---",
            v3.0, v3.1
        );
    }
}
