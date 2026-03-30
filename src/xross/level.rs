use crate::params::MetalXrossParams;
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
            delay_buffer: vec![vec![0.0; 256]; 2], // 約5ms分 (@48kHz)
            delay_idx: 0,
            delay_len: 0,
            sample_rate: 44100.0,
        }
    }

    pub fn initialize(&mut self, sample_rate: f32, num_channels: usize) {
        self.sample_rate = sample_rate;
        // ルックアヘッド時間を2msに設定
        self.delay_len = (sample_rate * 0.002) as usize;
        self.delay_buffer = vec![vec![0.0; self.delay_len]; num_channels];
        self.sc_hp_state = vec![0.0; num_channels];
        self.delay_idx = 0;
    }

    // --- Pre-Booster: サイドチェインHPF付き ---
    pub fn pre_process(&mut self, buffer: &mut Buffer) {
        let target_gain = self.params.general.input.gain.value();
        let limit = self.params.general.input.limit.value();

        // メタルはレスポンスが命なので速めに設定
        let attack_coef = (-1.0 / (self.sample_rate * 5.0 / 1000.0)).exp();
        let release_coef = (-1.0 / (self.sample_rate * 150.0 / 1000.0)).exp();

        let (num_samples, num_channels) = (buffer.samples(), buffer.channels());

        for i in 0..num_samples {
            let mut max_sc = 1e-6f32;
            for ch in 0..num_channels {
                let s = buffer.as_slice()[ch][i];
                // 100Hz以下の不要な振動を除去してレベル検出（ピッキングノイズ対策）
                let hp = s - self.sc_hp_state[ch];
                self.sc_hp_state[ch] = s * 0.1 + self.sc_hp_state[ch] * 0.9;
                max_sc = max_sc.max(hp.abs());
            }

            let target_boost = (limit / max_sc).min(target_gain).max(1.0);

            // ゲインが下がる時（アタック）は速く、上がる時（リリース）は遅く
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

    // --- Post-Limiter: Look-ahead（先読み）実装 ---
    pub fn post_process(&mut self, buffer: &mut Buffer) {
        let output_gain = self.params.general.output.gain.value();
        let ceiling = self.params.general.output.limit.value().min(0.99);

        // リミッターのアタックは非常に鋭く
        let attack_coef = (-1.0 / (self.sample_rate * 1.5 / 1000.0)).exp();
        let release_coef = (-1.0 / (self.sample_rate * 80.0 / 1000.0)).exp();

        let (num_samples, num_channels) = (buffer.samples(), buffer.channels());

        for i in 0..num_samples {
            let mut max_peak = 1e-6f32;
            for ch in 0..num_channels {
                let input_sample = buffer.as_slice()[ch][i] * output_gain;

                // 現在のサンプルをディレイバッファに入れ、過去のサンプルを取り出す
                let delayed_sample = self.delay_buffer[ch][self.delay_idx];
                self.delay_buffer[ch][self.delay_idx] = input_sample;

                // 「未来」のピークを検知するために現在の入力で判定
                max_peak = max_peak.max(input_sample.abs());

                // 出力バッファには遅延させたサンプルを書き込む準備
                buffer.as_slice()[ch][i] = delayed_sample;
            }

            let target_red = if max_peak > ceiling {
                ceiling / max_peak
            } else {
                1.0
            };

            // リダクションのスムージング
            let coef = if target_red < self.post_reduction {
                attack_coef
            } else {
                release_coef
            };
            self.post_reduction = target_red + coef * (self.post_reduction - target_red);

            // 遅延されたサンプルに対して、先読みして計算したリダクションを適用
            for ch in 0..num_channels {
                buffer.as_slice()[ch][i] *= self.post_reduction;
            }

            self.delay_idx = (self.delay_idx + 1) % self.delay_len;
        }
    }
}
