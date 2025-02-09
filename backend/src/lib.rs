//#![deny(warnings)]
#![feature(portable_simd)]

extern crate diesel;
extern crate serde;

pub mod components;
pub mod interfaces;
pub mod actors;
pub mod algorithm;
pub mod analytic;
pub mod cmds;
pub mod helpers;
pub mod load;
pub mod schemas;
pub mod api;
