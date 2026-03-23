use nih_plug::prelude::*;

use crate::params::*;
use std::sync::Arc;
mod equalizer;
use equalizer::XrossEqualizer;

pub struct MetalXross {
    params: Arc<MetalXrossParams>,
    equalizer: XrossEqualizer,
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

        Self { params, equalizer }
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
        self.equalizer.process(buffer, aux, context);
        ProcessStatus::Normal
    }
}
