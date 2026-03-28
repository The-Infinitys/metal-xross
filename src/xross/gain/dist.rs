use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossDistSystem {
    params: Arc<MetalXrossParams>,
    // 状態保持（ステレオ対応）
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

    /// ハード・クリッピング関数
    /// 線形領域を狭くし、限界値付近で急激に「角」を立たせる
    fn hard_clip(&self, x: f32, drive: f32) -> f32 {
        let pushed = x * drive;
        // 0.5を超えると急激にサチュレート
        if pushed.abs() < 0.5 {
            pushed
        } else {
            // tanhよりもクリッピングポイントが明確なシグモイド
            pushed.signum() * (0.5 + 0.5 * ((pushed.abs() - 0.5) * 2.0).tanh())
        }
    }

    /// サンプルごとの処理ロジック
    fn process_sample(
        &mut self,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
        ch: usize,
    ) -> f32 {
        // --- 1. PRE-EQ (Style Low -> Radical Tightening) ---
        // ディストーションは低域が濁ると致命的なため、s_lowが低いほど大胆にカット
        let hp_coef = 0.08 + (1.0 - s_low) * 0.72;
        let filtered = input - self.low_cut[ch];
        self.low_cut[ch] = input * hp_coef + self.low_cut[ch] * (1.0 - hp_coef);

        // --- 2. PRE-SHAPE (Style Mid -> Mid-Range Focus) ---
        // Style Midが高いほど、歪みの前に中域を強く盛り上げ、音を前に出す
        let mid_focus = s_mid * 0.8;
        let shaped = filtered + (filtered - self.pre_shape[ch]) * mid_focus;
        self.pre_shape[ch] = filtered;

        // --- 3. MAIN DISTORTION STAGE (Style Mid also affects Gain) ---
        // ディストーション用のハイゲイン設定
        let drive_amount = gain * 70.0 + 5.0 + (s_mid * 20.0);
        let mut x = self.hard_clip(shaped, drive_amount);

        // --- 4. SECONDARY CLIP (Wave Squashing) ---
        // 波形をさらに四角くし、サステインと倍音を稼ぐ
        x = (x * 1.6).clamp(-1.0, 1.0) * 0.85;

        // --- 5. POST-FILTER (Style High -> Edge Control) ---
        // Style Highが低いときはダークに、高いときは「ザクザク」したエッジを強調
        let lp_coef = 0.03 + (s_high * 0.57);
        let final_out = x * lp_coef + self.post_lp[ch] * (1.0 - lp_coef);
        self.post_lp[ch] = final_out;

        // --- 6. OUTPUT LEVEL ---
        // 圧縮感が強いため、出力は少し控えめに調整
        final_out * 0.45
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
        let s_low = self.params.style.low.value();
        let s_mid = self.params.style.mid.value();
        let s_high = self.params.style.high.value();

        for sample in slice {
            *sample = self.process_sample(*sample, gain, s_low, s_mid, s_high, ch_idx);
        }
    }
}
