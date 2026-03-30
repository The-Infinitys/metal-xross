use std::sync::Arc;

use crate::params::MetalXrossParams;
use crate::utils::DbToLinear;
use nih_plug::prelude::*;

pub struct XrossNoiseGate {
    sample_rate: f32,
    gate_gain: f32,
    hold_timer: i32,

    // 帯域別エネルギー (解析用)
    low_env: f32,
    mid_env: f32,
    high_env: f32,

    // フィルタ状態保持
    lp_state: [f32; 2], // 600Hz
    hp_state: [f32; 2], // 4kHz
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
            lp_state: [0.0; 2],
            hp_state: [0.0; 2],
            params,
        }
    }

    pub fn initialize(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// 歪み前のクリーン信号でゲートの開閉を判断
    pub fn process_pre(&mut self, buffer: &mut Buffer) {
        let threshold = self.params.noise_gate.threshold.value().db_to_linear();
        let release_ms = self.params.noise_gate.release.value();

        // リリース速度をサンプル単位の係数に変換
        // release_ms が小さいほど coefficient は小さくなり、早く閉じる
        let release_coeff = (-1.0 / (release_ms * self.sample_rate / 1000.0)).exp();

        for (ch_idx, channel_samples) in buffer.iter_samples().enumerate() {
            let ch = ch_idx % 2;
            for sample in channel_samples {
                let input = *sample;

                // 1. 帯域分離 (解析用)
                self.lp_state[ch] += 0.08 * (input - self.lp_state[ch]);
                let low_comp = self.lp_state[ch];
                let mid_high_comp = input - low_comp;

                self.hp_state[ch] += 0.2 * (mid_high_comp - self.hp_state[ch]);
                let high_comp = self.hp_state[ch];
                let mid_comp = mid_high_comp - high_comp;

                // 2. エンベロープ追従
                let atk = 0.5;
                let rel = 0.0005; // 解析用エンベロープ自体は安定性をとる
                let update_env = |env: &mut f32, val: f32| {
                    let v = val.abs();
                    if v > *env {
                        *env += atk * (v - *env);
                    } else {
                        *env += rel * (v - *env);
                    }
                };

                update_env(&mut self.low_env, low_comp);
                update_env(&mut self.mid_env, mid_comp);
                update_env(&mut self.high_env, high_comp);

                // 3. 判定ロジック
                // 中域が閾値超え、または上下帯域がバランス良く鳴っているか
                let is_playing = self.mid_env > threshold
                    || (self.low_env > threshold && self.high_env > threshold * 0.5);

                if is_playing {
                    // 演奏中は一瞬で開き、ホールドタイマーをリセット
                    self.hold_timer = (0.03 * self.sample_rate) as i32; // 30msホールド
                    self.gate_gain = (self.gate_gain + 0.15).min(1.0);
                } else if self.hold_timer > 0 {
                    self.hold_timer -= 1;
                } else {
                    // パラメータに基づいたリリース速度で閉じる
                    self.gate_gain *= release_coeff;
                }
            }
        }
    }

    /// 歪み後の信号からノイズをスペクトル的に除去
    pub fn process_post(&mut self, buffer: &mut Buffer) {
        let tolerance = self.params.noise_gate.tolerance.value();

        for (ch_idx, channel_samples) in buffer.iter_samples().enumerate() {
            let ch = ch_idx % 2;
            for sample in channel_samples {
                let input = *sample;

                // 1. スペクトル解析 (歪み後の分布を再計算)
                // 低域(low), 中域(mid), 高域(high)のコンポーネントに分ける
                let low_part = self.lp_state[ch];
                let mid_high_part = input - low_part;
                let high_part = self.hp_state[ch];
                let mid_part = mid_high_part - high_part;

                // 2. ホワイトノイズ判定 (MidとHighの比率)
                // ギターの有効な音なら mid_env が高いはず
                let harmonic_ratio = self.mid_env / (self.high_env + 0.00001);

                // Toleranceが高いほど、判定が厳しくなる (高域が削られやすくなる)
                let high_mask = (harmonic_ratio * (2.0 - tolerance * 1.5)).clamp(0.0, 1.0);

                // ゲートが閉じるにつれて強まるフィルタ係数
                let closing_factor = (1.0 - self.gate_gain).powi(2);

                // 3. 帯域別ゲイン適用
                // Low: 0.1を下限にして、閉じた時のスカスカ感を防止
                let g_low = self.gate_gain.max(0.1 * self.gate_gain);
                let g_mid = self.gate_gain;
                // High: ゲートが閉じている最中で、かつノイズと判断された時だけ強力に削る
                let g_high = self.gate_gain * (1.0 - (closing_factor * (1.0 - high_mask)));

                // 4. 再合成
                let output = (low_part * g_low) + (mid_part * g_mid) + (high_part * g_high);

                // 5. 最終的なシャットアウト (超低ゲイン時は完全にゼロにする)
                if self.gate_gain < 0.005 {
                    *sample = output * (self.gate_gain / 0.005).powi(2);
                } else {
                    *sample = output;
                }
            }
        }
    }
}
