use crate::params::MetalXrossParams;
use std::sync::Arc;
use truce::prelude::*;

// デシベルからリニアゲインへの変換
fn db_to_gain(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

#[derive(Default, Clone)]
struct LevelChannelState {
    sc_hp_state: f32,
    delay_buffer: Vec<f32>,
}

pub struct XrossLevelSystem {
    params: Arc<MetalXrossParams>,
    states: Vec<LevelChannelState>,

    // エンベロープ/リダクション状態
    pre_boost: f32,
    post_reduction: f32,

    delay_idx: usize,
    delay_len: usize,
    sample_rate: f32,
}

impl XrossLevelSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            states: Vec::new(),
            pre_boost: 1.0,
            post_reduction: 1.0,
            delay_idx: 0,
            delay_len: 0,
            sample_rate: 44100.0,
        }
    }

    pub fn initialize(&mut self, sample_rate: f32, num_channels: usize) {
        self.sample_rate = sample_rate;
        // 2.5ms look-ahead
        self.delay_len = ((sample_rate * 2.5) / 1000.0).max(1.0) as usize;

        self.states = vec![
            LevelChannelState {
                sc_hp_state: 0.0,
                delay_buffer: vec![0.0; self.delay_len],
            };
            num_channels
        ];

        self.delay_idx = 0;
        self.pre_boost = 1.0;
        self.post_reduction = 1.0;
    }

    /// 歪み前：入力ゲインの適用とピーク保護
    pub fn pre_process_buffer(&mut self, buffer: &mut AudioBuffer) {
        let num_channels = buffer.channels();
        let num_samples = buffer.num_samples();

        if self.states.len() != num_channels {
            self.initialize(self.sample_rate, num_channels);
        }

        let target_gain = db_to_gain(self.params.input_gain.value());
        let limit_linear = db_to_gain(self.params.input_limit.value());

        // スムージング用定数
        let attack_coef = (-1.0 / (self.sample_rate * 2.0 / 1000.0)).exp();
        let release_coef = (-1.0 / (self.sample_rate * 100.0 / 1000.0)).exp();

        for i in 0..num_samples {
            let mut max_sc = 1e-6f32;

            for ch in 0..num_channels {
                let (_, out) = buffer.io(ch);
                let s = out[i];

                // サイドチェイン用の直流カット
                let hp = s - self.states[ch].sc_hp_state;
                self.states[ch].sc_hp_state = s * 0.05 + self.states[ch].sc_hp_state * 0.95;
                max_sc = max_sc.max(hp.abs());
            }

            // ゲインを上げつつ、リミットを超えないようにブースト量を抑制
            let target_boost = (limit_linear / max_sc).min(target_gain).max(0.001);

            let coef = if target_boost < self.pre_boost {
                attack_coef
            } else {
                release_coef
            };
            self.pre_boost = target_boost + coef * (self.pre_boost - target_boost);

            for ch in 0..num_channels {
                let (_, out) = buffer.io(ch);
                out[i] *= self.pre_boost;
            }
        }
    }

    /// 最終段：アウトプットゲインと Look-ahead リミッター
    pub fn post_process_buffer(&mut self, buffer: &mut AudioBuffer) {
        let num_channels = buffer.channels();
        let num_samples = buffer.num_samples();

        let output_gain = db_to_gain(self.params.output_gain.value());
        let ceiling = db_to_gain(self.params.output_limit.value()).min(1.0);

        // リミッターのアタックは速く
        let attack_coef = (-1.0 / (self.sample_rate * 1.0 / 1000.0)).exp();
        let release_coef = (-1.0 / (self.sample_rate * 150.0 / 1000.0)).exp();

        for i in 0..num_samples {
            let mut max_peak = 1e-6f32;

            for ch in 0..num_channels {
                let (_, out) = buffer.io(ch);
                let current_input = out[i] * output_gain;

                // Look-ahead: 現在の入力をディレイに書き込み、過去のサンプルを出力候補にする
                let delayed_sample = self.states[ch].delay_buffer[self.delay_idx];
                self.states[ch].delay_buffer[self.delay_idx] = current_input;

                // ピーク検出は「未来のサンプル（現在の入力）」で行う
                max_peak = max_peak.max(current_input.abs());

                // 出力を一旦遅延サンプルに置き換える
                let (_, out) = buffer.io(ch);
                out[i] = delayed_sample;
            }

            // 必要リダクション量
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

            // 全チャンネルにリダクションを同期適用（ステレオイメージ保持）
            for ch in 0..num_channels {
                let (_, out) = buffer.io(ch);
                out[i] *= self.post_reduction;
            }

            self.delay_idx = (self.delay_idx + 1) % self.delay_len;
        }
    }
}
