use std::sync::Arc;

use truce::params::Params;
use truce::prelude::*;

use crate::editor::create;
use crate::MetalXrossParams;

pub struct MetalXross {
    params: Arc<MetalXrossParams>,
}

impl MetalXross {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self { params }
    }
    pub fn reset(&mut self, sr: f64, _bs: usize) {
        self.params.set_sample_rate(sr);
        self.params.snap_smoothers();
    }
    pub fn process(
        &mut self,
        buffer: &mut AudioBuffer,
        _events: &EventList,
        _context: &mut ProcessContext,
    ) -> ProcessStatus {
        for i in 0..buffer.num_samples() {
            let gain = db_to_linear(self.params.gain.smoothed_next() as f64) as f32;
            for ch in 0..buffer.channels() {
                let (inp, out) = buffer.io(ch);
                out[i] = inp[i] * gain;
            }
        }
        ProcessStatus::Normal
    }
    pub fn params(&self) -> Arc<MetalXrossParams> {
        self.params.clone()
    }
    pub fn ui(&self) -> Box<dyn Editor> {
        create(self.params())
    }
}
