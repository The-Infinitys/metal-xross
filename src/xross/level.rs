use crate::params::MetalXrossParams;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossLevelSystem {
    params: Arc<MetalXrossParams>,
    // 前段ブースター用のゲイン状態
    pre_boost: f32,
    // 後段リミッター用のゲイン状態
    post_reduction: f32,
    sample_rate: f32,
}

impl XrossLevelSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            pre_boost: 1.0,
            post_reduction: 1.0,
            sample_rate: 44100.0,
        }
    }

    pub fn initialize(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.pre_boost = 1.0;
        self.post_reduction = 1.0;
    }

    // --- Pre: ブースター (サステインを稼ぐ、1倍以下にしない) ---
    pub fn pre_process(&mut self, buffer: &mut Buffer) {
        let target_gain = self.params.general.input.gain.value();
        let limit = self.params.general.input.limit.value();

        let attack_coef = (-1.0 / (self.sample_rate * 10.0 / 1000.0)).exp(); // 10ms
        let release_coef = (-1.0 / (self.sample_rate * 200.0 / 1000.0)).exp(); // 200ms

        let (num_samples, num_channels) = (buffer.samples(), buffer.channels());

        for i in 0..num_samples {
            let mut max_in = 1e-6f32;
            for ch in 0..num_channels {
                max_in = max_in.max(buffer.as_slice()[ch][i].abs());
            }

            // 小さい音ほど持ち上げる (ターゲットは1.0〜target_gain)
            let target_boost = (limit / max_in).min(target_gain).max(1.0);

            // スムージング
            if target_boost > self.pre_boost {
                self.pre_boost = target_boost + attack_coef * (self.pre_boost - target_boost);
            } else {
                self.pre_boost = target_boost + release_coef * (self.pre_boost - target_boost);
            }

            for ch in 0..num_channels {
                buffer.as_slice()[ch][i] *= self.pre_boost;
            }
        }
    }

    // --- Post: リミッター (0dBを超えないように抑える、1倍以上にしない) ---
    pub fn post_process(&mut self, buffer: &mut Buffer) {
        let output_gain = self.params.general.output.gain.value();
        let ceiling = self.params.general.output.limit.value().min(0.99); // 安全のため0.99

        let attack_coef = (-1.0 / (self.sample_rate * 1.0 / 1000.0)).exp(); // 1ms (速攻)
        let release_coef = (-1.0 / (self.sample_rate * 100.0 / 1000.0)).exp(); // 100ms

        let (num_samples, num_channels) = (buffer.samples(), buffer.channels());

        for i in 0..num_samples {
            let mut max_in = 1e-6f32;
            for ch in 0..num_channels {
                // 出力ゲイン適用後のレベルで判定
                max_in = max_in.max(buffer.as_slice()[ch][i].abs() * output_gain);
            }

            // 1.0を超えている場合のみ減衰させる (targetは 0.x〜1.0)
            let target_red = if max_in > ceiling {
                ceiling / max_in
            } else {
                1.0
            };

            // スムージング
            if target_red < self.post_reduction {
                self.post_reduction = target_red + attack_coef * (self.post_reduction - target_red);
            } else {
                self.post_reduction =
                    target_red + release_coef * (self.post_reduction - target_red);
            }

            let final_gain = output_gain * self.post_reduction;
            for ch in 0..num_channels {
                let mut s = buffer.as_slice()[ch][i] * final_gain;
                // 最終安全装置
                if s.abs() > 0.999 {
                    s = s.signum() * 0.999;
                }
                buffer.as_slice()[ch][i] = s;
            }
        }
    }
}
