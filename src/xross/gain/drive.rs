use super::XrossGainProcessor;
use crate::MetalXross;
use crate::params::MetalXrossParams;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossDriveSystem {
    params: Arc<MetalXrossParams>,
    // 状態保持（ステレオ対応）
    low_cut: Vec<f32>,
    mid_boost: Vec<f32>,
    prev_sample: Vec<f32>,
}

impl XrossDriveSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            low_cut: vec![0.0; 2],
            mid_boost: vec![0.0; 2],
            prev_sample: vec![0.0; 2],
        }
    }

    // オーバードライブ特有のソフトクリッピング
    fn soft_clip(&self, x: f32) -> f32 {
        // 1.5 * x - 0.5 * x^3 に近い特性で、滑らかに飽和させる
        if x.abs() > 1.0 {
            x.signum()
        } else {
            1.5 * x - 0.5 * x.powi(3)
        }
    }
}

impl XrossGainProcessor for XrossDriveSystem {
    fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        let num_channels = layout.main_output_channels.map(|n| n.get()).unwrap_or(2) as usize;
        self.low_cut = vec![0.0; num_channels];
        self.mid_boost = vec![0.0; num_channels];
        self.prev_sample = vec![0.0; num_channels];
        true
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        let gain = self.params.gain.value();
        let tight = self.params.tight.value();
        let bright = self.params.bright.value();

        // 内部ゲイン: OverdriveなのでCrunchより少し低め、かつ密度を高く
        let drive_amount = gain * 40.0 + 1.5;

        for sample in slice {
            let mut x = *sample;

            // 1. PRE-FILTER (Mid-Push)
            // ギターにおいしい 700Hz〜1kHz 付近を強調しつつ、低域のモタつきをカット
            let hp_coef = 0.05 + (tight * 0.4);

            x = x - self.low_cut[ch_idx];
            self.low_cut[ch_idx] = *sample * hp_coef + self.low_cut[ch_idx] * (1.0 - hp_coef);

            // 2. DRIVE STAGE
            x *= drive_amount;

            // 3. SYMMETRIC SOFT CLIPPING
            // Overdriveは対称的なクリッピングにすることで、第3次高調波を出し
            // 「真空管アンプのパワー部」のような太い音にします
            x = self.soft_clip(x);

            // 4. TONE CONTROL (Bright)
            // 高域のチリチリした成分を調整
            let bright_val = x - self.mid_boost[ch_idx];
            self.mid_boost[ch_idx] = x * (0.1 + (1.0 - bright) * 0.5)
                + self.mid_boost[ch_idx] * (0.9 - (1.0 - bright) * 0.5);
            x = x + bright_val * bright;

            // 5. OUTPUT GAIN
            // 歪ませてもレベルが一定になるよう補正
            *sample = x * (0.6 / (1.0 + gain * 0.4));
        }
    }
}
