use serde::{Deserialize, Serialize};

use menu::Context as MenuContext;

#[derive(Serialize, Deserialize, Clone)]
struct Padding {
    page: String,
}

mod application;
mod dropdown;
mod footer;
mod header;
mod logo;
mod main;
mod menu;
mod search;
mod vertical;

pub use application::Application;
