#![warn(missing_docs, rust_2018_idioms, rust_2018_compatibility)]
#![allow(dead_code)]
#![allow(missing_docs)]
#[macro_use]
extern crate amethyst_derive;

#[macro_use]
extern crate shred_derive;

#[macro_use]
extern crate log;

pub use crate::{
    component::{
        Terrain, 
        ActiveTerrain,
    },
    renderer::{
        DrawTerrain,
        TerrainConfig,
        TerrainViewMode,
    },
    // system::{
    //     TerrainSystem,
    // },
};




mod component;
mod renderer;
mod system;