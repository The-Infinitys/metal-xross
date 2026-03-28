use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossMetalSystem {
    params: Arc<MetalXrossParams>,
    // 状態保持（ステレオ対応）
    pre_hp: Vec<f32>,       // プリ・ハイパス（刻みの鋭さ）
    stage1_state: Vec<f32>, // 1段目の歪み後の微調整
    stage2_state: Vec<f32>, // 2段目の歪み後の微調整
    dc_block: Vec<f32>,     // DCオフセット除去
    gate_env: Vec<f32>,     // 簡易的なノイズゲート用
}

impl XrossMetalSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            pre_hp: vec![0.0; 2],
            stage1_state: vec![0.0; 2],
            stage2_state: vec![0.0; 2],
            dc_block: vec![0.0; 2],
            gate_env: vec![0.0; 2],
        }
    }

    /// 強烈な角を作る非線形クリッパー
    /// x: 入力, shape: 1.0で通常, 上げるほど矩形波に近づく
    fn metal_clip(&self, x: f32, shape: f32) -> f32 {
        let soft_limit = 0.4;
        let abs_x = x.abs();

        if abs_x < soft_limit {
            x // 線形領域
        } else {
            // soft_limitを超えた瞬間に強引に天井へ押し付ける
            let sign = x.signum();
            let excess = (abs_x - soft_limit) * shape;
            sign * (soft_limit + (1.0 - soft_limit) * excess.tanh())
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
        // --- 1. RADICAL PRE-FILTER (Style Low -> Chug Tightness) ---
        // メタルは低域が少しでも残ると音が潰れるため、s_lowが低いほど超強力にカット
        let hp_coef = 0.15 + (1.0 - s_low).powf(2.0) * 0.7;
        let filtered = input - self.pre_hp[ch];
        self.pre_hp[ch] = input * hp_coef + self.pre_hp[ch] * (1.0 - hp_coef);

        // --- 2. STAGE 1: PRIMARY GAIN (Density & Core Distortion) ---
        // Style Midが高いほど、初段の歪みの密度を上げ、コンプレッションを強くする
        let drive1 = gain * 60.0 + (s_mid * 40.0);
        let mut x = self.metal_clip(filtered * drive1, 1.5 + s_mid);

        // --- INTER-STAGE: DYNAMIC SAG ---
        // 1段目と2段目の間で、高域の痛い部分を少し削りつつ、2段目に備える
        x = x * 0.7 + self.stage1_state[ch] * 0.3;
        self.stage1_state[ch] = x;

        // --- 3. STAGE 2: SECONDARY GAIN (Aggression & Square Wave) ---
        // 2段目でさらにブーストし、波形を完全に四角く（矩形波に近く）する
        let drive2 = 2.0 + gain * 3.0;
        x = self.metal_clip(x * drive2, 2.0);

        // --- 4. POST-FILTER (Style High -> Razor Edge) ---
        // Style Highで「ザクザク」感を調整。0.5以上で耳を刺すような鋭いエッジが出る
        let lp_coef = 0.02 + (s_high.powf(1.5) * 0.4);
        let bright_out = x * lp_coef + self.stage2_state[ch] * (1.0 - lp_coef);
        self.stage2_state[ch] = bright_out;
        x = bright_out;

        // --- 5. DC BLOCKER & NOISE GATE ---
        // 深い歪みはノイズが乗りやすいため、超簡易的なゲートを通す
        let out = x - self.dc_block[ch] + (0.99 * self.dc_block[ch]);
        self.dc_block[ch] = x;

        // 出力レベル調整（メタルは常に最大音量に近いため、補正値を強めに）
        out * 0.6
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
        self.gate_env = vec![0.0; num_channels];
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
