// src/xross/gain/Metal.rs
use super::XrossGainProcessor; // トレイトをインポート
use crate::MetalXross;
use crate::params::MetalXrossParams;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossMetalSystem {
    params: Arc<MetalXrossParams>,
}

impl XrossMetalSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self { params }
    }
}

// ここが重要！
impl XrossGainProcessor for XrossMetalSystem {
    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        true
    }

    fn process_channel(
        &mut self,
        slice: &mut [f32],
        ch_idx: usize,
        // contextが必要な場合は渡すが、多くの場合はparamsの値だけで十分
    ) {
    }
}
