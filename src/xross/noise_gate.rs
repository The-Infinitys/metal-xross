use crate::MetalXross;
use nih_plug::prelude::*;
use std::f32::consts::PI;

pub struct XrossNoiseGate {
    sample_rate: f32,
    envelope: f32,
    gate_gain: f32,

    // 常に最新のノイズフロアを追従する変数
    noise_floor: f32,

    // ステレオフィルタ状態
    lp_state: [f32; 2],
}

impl XrossNoiseGate {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            envelope: 0.0,
            gate_gain: 1.0,
            noise_floor: 0.001, // 緩やかに変動させる初期値
            lp_state: [0.0; 2],
        }
    }

    pub fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    pub fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<MetalXross>,
    ) -> bool {
        let attack_ms = 1.0; // 立ち上がりは鋭く
        let release_ms = 200.0; // サステインのために少し長めに
        let atk_coef = (-1.0 / (attack_ms * 0.001 * self.sample_rate)).exp();
        let rel_coef = (-1.0 / (release_ms * 0.001 * self.sample_rate)).exp();

        // --- パラメータの定数化 (ここが肝) ---
        let max_noise_floor = 0.01; // ノイズフロアがこれ以上上がるのを防ぐ (-40dB程度)
        let min_noise_floor = 0.00001; // (-100dB)
        let freeze_threshold = 0.05; // この値を超えたら演奏中とみなし、ノイズフロア更新を止める

        let mut is_audible = false;

        for channel_samples in buffer.iter_samples() {
            let mut ch_idx = 0;
            for sample in channel_samples {
                let input_abs = sample.abs();

                // 1. ノイズフロア追従 (修正: 演奏中は更新をフリーズ)
                // 入力が一定以下、かつ急激なピークでない時だけノイズ学習
                if input_abs < freeze_threshold {
                    // 追従速度を少し落として安定させる
                    self.noise_floor = self.noise_floor * 0.99995 + input_abs * 0.00005;
                }

                // 閾値の暴走をハードリミットで抑える
                self.noise_floor = self.noise_floor.clamp(min_noise_floor, max_noise_floor);

                // 2. エンベロープ検出
                if input_abs > self.envelope {
                    self.envelope = self.envelope * atk_coef + input_abs * (1.0 - atk_coef);
                } else {
                    self.envelope = self.envelope * rel_coef + input_abs * (1.0 - rel_coef);
                }

                // 3. ゲート判定 (ヒステリシスを持たせる)
                // 開く時は threshold、閉じる時はその半分にするなどしてバタつきを防止
                let open_threshold = self.noise_floor * 4.0;
                let close_threshold = open_threshold * 0.5; // ヒステリシス

                let target_gate = if self.gate_gain < 0.1 {
                    if self.envelope > open_threshold {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    if self.envelope > close_threshold {
                        1.0
                    } else {
                        0.0
                    }
                };

                // 4. ゲインスムージング (リリースをゲート専用に少し遅くするのも手)
                let g_coef = if target_gate > self.gate_gain {
                    atk_coef
                } else {
                    rel_coef
                };
                self.gate_gain = self.gate_gain * g_coef + target_gate * (1.0 - g_coef);

                // 5. 動的LPF (サステイン後半で高域ノイズを消す)
                let cutoff_freq = 150.0 + (19850.0 * self.gate_gain.powi(2)); // powi(2)で少し緩やかに
                let alpha = (2.0 * PI * cutoff_freq / self.sample_rate).min(0.9);

                let state = &mut self.lp_state[ch_idx % 2];
                *state = *state * (1.0 - alpha) + (*sample * alpha);

                // 6. 出力
                let output = *state * self.gate_gain;
                *sample = output;

                if output.abs() > 1e-6 {
                    is_audible = true;
                }
                ch_idx += 1;
            }
        }
        is_audible || self.gate_gain > 1e-6
    }
}
