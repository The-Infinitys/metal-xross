use crate::params::{self, *};
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
        buffer: &mut nih_plug::prelude::Buffer,
        aux: &mut nih_plug::prelude::AuxiliaryBuffers,
        context: &mut impl nih_plug::prelude::ProcessContext<Self>,
    ) -> nih_plug::prelude::ProcessStatus {
        let params = self.params();

        nih_plug::prelude::ProcessStatus::Normal
    }
}
