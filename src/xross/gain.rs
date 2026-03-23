use crate::{MetalXross, params::MetalXrossParams};
use nih_plug::prelude::*;
use std::sync::Arc;
mod crunch;
mod dist;
mod drive;
mod metal;
use crunch::XrossCrunchSystem;
use dist::XrossDistSystem;
use drive::XrossDriveSystem;
use metal::XrossMetalSystem;

pub struct XrossGainSystem {
    params: Arc<MetalXrossParams>,
    crunch: XrossCrunchSystem,
    drive: XrossDriveSystem,
    dist: XrossDistSystem,
    metal: XrossMetalSystem,
}

impl XrossGainSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        let crunch = XrossCrunchSystem::new(params.clone());
        let drive = XrossDriveSystem::new(params.clone());
        let dist = XrossDistSystem::new(params.clone());
        let metal = XrossMetalSystem::new(params.clone());
        Self {
            params,
            crunch,
            drive,
            dist,
            metal,
        }
    }
    pub fn process(
        &self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<MetalXross>,
    ) {
    }
    pub fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        true
    }
}
