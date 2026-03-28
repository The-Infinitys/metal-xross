use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
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

    /// オーバードライブ特有の対称的なソフトクリッピング
    /// 3次高調波を生成し、温かみのある太い歪みを作る
    fn soft_clip(&self, x: f32) -> f32 {
        let abs_x = x.abs();
        if abs_x > 1.0 {
            x.signum()
        } else {
            // 1.5*x - 0.5*x^3 の多項式近似
            1.5 * x - 0.5 * x * x * x
        }
    }

    /// サンプルごとの処理
    fn process_sample(
        &mut self,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
        ch: usize,
    ) -> f32 {
        // --- 1. PRE-FILTER (Style Low -> Tightness & Mid-Push) ---
        // Style Lowが低いほどローカットを強め、中域をタイトに押し出す (TS系のような挙動)
        let tight_amount = (1.0 - s_low).powf(1.2);
        let hp_coef = 0.05 + (tight_amount * 0.4);

        let filtered = input - self.low_cut[ch];
        self.low_cut[ch] = input * hp_coef + self.low_cut[ch] * (1.0 - hp_coef);

        // --- 2. DRIVE STAGE (Style Mid -> Saturation Density) ---
        // Midを上げるとクリッパーへの突っ込みが強くなり、歪みの粘り（サチュレーション）が増す
        let drive_boost = 1.0 + (s_mid * 0.8);
        let drive_amount = gain * 30.0 * drive_boost + 1.2;

        let mut x = filtered * drive_amount;

        // --- 3. SYMMETRIC SOFT CLIPPING ---
        x = self.soft_clip(x);

        // --- 4. TONE CONTROL (Style High -> Brightness) ---
        // 高域の抜けを調整。Style Highが低いときはマイルドな高域カット、高いときはプレゼンスを強調
        let bright_val = x - self.mid_boost[ch];
        let lp_weight = 0.1 + (1.0 - s_high) * 0.6;

        self.mid_boost[ch] = x * lp_weight + self.mid_boost[ch] * (1.0 - lp_weight);
        x = x + bright_val * s_high;

        // --- 5. OUTPUT GAIN ---
        // 歪ませてもレベルが一定になるよう補正
        x * (0.6 / (1.0 + gain * 0.5))
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

        // Styleセクションの値を抽出
        let s_low = self.params.style.low.value();
        let s_mid = self.params.style.mid.value();
        let s_high = self.params.style.high.value();

        for sample in slice {
            *sample = self.process_sample(*sample, gain, s_low, s_mid, s_high, ch_idx);
        }
    }
}
