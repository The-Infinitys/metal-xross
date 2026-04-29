use crate::params::MetalXrossParams;
use std::sync::Arc;
use truce::prelude::*;

// デシベルからリニアゲインへの変換
fn db_to_gain(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

#[derive(Default, Clone)]
struct NoiseGateChannelState {
    gate_gain: f32,
    hold_timer: i32,

    // 検出用エンベロープ (Low / Mid / High)
    low_env: f32,
    mid_env: f32,
    high_env: f32,

    // 検出用分離フィルター状態
    lp_pre: f32,
    hp_pre: f32,

    // ポストゲート LPF 状態
    post_lpf_state: f32,
}

pub struct XrossNoiseGate {
    sample_rate: f32,
    states: Vec<NoiseGateChannelState>,
    params: Arc<MetalXrossParams>,
}

impl XrossNoiseGate {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            sample_rate: 44100.0,
            states: Vec::new(),
            params,
        }
    }

    pub fn initialize(&mut self, sample_rate: f32, num_channels: usize) {
        self.sample_rate = sample_rate;
        self.states = vec![NoiseGateChannelState::default(); num_channels];
        for state in &mut self.states {
            state.gate_gain = 1.0;
        }
    }

    /// 歪みの前段：ノイズの根本をカット
    pub fn process_pre_buffer(&mut self, buffer: &mut AudioBuffer) {
        let num_channels = buffer.channels();
        let num_samples = buffer.num_samples();

        if self.states.len() != num_channels {
            self.initialize(self.sample_rate, num_channels);
        }

        let threshold_db = self.params.gate_threshold.value();
        let open_threshold = db_to_gain(threshold_db);
        let close_threshold = db_to_gain(threshold_db - 3.0); // 3dB ヒステリシス

        let release_ms = self.params.gate_release.value();
        let attack_coeff = (-1.0 / (0.001 * self.sample_rate)).exp(); // 1ms
        let release_coeff = (-1.0 / (release_ms * self.sample_rate / 1000.0)).exp();

        for ch in 0..num_channels {
            let state = &mut self.states[ch];
            for i in 0..num_samples {
                let (_, out) = buffer.io(ch);
                let input = out[i];

                // 1. Detection (マルチバンドエンベロープ)
                state.lp_pre += 0.12 * (input - state.lp_pre);
                let low = state.lp_pre;
                let mid_high = input - low;

                state.hp_pre += 0.35 * (mid_high - state.hp_pre);
                let high = state.hp_pre;
                let mid = mid_high - high;

                let atk = 0.6;
                let rel = 0.008;
                let update_env = |env: &mut f32, val: f32| {
                    let v = val.abs();
                    if v > *env {
                        *env += atk * (v - *env);
                    } else {
                        *env += rel * (v - *env);
                    }
                };

                update_env(&mut state.low_env, low);
                update_env(&mut state.mid_env, mid);
                update_env(&mut state.high_env, high);

                // ギターの美味しい帯域（Mid）を重視した判定
                let current_env =
                    (state.mid_env * 0.7) + (state.low_env * 0.15) + (state.high_env * 0.15);

                // 2. Gate Logic
                let is_playing = if state.gate_gain < 0.1 {
                    current_env > open_threshold
                } else {
                    current_env > close_threshold
                };

                let target_gain = if is_playing {
                    state.hold_timer = (0.020 * self.sample_rate) as i32; // 20ms hold
                    1.0
                } else if state.hold_timer > 0 {
                    state.hold_timer -= 1;
                    1.0
                } else {
                    0.0
                };

                // 3. Smooth & Apply
                let coef = if target_gain > state.gate_gain {
                    attack_coeff
                } else {
                    release_coeff
                };
                state.gate_gain = target_gain + coef * (state.gate_gain - target_gain);

                out[i] = input * state.gate_gain;
            }
        }
    }

    /// 歪みの後段：残留ヒスノイズのフィルタリング
    pub fn process_post_buffer(&mut self, buffer: &mut AudioBuffer) {
        let num_channels = buffer.channels();
        let num_samples = buffer.num_samples();
        let tolerance = self.params.gate_tolerance.value();

        for ch in 0..num_channels {
            let state = &mut self.states[ch];

            // ゲートが閉じている最中、または閉じ切っている時のみ実行
            if state.gate_gain < 0.999 {
                // ゲートの閉鎖具合に応じて LPF の強度（カットオフ）を変える
                // 閉じれば閉じるほど（gate_gain -> 0）、フィルターが深くかかる
                let closing_factor = (1.0 - state.gate_gain).powi(2);
                let lpf_alpha = (tolerance * closing_factor * 0.85).min(0.99);

                for i in 0..num_samples {
                    let (_, out) = buffer.io(ch);
                    let s = out[i];

                    // 1-pole LPF でジリジリした高域ノイズだけを削る
                    state.post_lpf_state = s + lpf_alpha * (state.post_lpf_state - s);
                    out[i] = state.post_lpf_state;
                }
            }
        }
    }
}
