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
    ) {
        let attack_ms = 2.0;
        let release_ms = 150.0;
        let atk_coef = (-1.0 / (attack_ms * 0.001 * self.sample_rate)).exp();
        let rel_coef = (-1.0 / (release_ms * 0.001 * self.sample_rate)).exp();

        for channel_samples in buffer.iter_samples() {
            let mut ch_idx = 0;
            for sample in channel_samples {
                let input_abs = sample.abs();

                // 1. ノイズフロアの常時追従 (移動平均的な挙動)
                // 演奏が止まった時に学習を進め、音がある時はホールドする
                if input_abs < self.noise_floor * 5.0 {
                    // 入力信号がノイズフロアに近い場合、ゆっくりと基準を更新
                    self.noise_floor = self.noise_floor * 0.9999 + input_abs * 0.0001;
                }
                self.noise_floor = self.noise_floor.max(0.00001); // ゼロ除算防止

                // 2. エンベロープ検出
                if input_abs > self.envelope {
                    self.envelope = self.envelope * atk_coef + input_abs * (1.0 - atk_coef);
                } else {
                    self.envelope = self.envelope * rel_coef + input_abs * (1.0 - rel_coef);
                }

                // 3. ゲート判定
                let threshold = self.noise_floor * 3.0;
                let target_gate = if self.envelope > threshold { 1.0 } else { 0.0 };

                // 4. スムージング
                let g_coef = if target_gate > self.gate_gain {
                    atk_coef
                } else {
                    rel_coef
                };
                self.gate_gain = self.gate_gain * g_coef + target_gate * (1.0 - g_coef);

                // 5. 動的ローパスフィルタ (Dynamic LPF)
                // ゲインが下がるほど高域を削り、ゲートが完全に閉じる寸前の「サー」音を消す
                let cutoff_freq = 200.0 + (19800.0 * self.gate_gain.powi(4));
                let alpha = (2.0 * PI * cutoff_freq / self.sample_rate).min(0.95);

                let state = &mut self.lp_state[ch_idx % 2];
                *state = *state * (1.0 - alpha) + (*sample * alpha);

                *sample = *state * self.gate_gain;

                ch_idx += 1;
            }
        }
    }
}
