use crate::{params::MetalXrossParams, utils::DbToLinear};
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossLevelSystem {
    params: Arc<MetalXrossParams>,
    // 前段ブースター
    pre_boost: f32,
    sc_hp_state: Vec<f32>, // サイドチェイン用HPF

    // 後段リミッター (Look-ahead用)
    post_reduction: f32,
    delay_buffer: Vec<Vec<f32>>,
    delay_idx: usize,
    delay_len: usize,

    sample_rate: f32,
}

impl XrossLevelSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            pre_boost: 1.0,
            sc_hp_state: vec![0.0; 2],
            post_reduction: 1.0,
            delay_buffer: vec![vec![0.0; 256]; 2],
            delay_idx: 0,
            delay_len: 0,
            sample_rate: 44100.0,
        }
    }

    pub fn initialize(&mut self, sample_rate: f32, num_channels: usize) {
        self.sample_rate = sample_rate;
        self.delay_len = (sample_rate * 0.002).max(1.0) as usize; // 最低1サンプル
        self.delay_buffer = vec![vec![0.0; self.delay_len]; num_channels];
        self.sc_hp_state = vec![0.0; num_channels];
        self.delay_idx = 0;
    }

    // --- Pre-Booster ---
    pub fn pre_process(&mut self, buffer: &mut Buffer) {
        // パラメーター（dB）をリニア倍率に変換
        let target_gain = self.params.general.input.gain.value().db_to_gain();
        let limit_linear = self.params.general.input.limit.value().db_to_gain();

        let attack_coef = (-1.0 / (self.sample_rate * 5.0 / 1000.0)).exp();
        let release_coef = (-1.0 / (self.sample_rate * 150.0 / 1000.0)).exp();

        let (num_samples, num_channels) = (buffer.samples(), buffer.channels());

        for i in 0..num_samples {
            let mut max_sc = 1e-6f32;
            for ch in 0..num_channels {
                let s = buffer.as_slice()[ch][i];
                // HPFで低域の揺れによる誤作動を防止
                let hp = s - self.sc_hp_state[ch];
                self.sc_hp_state[ch] = s * 0.1 + self.sc_hp_state[ch] * 0.9;
                max_sc = max_sc.max(hp.abs());
            }

            // リニア値でリミッティング計算
            let target_boost = (limit_linear / max_sc).min(target_gain).max(0.001);

            let coef = if target_boost < self.pre_boost {
                attack_coef
            } else {
                release_coef
            };
            self.pre_boost = target_boost + coef * (self.pre_boost - target_boost);

            for ch in 0..num_channels {
                buffer.as_slice()[ch][i] *= self.pre_boost;
            }
        }
    }

    // --- Post-Limiter ---
    pub fn post_process(&mut self, buffer: &mut Buffer) {
        // パラメーター（dB）をリニア倍率に変換
        let output_gain = self.params.general.output.gain.value().db_to_gain();
        // Ceilingは 0dB (1.0) を超えないように制限
        let ceiling = self
            .params
            .general
            .output
            .limit
            .value()
            .db_to_gain()
            .min(1.0);

        let attack_coef = (-1.0 / (self.sample_rate * 1.5 / 1000.0)).exp();
        let release_coef = (-1.0 / (self.sample_rate * 80.0 / 1000.0)).exp();

        let (num_samples, num_channels) = (buffer.samples(), buffer.channels());

        for i in 0..num_samples {
            let mut max_peak = 1e-6f32;
            for ch in 0..num_channels {
                // 入力にアウトプットゲインを適用
                let input_sample = buffer.as_slice()[ch][i] * output_gain;

                let delayed_sample = self.delay_buffer[ch][self.delay_idx];
                self.delay_buffer[ch][self.delay_idx] = input_sample;

                // 未来のピークを検知
                max_peak = max_peak.max(input_sample.abs());
                buffer.as_slice()[ch][i] = delayed_sample;
            }

            let target_red = if max_peak > ceiling {
                ceiling / max_peak
            } else {
                1.0
            };

            let coef = if target_red < self.post_reduction {
                attack_coef
            } else {
                release_coef
            };
            self.post_reduction = target_red + coef * (self.post_reduction - target_red);

            for ch in 0..num_channels {
                buffer.as_slice()[ch][i] *= self.post_reduction;
            }

            self.delay_idx = (self.delay_idx + 1) % self.delay_len;
        }
    }
}
