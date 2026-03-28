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
        let gain = self.params.general.output.gain.value();
        let limit = self.params.general.output.limit.value();
        self.process(buffer, gain, limit);
    }
    pub fn pre_process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<MetalXross>,
    ) {
        let gain = self.params.general.input.gain.value();
        let limit = self.params.general.input.limit.value();
        self.process(buffer, gain, limit);
    }
    fn process(&mut self, buffer: &mut Buffer, gain: f32, limit: f32) {
        // 1. まず、現在のバッファー内での最大絶対値（ピーク）を探す
        let mut max_abs = 0.0f32;
        buffer
            .as_slice_immutable()
            .iter()
            .for_each(|channel_slice| {
                channel_slice.iter().for_each(|sample| {
                    let abs_v = (sample * gain).abs();
                    if abs_v > max_abs {
                        max_abs = abs_v;
                    }
                });
            });

        // 2. もしピークが threshold を超えるなら、超えないための減衰係数を計算する
        // 超えない場合は 1.0 (そのまま)
        let reduction_factor = if max_abs > limit {
            limit / max_abs
        } else {
            1.0
        };

        // 3. 最終的なゲインを適用（一律に掛けるので音の性質は変わらない）
        let final_gain = gain * reduction_factor;

        buffer.as_slice().iter_mut().for_each(|channel_slice| {
            for v in channel_slice.iter_mut() {
                *v *= final_gain;
            }
        });
    }
}
