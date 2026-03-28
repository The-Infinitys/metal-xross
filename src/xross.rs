use nih_plug::prelude::*;

use crate::params::*;
use std::sync::Arc;
mod equalizer;
use equalizer::XrossEqualizer;
mod noise_gate;
use noise_gate::XrossNoiseGate;
mod gain;
use gain::XrossGainSystem;
mod level;
use level::XrossLevelSystem;

pub struct MetalXross {
    params: Arc<MetalXrossParams>,
    equalizer: XrossEqualizer,
    noise_gate: XrossNoiseGate,
    gain: XrossGainSystem,
    level: XrossLevelSystem,
}
impl Default for MetalXross {
    fn default() -> Self {
        Self::new()
    }
}
impl MetalXross {
    pub fn new() -> Self {
        let params = Arc::new(MetalXrossParams::default());
        let equalizer = XrossEqualizer::new(Arc::clone(&params));
        let noise_gate = XrossNoiseGate::new();
        let gain = XrossGainSystem::new(Arc::clone(&params));
        let level = XrossLevelSystem::new(Arc::clone(&params));
        Self {
            params,
            equalizer,
            noise_gate,
            gain,
            level,
        }
    }
    pub fn params(&self) -> Arc<MetalXrossParams> {
        self.params.clone()
    }
    pub fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.noise_gate.process_pre(buffer);
        self.gain.process(buffer, aux, context);
        self.noise_gate.process_post(buffer);
        self.equalizer.process(buffer, aux, context);
        self.level.process(buffer, aux, context);
        ProcessStatus::Normal
    }
    pub fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.noise_gate.initialize(buffer_config.sample_rate);
        let gain = self
            .gain
            .initialize(audio_io_layout, buffer_config, context);
        gain
    }
}
