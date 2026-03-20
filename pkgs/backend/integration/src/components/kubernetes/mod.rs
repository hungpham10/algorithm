mod node;
pub use node::*;

mod log;
pub use log::*;

mod ingress;
pub use ingress::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct NodeUsage {
    node: String,
    cpu: f64,
    mem: f64,
}
