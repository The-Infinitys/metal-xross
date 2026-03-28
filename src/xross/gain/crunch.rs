use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossCrunchSystem {
    params: Arc<MetalXrossParams>,
    // チャンネルごとの状態保持 (ステレオ対応)
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

    /// 真空管の非対称なカーブをシミュレート
    /// x: 入力, bias: グリッドバイアスのズレ
    fn tube_sample(&self, x: f32, bias: f32) -> f32 {
        let input = x + bias;
        if input > 0.0 {
            // 正相：柔らかく飽和 (Soft Saturation)
            input.tanh()
        } else {
            // 逆相：グリッドクリッピングを模した鋭い非対称性
            let out = (input * 0.6).atan() * 1.6;
            out.max(-0.95) // 物理的な限界値
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
        // --- 1. PRE-FILTERING (Style Low -> Tightness) ---
        // Style Lowが低い(0.0)ほど、Tight（ローカット）が強くなり、刻みがシャープになる
        let tight_amount = (1.0 - s_low).powf(1.5);
        let hp_coef = 0.01 + (tight_amount * 0.9).powf(2.0) * 0.5;
        let filtered = input - self.low_cut_state[ch];
        self.low_cut_state[ch] = input * hp_coef + self.low_cut_state[ch] * (1.0 - hp_coef);

        // --- 2. DYNAMIC BIAS SHIFT (Sagging / Rectification) ---
        // 信号が強いほどバイアスがマイナスに振れ、クランチ特有の「粘り」を出す
        let target_bias = -(filtered.abs() * 0.3 * gain);
        self.bias_env[ch] = self.bias_env[ch] * 0.99 + target_bias * 0.01;

        // --- 3. DRIVE STAGES (Style Mid -> Density) ---
        // Midを上げると歪みの密度が増し、中域が飽和しやすくなる
        let drive_boost = 1.0 + (s_mid * 0.6);
        let drive = gain * 6.0 * drive_boost;

        // Stage 1: プリアンプ初段
        let mut x = self.tube_sample(filtered * drive, self.bias_env[ch]);

        // Inter-stage: 高域のチリチリ感を抑える軽いLPF
        x = x * 0.85 + self.prev_in[ch] * 0.15;

        // Stage 2: パワー管のクリップ感を追加
        x = self.tube_sample(x * 1.5, self.bias_env[ch] * 0.8);

        // --- 4. POST-EQ (Style High -> Brightness) ---
        // Style Highに連動したシェルビングフィルタ
        let bright_amount = s_high;
        let b_cutoff = 0.02 + (bright_amount * 0.5);
        let bright_out = x + b_cutoff * (x - self.high_shelf_state[ch]);
        self.high_shelf_state[ch] = x;
        x = bright_out;

        // --- 5. DC BLOCKER ---
        // バイアス移動によるDCオフセットを除去
        let out = x - self.prev_in[ch] + (0.995 * self.prev_out[ch]);
        self.prev_in[ch] = x;
        self.prev_out[ch] = out;

        // 出力ゲイン調整（歪み量に応じた自動補正）
        out * (0.6 / (1.0 + gain * 0.4))
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
        let gain = self.params.general.gain.value();

        // Styleセクションの値を抽出
        let s_low = self.params.style.low.value();
        let s_mid = self.params.style.mid.value();
        let s_high = self.params.style.high.value();

        for sample in slice {
            *sample = self.process_sample(*sample, gain, s_low, s_mid, s_high, ch_idx);
        }
    }
}
