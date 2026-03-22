use crate::params::*;
use std::sync::Arc;

#[derive(Default)]
pub struct MetalXross {
    params: Arc<MetalXrossParams>,
}
impl MetalXross {
    pub fn new() -> Self {
        Self {
            params: Arc::new(MetalXrossParams::default()),
        }
    }
    pub fn params(&self) -> Arc<MetalXrossParams> {
        self.params.clone()
    }
    pub fn process(
        &mut self,
        _buffer: &mut nih_plug::prelude::Buffer,
        _aux: &mut nih_plug::prelude::AuxiliaryBuffers,
        _context: &mut impl nih_plug::prelude::ProcessContext<Self>,
    ) -> nih_plug::prelude::ProcessStatus {
        let _params = self.params();

        nih_plug::prelude::ProcessStatus::Normal
    }
}
