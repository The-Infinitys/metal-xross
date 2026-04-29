use super::XrossGainProcessor;
use crate::params::MetalXrossParams;
use std::sync::Arc;

#[derive(Default, Clone)]
struct DriveChannelState {
    low_cut: f32,
    lpf_state: f32,
    os_prev_sub: f32,
    dc_blocker: f32,
    sag_state: f32,
    drive_feedback: f32, // 粘りのための巡回成分
}

pub struct XrossDriveSystem {
    params: Arc<MetalXrossParams>,
    states: Vec<DriveChannelState>,
    sample_rate: f32,
}

impl XrossDriveSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            states: Vec::new(),
            sample_rate: 44100.0,
        }
    }

    /// 非対称ソフトサチュレーター（真空管のグリッド歪みを再現）
    #[inline(always)]
    fn tube_shaper(x: f32, bias: f32) -> f32 {
        let v = x + bias;
        if v > 0.0 {
            // 正相：滑らかな飽和
            (v * 1.2).tanh()
        } else {
            // 負相：少し沈み込みの深い非対称な圧縮
            let s = v * 0.9;
            s / (1.0 + s.abs())
        }
    }

    fn drive_core(
        state: &mut DriveChannelState,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
    ) -> f32 {
        // 1. INPUT PREP (Tight & Punchy)
        // Style Low が低いほど低域をカットしてタイトにする
        let hp_freq = 0.015 + (1.0 - s_low).powi(2) * 0.12;
        state.low_cut += hp_freq * (input - state.low_cut);
        let mut x = input - state.low_cut;

        // 2. POWER SAG (動的なコンプレッション)
        let target_sag = 1.0 - (x.abs() * 0.45 * gain).min(0.5);
        // リカバリーを少し遅くして「重み」を出す
        let sag_coeff = if target_sag < state.sag_state {
            0.15
        } else {
            0.03
        };
        state.sag_state += (target_sag - state.sag_state) * sag_coeff;

        // 3. BOOST STAGE
        let drive_amount = 5.0 + (gain * 50.0);
        x *= drive_amount * state.sag_state;

        // 4. CASCADE SATURATION & FEEDBACK
        let bias = 0.1 + (s_mid * 0.2);
        x = Self::tube_shaper(x, bias);

        // 内部フィードバック：ドライブ感に「うねり」と「粘り」を付与
        let fb_amount = 0.25 + (s_low * 0.3);
        x = x * 0.7 + state.drive_feedback * fb_amount;
        // フィードバックが発散しないようクリップ
        state.drive_feedback = x.clamp(-1.5, 1.5);

        // 2段目：ミッドレンジの強調
        let mid_focus = 1.0 + s_mid * 1.8;
        x = (x * mid_focus).tanh();

        // 5. TONE SHAPING (LPF)
        let lp_freq = 0.1 + (s_high * 0.55);
        state.lpf_state += lp_freq * (x - state.lpf_state);
        let out = state.lpf_state;

        // 6. DC BLOCK
        let dc_fix = out - state.dc_blocker;
        state.dc_blocker = out * 0.005 + state.dc_blocker * 0.995;

        dc_fix
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
            let sub = state.os_prev_sub + (input - state.os_prev_sub) * fraction;
            output_sum += Self::drive_core(state, sub, g, sl, sm, sh);
        }
        state.os_prev_sub = input;

        // メイクアップゲインを適用
        (output_sum * inv_os) * 1.25
    }
}

impl XrossGainProcessor for XrossDriveSystem {
    fn initialize(&mut self, sample_rate: f32, num_channels: usize) {
        self.sample_rate = sample_rate;
        self.states = vec![DriveChannelState::default(); num_channels];
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
