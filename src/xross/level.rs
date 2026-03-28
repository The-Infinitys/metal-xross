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

    /// 最終出力用のソフト・リミッター
    /// 0.9を超えたあたりから tanh で滑らかに圧縮し、デジタルクリップを防ぐ
    fn safety_clip(&self, x: f32) -> f32 {
        let threshold = 0.9;
        let abs_x = x.abs();

        if abs_x <= threshold {
            x
        } else {
            // thresholdを超えた分を滑らかに圧縮 (0.9 ~ 1.0の間に収める)
            let sign = x.signum();
            let excess = abs_x - threshold;
            // 1.0を漸近線とするようにスケーリング
            sign * (threshold + (1.0 - threshold) * excess.tanh())
        }
    }

    pub fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<MetalXross>,
    ) {
        let target_level = self.params.level.value();

        // チャンネルごとに処理を行う
        for channel_slice in buffer.as_slice() {
            for sample in channel_slice.iter_mut() {
                // 1. パラメータによるレベル増幅
                let mut x = *sample * target_level;

                // 2. セーフティ・クリップの適用
                x = self.safety_clip(x);

                *sample = x;
            }
        }
    }
}
