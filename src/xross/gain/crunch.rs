use super::XrossGainProcessor;
use crate::MetalXross;
use crate::params::MetalXrossParams;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossCrunchSystem {
    params: Arc<MetalXrossParams>,
    // チャンネルごとの状態保持
    prev_in: Vec<f32>,
    prev_out: Vec<f32>,
    low_cut_state: Vec<f32>,
    high_shelf_state: Vec<f32>,
    // 動的なバイアス（エンベロープ）追従用
    bias_env: Vec<f32>,
}

impl XrossCrunchSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            prev_in: vec![0.0; 2],
            prev_out: vec![0.0; 2],
            low_cut_state: vec![0.0; 2],
            high_shelf_state: vec![0.0; 2],
            bias_env: vec![0.0; 2],
        }
    }

    // 真空管の非対称なカーブをシミュレート
    // x: 入力, bias: グリッドバイアスのズレ
    fn tube_sample(&self, x: f32, bias: f32) -> f32 {
        let input = x + bias;
        if input > 0.0 {
            // 正相：柔らかく飽和 (Soft Saturation)
            input.tanh()
        } else {
            // 逆相：より急激に、かつ深く沈み込む (Grid Clipping)
            let out = (input * 0.6).atan() * 1.6;
            out.max(-0.95) // 物理的な限界値
        }
    }

    fn process_sample(&mut self, input: f32, gain: f32, tight: f32, bright: f32, ch: usize) -> f32 {
        // --- 1. PRE-FILTERING (Grid Stopper / Coupling Cap) ---
        // Tightを上げると低域がタイトになり、歪みが明瞭になる
        let hp_coef = 0.02 + (tight * 0.8).powf(2.0) * 0.4;
        let filtered = input - self.low_cut_state[ch];
        self.low_cut_state[ch] = input * hp_coef + self.low_cut_state[ch] * (1.0 - hp_coef);

        // --- 2. DYNAMIC BIAS SHIFT (Sagging / Rectification) ---
        // 信号が強いほどバイアスがマイナスに振れ、非対称性が増す「粘り」の正体
        let target_bias = -(filtered.abs() * 0.4 * gain);
        self.bias_env[ch] = self.bias_env[ch] * 0.99 + target_bias * 0.01;

        let drive = gain * 8.0;

        // Stage 1: プリアンプ初段（キャラクター決定）
        let mut x = self.tube_sample(filtered * drive, self.bias_env[ch]);

        // Inter-stage: 1段目と2段目の間で軽く高域を落とす
        x = x * 0.8 + self.prev_in[ch] * 0.2;

        // Stage 2: パワー管のクリップ感を追加
        x = self.tube_sample(x * 2.0, self.bias_env[ch] * 0.5);

        // --- 4. POST-EQ (Tone Stack / Bright) ---
        // 高域の抜けを調整
        let b_cutoff = 0.05 + (bright * 0.4);
        let bright_out = x + b_cutoff * (x - self.high_shelf_state[ch]);
        self.high_shelf_state[ch] = x;
        x = bright_out;

        // --- 5. DC BLOCKER ---
        let out = x - self.prev_in[ch] + (0.995 * self.prev_out[ch]);
        self.prev_in[ch] = x;
        self.prev_out[ch] = out;

        // 出力ゲイン調整（歪ませても音量が跳ね上がりすぎないように）
        out * (0.5 / (1.0 + gain * 0.5))
    }
}

impl XrossGainProcessor for XrossCrunchSystem {
    fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        let num_channels = layout.main_output_channels.map(|n| n.get()).unwrap_or(2) as usize;
        self.prev_in = vec![0.0; num_channels];
        self.prev_out = vec![0.0; num_channels];
        self.low_cut_state = vec![0.0; num_channels];
        self.high_shelf_state = vec![0.0; num_channels];
        self.bias_env = vec![0.0; num_channels];
        true
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        let gain = self.params.gain.value();
        let tight = self.params.tight.value();
        let bright = self.params.bright.value();

        for sample in slice {
            *sample = self.process_sample(*sample, gain, tight, bright, ch_idx);
        }
    }
}
