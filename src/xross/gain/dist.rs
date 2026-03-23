use super::XrossGainProcessor;
use crate::MetalXross;
use crate::params::MetalXrossParams;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossDistSystem {
    params: Arc<MetalXrossParams>,
    // 状態保持
    low_cut: Vec<f32>,
    pre_shape: Vec<f32>,
    post_lp: Vec<f32>,
}

impl XrossDistSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            low_cut: vec![0.0; 2],
            pre_shape: vec![0.0; 2],
            post_lp: vec![0.0; 2],
        }
    }

    // ハード・クリッピング関数
    // soft_clipよりも急激に限界値へ到達させ、高調波を増やす
    fn hard_clip(&self, x: f32, drive: f32) -> f32 {
        let pushed = x * drive;
        // 独自の非線形関数: tanhよりも「角」が立つ特性
        if pushed.abs() < 0.5 {
            pushed // 線形領域
        } else {
            // 0.5を超えると急激にサチュレートし、1.0付近で止める
            pushed.signum() * (0.5 + 0.5 * ((pushed.abs() - 0.5) * 2.0).tanh())
        }
    }
}

impl XrossGainProcessor for XrossDistSystem {
    fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        let num_channels = layout.main_output_channels.map(|n| n.get()).unwrap_or(2) as usize;
        self.low_cut = vec![0.0; num_channels];
        self.pre_shape = vec![0.0; num_channels];
        self.post_lp = vec![0.0; num_channels];
        true
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        let gain = self.params.gain.value();
        let tight = self.params.tight.value();
        let bright = self.params.bright.value();

        // Distortion用のハイゲイン設定 (Driveよりさらに強力)
        let drive_amount = gain * 80.0 + 5.0;

        for sample in slice {
            let input = *sample;

            // 1. PRE-EQ (Tightening)
            // ディストーションは低域が濁ると「ブーミー」になるため、
            // tightを上げるとかなり大胆に低域をカットする
            let hp_coef = 0.1 + (tight * 0.7);
            let filtered = input - self.low_cut[ch_idx];
            self.low_cut[ch_idx] = input * hp_coef + self.low_cut[ch_idx] * (1.0 - hp_coef);

            // 2. PRE-SHAPE (中域の強調)
            // 80年代ディストーションのような「突き抜ける音」にするため中域を盛る
            let mid_boost = filtered + (filtered - self.pre_shape[ch_idx]) * 0.5;
            self.pre_shape[ch_idx] = filtered;

            // 3. MAIN DISTORTION STAGE
            // ハード・クリッピングによる深い歪み
            let mut x = self.hard_clip(mid_boost, drive_amount);

            // 4. SECONDARY CLIP (さらに波形を四角くする)
            x = (x * 1.5).clamp(-1.0, 1.0) * 0.8;

            // 5. POST-FILTER (Tone / Bright)
            // ディストーション特有の「痛い高域」をBrightつまみでコントロール
            // Brightが低いときはかなりダークに、高いときはザクザクしたエッジを残す
            let lp_coef = 0.05 + (bright * 0.6);
            let final_out = x * lp_coef + self.post_lp[ch_idx] * (1.0 - lp_coef);
            self.post_lp[ch_idx] = final_out;

            // 6. OUTPUT LEVEL
            *sample = final_out * 0.5;
        }
    }
}
