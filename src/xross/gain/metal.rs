use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossMetalSystem {
    params: Arc<MetalXrossParams>,
    ap_state: Vec<f32>,
    pre_hp: Vec<f32>,
    stage_states: Vec<[f32; 3]>, // 多段処理用
    slew_state: Vec<f32>,        // スルーレート制御用
    dc_block: Vec<f32>,
    prev_input: Vec<f32>, // アップサンプリング補間用
}

impl XrossMetalSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            ap_state: vec![0.0; 2],
            pre_hp: vec![0.0; 2],
            stage_states: vec![[0.0; 3]; 2],
            slew_state: vec![0.0; 2],
            dc_block: vec![0.0; 2],
            prev_input: vec![0.0; 2],
        }
    }

    fn drive_core(
        &mut self,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
        ch: usize,
    ) -> f32 {
        // 1. TIGHTENING & BRIGHTENING (Pre-Filtering)
        // 歪みに入る前に高域を少し持ち上げる（Style Highに連動）
        let brightness = s_high * 0.4;
        let pre_emphasized = input + (input - self.pre_hp[ch]) * brightness;

        // 低域カット（s_low 0.0で超絶タイト、1.0でドロドロ）
        let hp_freq = 0.15 + (1.0 - s_low).powf(1.5) * 0.45;
        let filtered = pre_emphasized - self.pre_hp[ch];
        self.pre_hp[ch] = pre_emphasized * hp_freq + self.pre_hp[ch] * (1.0 - hp_freq);

        // 2. HARDER CLIPPING (より鋭いエッジ)
        let total_gain = 30.0 + gain * 140.0; // ゲイン上限をさらにアップ
        let mut x = filtered * total_gain;

        // 非対称性を減らし、矩形波（四角）に近づけることで「芯」を出す
        // 0.05までリニア領域を狭める
        let metal_shaper = |val: f32, h: f32| {
            let abs_v = val.abs();
            if abs_v < 0.05 {
                val
            } else {
                val.signum() * (0.05 + (1.0 - 0.05) * (1.0 - (-h * (abs_v - 0.05)).exp()))
            }
        };

        x = metal_shaper(x, 2.0 + s_mid); // Stage 1
        x = metal_shaper(x * 2.0, 6.0); // Stage 2: ここで完全に壁を作る

        // 3. SLEW RATE (エッジの鋭さを解放)
        // 丸まっていた原因：max_step を大幅に引き上げ、高域をスルーさせる
        let max_step = 0.2 + s_high * 0.8; // 以前より4倍近く速い変化を許容
        let diff = x - self.slew_state[ch];
        self.slew_state[ch] += diff.clamp(-max_step, max_step);
        x = self.slew_state[ch];

        // 4. POST-FILTER (Razor Edge)
        // Style Highが最大なら、ほぼフィルタを通さない（全開）にする
        let lp_freq = 0.1 + (s_high.powf(1.2) * 0.85);
        let out = x * lp_freq + self.stage_states[ch][1] * (1.0 - lp_freq);
        self.stage_states[ch][1] = out;

        out
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
        // 高ゲイン時はエイリアシングノイズを避けるため4倍、低ゲインでも2倍OS
        let os_factor = if gain > 0.6 { 4 } else { 2 };

        let mut output = 0.0;
        let step = (input - self.prev_input[ch]) / os_factor as f32;
        let mut current_input = self.prev_input[ch];

        for _ in 0..os_factor {
            current_input += step; // 線形補間によるアップサンプリング
            output += self.drive_core(current_input, gain, s_low, s_mid, s_high, ch);
        }

        self.prev_input[ch] = input;
        output /= os_factor as f32;

        // DC Block (低周波の揺れをカット)
        let final_out = output - self.dc_block[ch] + (0.995 * self.dc_block[ch]);
        self.dc_block[ch] = output;

        // 最終メイクアップ: 圧倒的な音圧だが歪みの芯を残す
        final_out * 0.45
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
        self.ap_state = vec![0.0; num_channels];
        self.pre_hp = vec![0.0; num_channels];
        self.stage_states = vec![[0.0; 3]; num_channels];
        self.slew_state = vec![0.0; num_channels];
        self.dc_block = vec![0.0; num_channels];
        self.prev_input = vec![0.0; num_channels];
        true
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        let g = self.params.general.gain.value();
        let sl = self.params.style.low.value();
        let sm = self.params.style.mid.value();
        let sh = self.params.style.high.value();

        for sample in slice {
            *sample = self.process_sample(*sample, g, sl, sm, sh, ch_idx);
        }
    }
}
