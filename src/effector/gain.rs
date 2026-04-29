use crate::params::MetalXrossParams;
use std::sync::Arc;
use truce::prelude::*;

mod crunch;
mod dist;
mod drive;
mod metal;

use crunch::XrossCrunchSystem;
use dist::XrossDistSystem;
use drive::XrossDriveSystem;
use metal::XrossMetalSystem;

/// 各歪みエンジンの共通インターフェース
pub trait XrossGainProcessor {
    fn initialize(&mut self, sample_rate: f32, num_channels: usize);
    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize);
}

pub struct XrossGainSystem {
    params: Arc<MetalXrossParams>,
    crunch: XrossCrunchSystem,
    drive: XrossDriveSystem,
    dist: XrossDistSystem,
    metal: XrossMetalSystem,

    // 各スタイルのモーフィング用一時バッファ
    tmp_buffer_a: Vec<f32>,
    tmp_buffer_b: Vec<f32>,
    input_copy: Vec<f32>,

    sample_rate: f32,
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
            sample_rate: 44100.0,
        }
    }

    pub fn initialize(&mut self, sample_rate: f32, num_channels: usize) {
        self.sample_rate = sample_rate;

        // 各エンジンの初期化
        self.crunch.initialize(sample_rate, num_channels);
        self.drive.initialize(sample_rate, num_channels);
        self.dist.initialize(sample_rate, num_channels);
        self.metal.initialize(sample_rate, num_channels);
    }

    /// バッファサイズに応じて作業用領域を確保
    fn ensure_buffer_capacity(&mut self, size: usize) {
        if self.input_copy.len() < size {
            self.input_copy.resize(size, 0.0);
            self.tmp_buffer_a.resize(size, 0.0);
            self.tmp_buffer_b.resize(size, 0.0);
        }
    }

    pub fn process_buffer(&mut self, buffer: &mut AudioBuffer) {
        let num_channels = buffer.channels();
        let num_samples = buffer.num_samples();

        if num_samples == 0 {
            return;
        }
        self.ensure_buffer_capacity(num_samples);

        // Style パラメータ (0.0 - 3.0) から補間対象を決定
        let style = self.params.style_kind.value();
        let idx_a = (style.floor() as usize).min(3);
        let idx_b = (idx_a + 1).min(3);
        let fraction = style - idx_a as f32;

        for ch_idx in 0..num_channels {
            // truce AudioBuffer から入力を取得
            let (inp, out) = buffer.io(ch_idx);

            // 1. オリジナル入力を退避
            self.input_copy[..num_samples].copy_from_slice(&inp[..num_samples]);

            // 2. スタイルAの計算
            self.tmp_buffer_a[..num_samples].copy_from_slice(&self.input_copy[..num_samples]);
            apply_style_to_slice(
                idx_a,
                &mut self.crunch,
                &mut self.drive,
                &mut self.dist,
                &mut self.metal,
                &mut self.tmp_buffer_a[..num_samples],
                ch_idx,
            );

            if fraction > 0.001 {
                // 3. スタイルBの計算（モーフィング中のみ）
                self.tmp_buffer_b[..num_samples].copy_from_slice(&self.input_copy[..num_samples]);
                apply_style_to_slice(
                    idx_b,
                    &mut self.crunch,
                    &mut self.drive,
                    &mut self.dist,
                    &mut self.metal,
                    &mut self.tmp_buffer_b[..num_samples],
                    ch_idx,
                );

                // 4. 線形補間（モーフィング）
                let a = &self.tmp_buffer_a[..num_samples];
                let b = &self.tmp_buffer_b[..num_samples];
                for i in 0..num_samples {
                    out[i] = a[i] * (1.0 - fraction) + b[i] * fraction;
                }
            } else {
                // 補間不要な場合は A をそのまま出力
                out[..num_samples].copy_from_slice(&self.tmp_buffer_a[..num_samples]);
            }
        }
    }
}

/// インデックスに応じて適切な歪みエンジンを呼び出すヘルパー
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
