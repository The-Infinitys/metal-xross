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

pub trait XrossGainProcessor {
    fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        config: &BufferConfig,
        context: &mut impl InitContext<MetalXross>,
    ) -> bool;

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize);
}

pub struct XrossGainSystem {
    params: Arc<MetalXrossParams>,
    crunch: XrossCrunchSystem,
    drive: XrossDriveSystem,
    dist: XrossDistSystem,
    metal: XrossMetalSystem,

    // 各スタイルの計算結果を一時保存するバッファ
    tmp_buffer_a: Vec<f32>,
    tmp_buffer_b: Vec<f32>,
    input_copy: Vec<f32>,
}

impl XrossGainSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params: params.clone(),
            crunch: XrossCrunchSystem::new(params.clone()),
            drive: XrossDriveSystem::new(params.clone()),
            dist: XrossDistSystem::new(params.clone()),
            metal: XrossMetalSystem::new(params.clone()),
            tmp_buffer_a: Vec::new(),
            tmp_buffer_b: Vec::new(),
            input_copy: Vec::new(),
        }
    }

    pub fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        config: &BufferConfig,
        context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        // 予期せぬ大きなバッファが来ても耐えられるよう、max または 2048 の大きい方を確保
        let reserve_size = (config.max_buffer_size as usize).max(2048);

        self.tmp_buffer_a = vec![0.0; reserve_size];
        self.tmp_buffer_b = vec![0.0; reserve_size];
        self.input_copy = vec![0.0; reserve_size];

        self.crunch.initialize(layout, config, context);
        self.drive.initialize(layout, config, context);
        self.dist.initialize(layout, config, context);
        self.metal.initialize(layout, config, context);

        true
    }
    pub fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<MetalXross>,
    ) {
        let style = self.params.style.kind.value();
        let idx_a = (style.floor() as usize).min(3);
        let idx_b = (idx_a + 1).min(3);
        let fraction = style - idx_a as f32;

        let num_samples = buffer.samples();

        // 各エンジンの参照
        let crunch = &mut self.crunch;
        let drive = &mut self.drive;
        let dist = &mut self.dist;
        let metal = &mut self.metal;

        for (ch_idx, channel_slice) in buffer.as_slice().iter_mut().enumerate() {
            // 現在のバッファサイズに合わせて、確保済みの領域からスライスを切り出す
            // もし何らかの理由で確保サイズより大きい要求が来ても落ちないように safety check
            let end = num_samples.min(self.input_copy.len());

            let in_copy = &mut self.input_copy[..end];
            let buf_a = &mut self.tmp_buffer_a[..end];
            let buf_b = &mut self.tmp_buffer_b[..end];

            // 1. 入力をコピー
            in_copy.copy_from_slice(&channel_slice[..end]);

            // 2. スタイルAの計算
            buf_a.copy_from_slice(in_copy);
            apply_style_to_slice(idx_a, crunch, drive, dist, metal, buf_a, ch_idx);

            // 3. スタイルBの計算
            buf_b.copy_from_slice(in_copy);
            apply_style_to_slice(idx_b, crunch, drive, dist, metal, buf_b, ch_idx);

            // 4. 線形補間
            for i in 0..end {
                channel_slice[i] = buf_a[i] * (1.0 - fraction) + buf_b[i] * fraction;
            }
        }
    }
}

fn apply_style_to_slice(
    index: usize,
    crunch: &mut XrossCrunchSystem,
    drive: &mut XrossDriveSystem,
    dist: &mut XrossDistSystem,
    metal: &mut XrossMetalSystem,
    slice: &mut [f32],
    ch_idx: usize,
) {
    match index {
        0 => crunch.process_channel(slice, ch_idx),
        1 => drive.process_channel(slice, ch_idx),
        2 => dist.process_channel(slice, ch_idx),
        _ => metal.process_channel(slice, ch_idx),
    }
}
