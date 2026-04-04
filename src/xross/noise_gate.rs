use crate::params::MetalXrossParams;
use crate::utils::DbToLinear;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossNoiseGate {
    sample_rate: f32,
    gate_gain: f32,
    hold_timer: i32,

    // 演奏判定用（pre）
    low_env: f32,
    mid_env: f32,
    high_env: f32,
    lp_pre: [f32; 2],
    hp_pre: [f32; 2],

    // 歪み後専用（post）← ここを大幅強化
    lp_post: [f32; 2],
    hp_post: [f32; 2],
    // post専用のエンベロープ（歪み後の実際のエネルギー追従）
    post_mid_env: f32,
    post_high_env: f32,

    params: Arc<MetalXrossParams>,
}

impl XrossNoiseGate {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            sample_rate: 44100.0,
            gate_gain: 1.0,
            hold_timer: 0,
            low_env: 0.0,
            mid_env: 0.0,
            high_env: 0.0,
            lp_pre: [0.0; 2],
            hp_pre: [0.0; 2],
            lp_post: [0.0; 2],
            hp_post: [0.0; 2],
            post_mid_env: 0.0,
            post_high_env: 0.0,
            params,
        }
    }

    pub fn initialize(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn process_pre(&mut self, buffer: &mut Buffer) {
        let threshold = self.params.noise_gate.threshold.value().db_to_linear();
        let release_ms = self.params.noise_gate.release.value();
        let release_coeff = (-1.0 / (release_ms * self.sample_rate / 1000.0 * 0.8)).exp(); // 少し速め

        for (ch_idx, channel_samples) in buffer.iter_samples().enumerate() {
            let ch = ch_idx % 2;
            for sample in channel_samples {
                let input = *sample;

                // pre帯域分割
                self.lp_pre[ch] += 0.085 * (input - self.lp_pre[ch]);
                let low = self.lp_pre[ch];
                let mid_high = input - low;
                self.hp_pre[ch] += 0.24 * (mid_high - self.hp_pre[ch]);
                let high = self.hp_pre[ch];
                let mid = mid_high - high;

                // preエンベロープ
                let atk = 0.65;
                let rel = 0.0015;
                let update = |env: &mut f32, val: f32| {
                    let v = val.abs();
                    if v > *env {
                        *env += atk * (v - *env);
                    } else {
                        *env += rel * (v - *env);
                    }
                };
                update(&mut self.low_env, low);
                update(&mut self.mid_env, mid);
                update(&mut self.high_env, high);

                let open_thresh = threshold;
                let is_playing = self.mid_env > open_thresh
                    || (self.low_env > open_thresh * 0.75 && self.high_env > open_thresh * 0.45);

                if is_playing {
                    self.hold_timer = (0.04 * self.sample_rate) as i32; // 40msホールド
                    self.gate_gain = (self.gate_gain + 0.22).min(1.0);
                } else if self.hold_timer > 0 {
                    self.hold_timer -= 1;
                } else {
                    self.gate_gain *= release_coeff;
                }
            }
        }
    }

    pub fn process_post(&mut self, buffer: &mut Buffer) {
        let tolerance = self.params.noise_gate.tolerance.value();

        for (ch_idx, channel_samples) in buffer.iter_samples().enumerate() {
            let ch = ch_idx % 2;
            for sample in channel_samples {
                let input = *sample;

                // post独立帯域分割
                self.lp_post[ch] += 0.085 * (input - self.lp_post[ch]);
                let low_part = self.lp_post[ch];
                let mid_high_part = input - low_part;
                self.hp_post[ch] += 0.24 * (mid_high_part - self.hp_post[ch]);
                let high_part = self.hp_post[ch];
                let mid_part = mid_high_part - high_part;

                // post専用エンベロープ（歪み後の実際のエネルギー）
                let post_atk = 0.7;
                let post_rel = 0.002;
                let update_post = |env: &mut f32, val: f32| {
                    let v = val.abs();
                    if v > *env {
                        *env += post_atk * (v - *env);
                    } else {
                        *env += post_rel * (v - *env);
                    }
                };
                update_post(&mut self.post_mid_env, mid_part);
                update_post(&mut self.post_high_env, high_part);

                // ハーモニック比率（pre + postをブレンドして頑丈に）
                let harmonic_ratio = ((self.mid_env + self.post_mid_env * 0.6) + 0.00001)
                    / ((self.high_env + self.post_high_env * 0.6) + 0.00001);

                let high_mask = (harmonic_ratio * (2.5 - tolerance * 1.8)).clamp(0.0, 1.0);

                let closing_factor = (1.0 - self.gate_gain).powi(2);

                let g_low = self.gate_gain.max(0.15 * self.gate_gain);
                let g_mid = self.gate_gain;
                let g_high = self.gate_gain * (1.0 - closing_factor * (1.0 - high_mask) * 1.65); // 高域削り強化

                let output = low_part * g_low + mid_part * g_mid + high_part * g_high;

                // 引き終わりノイズ残り対策：より急激にシャットアウト
                if self.gate_gain < 0.01 {
                    *sample = output * (self.gate_gain / 0.01).powi(4); // より急な減衰
                } else {
                    *sample = output;
                }
            }
        }
    }
}
