use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossMetalSystem {
    params: Arc<MetalXrossParams>,
    pre_hp: Vec<f32>,
    stage1_state: Vec<f32>,
    stage2_state: Vec<f32>,
    dc_block: Vec<f32>,
}

impl XrossMetalSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            pre_hp: vec![0.0; 2],
            stage1_state: vec![0.0; 2],
            stage2_state: vec![0.0; 2],
            dc_block: vec![0.0; 2],
        }
    }

    /// モダンメタルのための過激なクリッパー
    /// 線形領域を極端に狭め、0.2を超えたら即座に飽和させる
    fn metal_clip(&self, x: f32, hardness: f32) -> f32 {
        let threshold = 0.2;
        let abs_x = x.abs();

        if abs_x < threshold {
            x
        } else {
            // 閾値を超えた瞬間にtanhで急激に潰す。hardnessを上げると矩形波に近づく。
            let sign = x.signum();
            let excess = (abs_x - threshold) * hardness;
            sign * (threshold + (1.0 - threshold) * excess.tanh())
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
        // --- 1. EXTREME PRE-FILTER (Style Low: Tightness) ---
        // モダンメタルの命である「刻み」の鋭さを出すため、s_lowが低いほど超高域までカット
        // 5弦・6弦の開放弦を弾いてもボタつかないタイトさを確保
        let hp_freq = 0.15 + (1.0 - s_low).powf(1.8) * 0.45;
        let filtered = input - self.pre_hp[ch];
        self.pre_hp[ch] = input * hp_freq + self.pre_hp[ch] * (1.0 - hp_freq);

        // --- 2. PRE-GAIN & MID-SCOOP (Style Mid: Density) ---
        // Style Midが低いほどドンシャリ(Scooped)、高いほど中域が詰まったモダンな響きに
        let mid_impact = 0.5 + s_mid * 1.5;
        let p_gain = (15.0 + gain * 60.0) * mid_impact;

        // Stage 1: 初段で一気に歪ませる
        let mut x = self.metal_clip(filtered * p_gain, 2.0 + s_mid);

        // インターステージ・シェーピング（耳に痛い成分をわずかに平滑化）
        x = x * 0.8 + self.stage1_state[ch] * 0.2;
        self.stage1_state[ch] = x;

        // --- 3. STAGE 2: FINAL SATURATION ---
        // さらにゲインを重ねて波形を完全に四角くする
        let s2_gain = 2.0 + gain * 2.0;
        x = self.metal_clip(x * s2_gain, 3.0);

        // --- 4. POST-FILTER (Style High: Razor Edge) ---
        // メタル特有の「シュワシュワ」した高域をコントロール
        // Style Highが高いほど、高域を突き刺すようなエッジに変更
        let lp_freq = 0.05 + (s_high.powf(1.2) * 0.45);
        let bright_out = x * lp_freq + self.stage2_state[ch] * (1.0 - lp_freq);
        self.stage2_state[ch] = bright_out;

        // --- 5. DC BLOCKER & FINAL MAKEUP ---
        // バイアスが偏りやすいため、DC成分を除去
        let out = bright_out - self.dc_block[ch] + (0.995 * self.dc_block[ch]);
        self.dc_block[ch] = bright_out;

        // メタルは常に音圧が最大のため、ゲイン量に関わらず一定の出力を維持
        out * 0.5
    }
}

impl XrossGainProcessor for XrossMetalSystem {
    fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        let num_channels = layout.main_output_channels.map(|n| n.get()).unwrap_or(2) as usize;
        self.pre_hp = vec![0.0; num_channels];
        self.stage1_state = vec![0.0; num_channels];
        self.stage2_state = vec![0.0; num_channels];
        self.dc_block = vec![0.0; num_channels];
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
