use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossDistSystem {
    params: Arc<MetalXrossParams>,
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

    /// ディストーション特有の「粘り」のあるハードクリッピング
    /// 完全に角を落とすのではなく、倍音が豊かになるポイントで非線形性を強める
    fn dist_clip(&self, x: f32) -> f32 {
        let abs_x = x.abs();
        if abs_x < 0.3 {
            // 微小信号はリニアに（音の輪郭を維持）
            x
        } else {
            // 0.3を超えるとソフトかつ力強く圧縮
            // tanhよりも少し「壁」を感じるシグモイド曲線
            let sign = x.signum();
            let saturated = 0.3 + 0.65 * ((abs_x - 0.3) * 1.5).tanh();
            sign * saturated.min(0.95)
        }
    }

    fn process_sample(
        &mut self,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
        ch: usize,
    ) -> f32 {
        // --- 1. PRE-EQ (Style Low: Tightness) ---
        // DISTは低域の「ボフボフ」を防ぐため、s_lowが低いほど大胆にカット
        let hp_freq = 0.05 + (1.0 - s_low).powf(1.8) * 0.3;
        let filtered = input - self.low_cut[ch];
        self.low_cut[ch] = input * hp_freq + self.low_cut[ch] * (1.0 - hp_freq);

        // --- 2. MID-FOCUS (Style Mid: Presence) ---
        // Style Midが高いほど、歪み前の段階で中域を強調し、粘りと押し出しを強くする
        let mid_boost = 1.0 + s_mid * 1.2;
        let shaped = filtered * mid_boost;

        // --- 3. GAIN STAGE (Gain 0.0でも歪む設定) ---
        // 基礎ゲインを 8.0 に設定。Gain 0.0でも十分なドライブ感。
        // 最大ゲインは METAL モードの手前（約 60倍）に留める
        let drive_amount = 8.0 + (gain * 52.0);
        let mut x = shaped * drive_amount;

        // --- 4. MULTI-STAGE DISTORTION ---
        // 2段階でクリップさせ、サステインを極限まで稼ぐ
        x = self.dist_clip(x);
        x = self.dist_clip(x * 1.4);

        // --- 5. POST-FILTER (Style High: Edge) ---
        // 歪みで増えた高域をStyle Highで調整。高いほど「ザクッ」とした質感に。
        let lp_freq = 0.1 + (s_high * 0.55);
        let final_out = x * lp_freq + self.post_lp[ch] * (1.0 - lp_freq);
        self.post_lp[ch] = final_out;

        // --- 6. OUTPUT MAPPING ---
        // 強い圧縮がかかるため、出力レベルを補正
        let makeup = 0.55 / (1.0 + gain * 0.4);
        final_out * makeup
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
        let gain = self.params.general.gain.value();
        let s_low = self.params.style.low.value();
        let s_mid = self.params.style.mid.value();
        let s_high = self.params.style.high.value();

        for sample in slice {
            *sample = self.process_sample(*sample, gain, s_low, s_mid, s_high, ch_idx);
        }
    }
}
