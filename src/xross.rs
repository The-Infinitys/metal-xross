use nih_plug::prelude::*;

use crate::params::*;
use std::sync::Arc;
mod equalizer;
use equalizer::XrossEqualizer;
mod noise_gate;
use noise_gate::XrossNoiseGate;
mod gain;
use gain::XrossGainSystem;

pub struct MetalXross {
    params: Arc<MetalXrossParams>,
    equalizer: XrossEqualizer,
    noise_gate: XrossNoiseGate,
    gain: XrossGainSystem,
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
        Self {
            params,
            equalizer,
            noise_gate,
            gain,
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
        self.noise_gate.process(buffer, aux, context);
        self.equalizer.process(buffer, aux, context);
        ProcessStatus::Normal
    }
    pub fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.noise_gate
            .initialize(audio_io_layout, buffer_config, context)
            && self
                .gain
                .initialize(audio_io_layout, buffer_config, context)
    }
}
