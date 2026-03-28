use crate::MetalXross;
use crate::params::MetalXrossParams;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossLevelSystem {
    params: Arc<MetalXrossParams>,
}

impl XrossLevelSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self { params }
    }

    pub fn post_process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<MetalXross>,
    ) {
        let target_level = self.params.general.out_level.value();
        let threshold = 0.9; // 抑え込みたい基準値

        // 1. まず、現在のバッファー内での最大絶対値（ピーク）を探す
        let mut max_abs = 0.0f32;
        buffer
            .as_slice_immutable()
            .iter()
            .for_each(|channel_slice| {
                channel_slice.iter().for_each(|sample| {
                    let abs_v = (sample * target_level).abs();
                    if abs_v > max_abs {
                        max_abs = abs_v;
                    }
                });
            });

        // 2. もしピークが threshold を超えるなら、超えないための減衰係数を計算する
        // 超えない場合は 1.0 (そのまま)
        let reduction_factor = if max_abs > threshold {
            threshold / max_abs
        } else {
            1.0
        };

        // 3. 最終的なゲインを適用（一律に掛けるので音の性質は変わらない）
        let final_gain = target_level * reduction_factor;

        buffer.as_slice().iter_mut().for_each(|channel_slice| {
            for v in channel_slice.iter_mut() {
                *v *= final_gain;
            }
        });
    }
    pub fn pre_process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<MetalXross>,
    ) {
        let target_level = self.params.general.in_level.value();
        buffer.as_slice().iter_mut().for_each(|channel_slice| {
            channel_slice.iter_mut().for_each(|v| *v *= target_level);
        });
    }
}
