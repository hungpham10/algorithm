use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Padding {
    page: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Features {
    padding: Padding,
    logo: String,
    menu: Vec<String>,
    contents: Vec<String>,
    searchable: bool,
}

mod application;
mod contents;
mod footer;
mod header;
mod logo;
mod main;
mod menu;
mod search;

pub use application::Application;
