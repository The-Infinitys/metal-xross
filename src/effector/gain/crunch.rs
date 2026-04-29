use super::XrossGainProcessor;
use crate::params::MetalXrossParams;
use std::sync::Arc;

#[derive(Default, Clone)]
struct CrunchChannelState {
    low_cut: f32,
    lpf_state: f32,
    dc_block: f32,
    sag_state: f32,
    prev_input: f32,
    bias_drift: f32,
}

pub struct XrossCrunchSystem {
    params: Arc<MetalXrossParams>,
    states: Vec<CrunchChannelState>,
    sample_rate: f32,
}

impl XrossCrunchSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            states: Vec::new(),
            sample_rate: 44100.0,
        }
    }

    /// 真空管特有の非対称飽和モデル
    /// 正相は tanh で滑らかに、逆相は sqrt ベースで少し硬めにクリップ
    #[inline(always)]
    fn tube_model(x: f32, bias: f32) -> f32 {
        let v = x + bias;
        if v > 0.0 {
            (v * 0.8).tanh() * 1.25
        } else {
            let s = v * 1.5;
            (s / (1.0 + s.powi(2)).sqrt()) * 0.8
        }
    }

    fn drive_core(
        state: &mut CrunchChannelState,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
    ) -> f32 {
        // 1. PRE-FILTERING (タイトな低域とミッドの押し)
        let hp_freq = 0.01 + (1.0 - s_low).powi(2) * 0.08;
        state.low_cut += hp_freq * (input - state.low_cut);
        let mut x = (input - state.low_cut) * (1.0 + s_mid * 1.5);

        // 2. DYNAMIC BIAS & SAG
        // 入力信号の強さに応じて電源電圧が落ちる(Sag)挙動
        let env = x.abs();
        state.bias_drift += (env * 0.2 - state.bias_drift) * 0.01;

        // Sag のリカバリーを少し遅くして「粘り」を出す
        let sag_target = (1.0 - env * 0.4 * gain).max(0.4);
        let sag_coeff = if sag_target < state.sag_state {
            0.1
        } else {
            0.02
        };
        state.sag_state += (sag_target - state.sag_state) * sag_coeff;

        // 3. MULTI-STAGE DRIVE
        let drive = 2.0 + (gain * 15.0);
        x *= drive * state.sag_state;

        // Stage 1: Pre-amp (Asymmetric bias)
        let bias1 = 0.12 + state.bias_drift + (s_mid * 0.15);
        x = Self::tube_model(x, bias1);

        // Stage 2: Power-amp (Heavy saturation)
        let bias2 = -0.08 - (s_low * 0.12);
        x = Self::tube_model(x * 1.6, bias2);

        // 4. POST-FILTERING
        // s_high に応じて高域の「ジリジリ感」を調整
        let lp_freq = 0.05 + (s_high * 0.5);
        state.lpf_state += lp_freq * (x - state.lpf_state);

        // Make-up
        state.lpf_state * 1.5
    }

    fn process_sample(&mut self, input: f32, ch_idx: usize) -> f32 {
        let g = self.params.gain.value();
        let sl = self.params.style_low.value();
        let sm = self.params.style_mid.value();
        let sh = self.params.style_high.value();

        let state = &mut self.states[ch_idx];

        // 4x Oversampling
        let os_factor = 4;
        let inv_os = 1.0 / os_factor as f32;
        let mut output_sum = 0.0;

        for i in 0..os_factor {
            let fraction = i as f32 * inv_os;
            // 入力信号の線形補間
            let sub = state.prev_input + (input - state.prev_input) * fraction;
            output_sum += Self::drive_core(state, sub, g, sl, sm, sh);
        }
        state.prev_input = input;
        let out = output_sum * inv_os;

        // DC Block (0.5Hz @ 44.1kHz 相当)
        let dc_fix = out - state.dc_block;
        state.dc_block = out + 0.997 * (state.dc_block - out);

        dc_fix
    }
}

impl XrossGainProcessor for XrossCrunchSystem {
    fn initialize(&mut self, sample_rate: f32, num_channels: usize) {
        self.sample_rate = sample_rate;
        self.states = vec![CrunchChannelState::default(); num_channels];
        for state in &mut self.states {
            state.sag_state = 1.0;
        }
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
