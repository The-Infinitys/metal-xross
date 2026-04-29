use crate::params::MetalXrossParams;
use std::f32::consts::PI;
use std::sync::Arc;
use truce::prelude::*;

pub enum FilterType {
    LowShelf,
    Peaking,
    HighShelf,
}

#[derive(Default, Clone)]
struct BiquadState {
    z1: f32,
    z2: f32,
}

struct Coeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

pub struct XrossEqualizer {
    params: Arc<MetalXrossParams>,
    // チャンネルごとの状態（0:Low, 1:Mid, 2:High）
    states: Vec<[BiquadState; 3]>,
    sample_rate: f32,
}

impl XrossEqualizer {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            states: Vec::new(),
            sample_rate: 44100.0,
        }
    }

    pub fn initialize(&mut self, sample_rate: f32, num_channels: usize) {
        self.sample_rate = sample_rate;
        self.states = vec![
            [
                BiquadState::default(),
                BiquadState::default(),
                BiquadState::default(),
            ];
            num_channels
        ];
    }

    /// 特定のバンドの係数を計算
    fn calculate_coeffs(f0: f32, gain_db: f32, q: f32, f_type: FilterType, sr: f32) -> Coeffs {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * f0 / sr;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match f_type {
            FilterType::LowShelf => {
                let ap = a + 1.0;
                let am = a - 1.0;
                let s = 2.0 * a.sqrt() * alpha;
                (
                    a * (ap - am * cos_w0 + s),
                    2.0 * a * (am - ap * cos_w0),
                    a * (ap - am * cos_w0 - s),
                    ap + am * cos_w0 + s,
                    -2.0 * (am + ap * cos_w0),
                    ap + am * cos_w0 - s,
                )
            }
            FilterType::Peaking => (
                1.0 + alpha * a,
                -2.0 * cos_w0,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos_w0,
                1.0 - alpha / a,
            ),
            FilterType::HighShelf => {
                let ap = a + 1.0;
                let am = a - 1.0;
                let s = 2.0 * a.sqrt() * alpha;
                (
                    a * (ap + am * cos_w0 + s),
                    -2.0 * a * (am + ap * cos_w0),
                    a * (ap + am * cos_w0 - s),
                    ap - am * cos_w0 + s,
                    2.0 * (am - ap * cos_w0),
                    ap - am * cos_w0 - s,
                )
            }
        };

        Coeffs {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    #[inline(always)]
    fn process_sample(input: f32, c: &Coeffs, s: &mut BiquadState) -> f32 {
        // Direct Form II Transposed
        let output = c.b0 * input + s.z1;
        s.z1 = c.b1 * input - c.a1 * output + s.z2;
        s.z2 = c.b2 * input - c.a2 * output;
        output
    }

    pub fn process_buffer(&mut self, buffer: &mut AudioBuffer) {
        let num_channels = buffer.channels();
        let num_samples = buffer.num_samples();

        if self.states.len() != num_channels {
            self.initialize(self.sample_rate, num_channels);
        }

        // パラメータを事前に取得して係数を一括計算（サンプルループ外）
        let c_low = Self::calculate_coeffs(
            self.params.eq_lo_freq.value(),
            self.params.eq_lo_gain.value(),
            self.params.eq_lo_q.value(),
            FilterType::LowShelf,
            self.sample_rate,
        );
        let c_mid = Self::calculate_coeffs(
            self.params.eq_mi_freq.value(),
            self.params.eq_mi_gain.value(),
            self.params.eq_mi_q.value(),
            FilterType::Peaking,
            self.sample_rate,
        );
        let c_high = Self::calculate_coeffs(
            self.params.eq_hi_freq.value(),
            self.params.eq_hi_gain.value(),
            self.params.eq_hi_q.value(),
            FilterType::HighShelf,
            self.sample_rate,
        );

        for ch in 0..num_channels {
            let channel_state = &mut self.states[ch];
            for i in 0..num_samples {
                let (_, out) = buffer.io(ch);
                let mut x = out[i];

                // 3バンドを直列に処理
                x = Self::process_sample(x, &c_low, &mut channel_state[0]);
                x = Self::process_sample(x, &c_mid, &mut channel_state[1]);
                x = Self::process_sample(x, &c_high, &mut channel_state[2]);

                out[i] = x;
            }
        }
    }
}
