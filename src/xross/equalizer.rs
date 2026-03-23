use crate::MetalXross;
use crate::params::{MetalXrossParams, PeqBandParams};
use nih_plug::prelude::*;
use std::f32::consts::PI;
use std::sync::Arc;

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
    // チャンネルごとの状態（ステレオ前提だが可変に対応可能）
    states: Vec<[BiquadState; 3]>, // 0:Low, 1:Mid, 2:High
}

impl XrossEqualizer {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            // 初期値は空。processの最初の呼び出しでチャンネル数に合わせてリサイズされる
            states: Vec::new(),
        }
    }

    fn calculate_coeffs(band: &PeqBandParams, f_type: FilterType, sr: f32) -> Coeffs {
        let f0 = band.freq.value();
        let gain_db = band.gain.value();
        let q = band.q.value();

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
        let output = c.b0 * input + s.z1;
        s.z1 = c.b1 * input - c.a1 * output + s.z2;
        s.z2 = c.b2 * input - c.a2 * output;
        output
    }

    pub fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<MetalXross>,
    ) {
        let sr = context.transport().sample_rate;
        let num_channels = buffer.channels();

        // チャンネル数に合わせてステートをリサイズ
        if self.states.len() != num_channels {
            self.states.resize(
                num_channels,
                [
                    BiquadState::default(),
                    BiquadState::default(),
                    BiquadState::default(),
                ],
            );
        }

        let c_low = Self::calculate_coeffs(&self.params.eq.low, FilterType::LowShelf, sr);
        let c_mid = Self::calculate_coeffs(&self.params.eq.mid, FilterType::Peaking, sr);
        let c_high = Self::calculate_coeffs(&self.params.eq.high, FilterType::HighShelf, sr);

        // buffer.as_slice() は &[&mut [f32]] を返すので、
        // .iter_mut() を使うと各要素が &mut &mut [f32] になってしまいます。
        // シンプルに enumerate() で回すのが確実です。
        for (channel_idx, samples) in buffer.as_slice().iter_mut().enumerate() {
            let channel_state = &mut self.states[channel_idx];

            // samples は &mut [f32] なので、さらに iter_mut() でサンプルを取り出します
            for sample in samples.iter_mut() {
                let mut x = *sample;

                x = Self::process_sample(x, &c_low, &mut channel_state[0]);
                x = Self::process_sample(x, &c_mid, &mut channel_state[1]);
                x = Self::process_sample(x, &c_high, &mut channel_state[2]);

                *sample = x;
            }
        }
    }
}
