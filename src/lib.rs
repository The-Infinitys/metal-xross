use truce::prelude::*;
mod editor;
mod effector;
mod params;
mod plugin;
mod utils;
use effector::*;
use params::*;

truce::plugin! {
    logic: MetalXross,
    params: MetalXrossParams,
}
