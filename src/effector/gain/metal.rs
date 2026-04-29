use super::XrossGainProcessor;
use crate::params::MetalXrossParams;
use std::f32::consts::PI;
use std::sync::Arc;

/// 2次フィルタ（Biquad）の状態保持用
#[derive(Default, Clone)]
struct Biquad {
    z1: f32,
    z2: f32,
}

impl Biquad {
    // ダイレクトフォームII転置形式
    #[inline(always)]
    fn process(&mut self, input: f32, a1: f32, a2: f32, b0: f32, b1: f32, b2: f32) -> f32 {
        let out = b0 * input + self.z1;
        self.z1 = b1 * input - a1 * out + self.z2;
        self.z2 = b2 * input - a2 * out;
        out
    }
}

#[derive(Default, Clone)]
struct MetalChannelState {
    pre_hp: f32,
    slew_state: f32,
    dc_block: f32,
    envelope: f32,
    prev_input: f32,
    low_resonance: f32,
    post_tight: f32,
    feedback_state: f32,
    os_lpf_biquad: Biquad,
}

pub struct XrossMetalSystem {
    params: Arc<MetalXrossParams>,
    states: Vec<MetalChannelState>,
    sample_rate: f32,
}

impl XrossMetalSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            states: Vec::new(),
            sample_rate: 44100.0,
        }
    }

    #[inline(always)]
    fn drive_core(
        state: &mut MetalChannelState,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
    ) -> f32 {
        let env = state.envelope;

        // 1. DYNAMIC PRE-HP (ピッキング時のタイトさ)
        let hp_freq = 0.05 + (1.0 - s_low) * 0.18 + (env * 0.25);
        state.pre_hp += hp_freq * (input - state.pre_hp);
        let mut x = input - state.pre_hp;

        // 2. GAIN STAGING
        // 指数関数的なゲインカーブで強烈なハイゲインを実現
        let noise_gate_scale = (env * 25.0).min(1.0).powf(1.1);
        let drive_amt = ((gain * 7.8).exp() * 15.0) * noise_gate_scale;
        x *= drive_amt;

        // 3. MULTI-STAGE SATURATION (粘りと絡み)
        x += state.feedback_state * 0.25;

        // 非対称サチュレーション
        x = if x > 0.0 {
            (x * 1.3).tanh()
        } else {
            (x * 1.15).tanh() * 0.97
        };

        // Style Mid Scoop (s_midが低いほどドンシャリ、高いほど中域を盛る)
        let mid_scoop = (0.5 - s_mid).max(0.0) * 0.8;
        if mid_scoop > 0.0 {
            x -= (x - x.powi(3)) * mid_scoop;
        }

        // Style High に連動するエッジの硬さ
        let soft_out = (x * 1.2).atan() * 0.85;
        let hard_limit = 0.85 - (s_high * 0.3);
        let hard_out = x.clamp(-hard_limit, hard_limit);

        let square_mix = s_high * 0.6;
        x = (soft_out * (1.0 - square_mix)) + (hard_out * square_mix);

        state.feedback_state = x;

        // 4. POST-PROCESSING
        let lpf_cutoff = 0.25 + (s_high * 0.55);
        state.post_tight += lpf_cutoff * (x - state.post_tight);
        x = state.post_tight;

        // Low Resonance (キャビネットの鳴りやミュートの重厚感)
        let low_boost = s_low * 0.65 * (1.0 - env.min(0.8));
        state.low_resonance += 0.15 * (x - state.low_resonance);
        x += state.low_resonance * low_boost;

        // 5. SLEW RATE (アタックのトゲ調整)
        let max_step = 0.04 + (s_high * 0.9);
        let diff = x - state.slew_state;
        state.slew_state += diff.clamp(-max_step, max_step);

        state.slew_state
    }

    fn process_sample(&mut self, input: f32, ch_idx: usize) -> f32 {
        let g = self.params.gain.value();
        let sl = self.params.style_low.value();
        let sm = self.params.style_mid.value();
        let sh = self.params.style_high.value();

        let state = &mut self.states[ch_idx];

        // ゲイン量に応じてオーバーサンプリング倍数を可変 (負荷軽減)
        let os_factor = if g < 0.25 {
            1
        } else if g < 0.55 {
            2
        } else {
            4
        };
        let inv_os = 1.0 / os_factor as f32;

        state.envelope += (input.abs() - state.envelope) * 0.3;

        let mut output_sum = 0.0;
        for i in 0..os_factor {
            let fraction = i as f32 * inv_os;
            let sub_sample = state.prev_input + (input - state.prev_input) * fraction;
            output_sum += Self::drive_core(state, sub_sample, g, sl, sm, sh);
        }
        state.prev_input = input;

        let raw_out = output_sum * inv_os;

        // 2次 Butterworth LPF によるエイリアシング除去 (約17kHz)
        let (a1, a2, b0, b1, b2) = Self::calculate_biquad_lpf(self.sample_rate, 17000.0);
        let filtered_out = state.os_lpf_biquad.process(raw_out, a1, a2, b0, b1, b2);

        // DC Block
        let dc_fix = filtered_out - state.dc_block;
        state.dc_block = filtered_out + 0.997 * (state.dc_block - filtered_out);

        dc_fix * 0.7
    }

    fn calculate_biquad_lpf(sample_rate: f32, cutoff: f32) -> (f32, f32, f32, f32, f32) {
        let ff = (cutoff / sample_rate).min(0.45);
        let omega = 2.0 * PI * ff;
        let sn = omega.sin();
        let cs = omega.cos();
        let alpha = sn / (2.0f32).sqrt();

        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cs / a0;
        let a2 = (1.0 - alpha) / a0;
        let b1 = (1.0 - cs) / a0;
        let b0 = b1 * 0.5;
        let b2 = b0;

        (a1, a2, b0, b1, b2)
    }
}

impl XrossGainProcessor for XrossMetalSystem {
    fn initialize(&mut self, sample_rate: f32, num_channels: usize) {
        self.sample_rate = sample_rate;
        self.states = vec![MetalChannelState::default(); num_channels];
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        if ch_idx >= self.states.len() {
            return;
        }

        for sample in slice {
            *sample = self.process_sample(*sample, ch_idx);
        }
    }
}
