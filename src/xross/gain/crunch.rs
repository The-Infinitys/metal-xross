use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossCrunchSystem {
    params: Arc<MetalXrossParams>,
    // チャンネルごとの状態保持
    low_cut_state: Vec<f32>,
    high_shelf_state: Vec<f32>,
    prev_x: Vec<f32>,
    prev_y: Vec<f32>,
    bias_env: Vec<f32>,
}

impl XrossCrunchSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            low_cut_state: vec![0.0; 2],
            high_shelf_state: vec![0.0; 2],
            prev_x: vec![0.0; 2],
            prev_y: vec![0.0; 2],
            bias_env: vec![0.0; 2],
        }
    }

    /// 真空管の非対称な飽和特性
    fn tube_sample(&self, x: f32, bias: f32) -> f32 {
        let input = x + bias;
        if input > 0.0 {
            // 正相：非常にソフトなサチュレーション
            input.tanh()
        } else {
            // 逆相：グリッド電流によるクリッピングを模し、より早く、深く歪む
            // 係数を調整して、マイナス側の波形をより平坦にする
            let out = (input * 1.8).atan() * 0.8;
            out.max(-0.98)
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
        // --- 0. Gain 0.0の時は完全なクリーン ---
        if gain < 0.001 {
            return input;
        }

        // --- 1. PRE-FILTERING (Style Low: Tightness) ---
        // Style Lowが低いほど、低域をカットして明瞭な「クランチ」にする
        let hp_freq = (1.0 - s_low).powf(2.0) * 0.15;
        let filtered = input - self.low_cut_state[ch];
        self.low_cut_state[ch] = input * hp_freq + self.low_cut_state[ch] * (1.0 - hp_freq);

        // --- 2. DRIVE SETTINGS ---
        // 入力のピークが0.8想定。Gainが上がるにつれてDriveを増幅
        let drive_amount = gain * 4.0 * (1.0 + s_mid * 0.5);
        let dry_mix = 1.0 - (gain * 0.8).min(0.9); // Gain 0付近で芯を残すためのDryミックス

        // --- 3. DYNAMIC BIAS (Sagging) ---
        // 信号の強さに応じて、動作点をマイナス側にずらす
        let target_bias = -(filtered.abs() * 0.25 * gain);
        self.bias_env[ch] = self.bias_env[ch] * 0.995 + target_bias * 0.005;

        // --- 4. MULTI-STAGE SATURATION ---
        // Stage 1: Soft Drive
        let stage1 = self.tube_sample(filtered * drive_amount, self.bias_env[ch]);

        // Stage 2: さらに少しだけ飽和させ、音の太さを出す
        let stage2 = self.tube_sample(stage1 * 1.2, self.bias_env[ch] * 0.5);

        // --- 5. POST-FILTERING (Style High: Clarity) ---
        // 歪みで発生した高域の「ジリジリ」をStyle Highでコントロール
        let lp_freq = 0.2 + (s_high * 0.6);
        let bright_out = stage2 * lp_freq + self.high_shelf_state[ch] * (1.0 - lp_freq);
        self.high_shelf_state[ch] = bright_out;

        // --- 6. OUTPUT MAPPING ---
        // Gain 0.0で1.0倍、Gain 1.0で音量バランスを保つための自動補正
        let makeup_gain = 1.0 / (1.0 + gain * 0.5);
        let mixed = (bright_out * (1.0 - dry_mix) + filtered * dry_mix) * makeup_gain;

        // DCカット (バイアス移動によるブツブツ音防止)
        let out = mixed - self.prev_x[ch] + (0.997 * self.prev_y[ch]);
        self.prev_x[ch] = mixed;
        self.prev_y[ch] = out;

        out
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
        self.low_cut_state = vec![0.0; num_channels];
        self.high_shelf_state = vec![0.0; num_channels];
        self.prev_x = vec![0.0; num_channels];
        self.prev_y = vec![0.0; num_channels];
        self.bias_env = vec![0.0; num_channels];
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
