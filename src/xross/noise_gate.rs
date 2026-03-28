use nih_plug::prelude::*;
use std::f32::consts::PI;

pub struct XrossNoiseGate {
    sample_rate: f32,
    gate_gain: f32,     // 0.0 ~ 1.0 の開閉状態
    energy_smooth: f32, // 入力エネルギーの追従

    // フィルタ状態
    lp_state: [f32; 2],
    hp_state: [f32; 2],
}

impl XrossNoiseGate {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            gate_gain: 1.0,
            energy_smooth: 0.0,
            lp_state: [0.0; 2],
            hp_state: [0.0; 2],
        }
    }

    pub fn initialize(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// 1. 入力直後に適用：ノイズ帯域を動的にフィルタリング
    pub fn process_pre(&mut self, buffer: &mut Buffer) {
        let atk = 0.999;
        let rel = 0.99995;

        for (ch_idx, channel_samples) in buffer.iter_samples().enumerate() {
            for sample in channel_samples {
                let input_abs = sample.abs();

                // エネルギー検知 (中域〜全域)
                if input_abs > self.energy_smooth {
                    self.energy_smooth = self.energy_smooth * 0.9 + input_abs * 0.1;
                } else {
                    self.energy_smooth = self.energy_smooth * 0.9999 + input_abs * 0.0001;
                }

                // ゲートターゲットの判定
                let threshold = 0.008;
                let target = if self.energy_smooth > threshold {
                    1.0
                } else {
                    0.0
                };

                // ゲインスムージング (開くのは速く、閉じるのは滑らかに)
                let g_coef = if target > self.gate_gain { atk } else { rel };
                self.gate_gain = self.gate_gain * g_coef + target * (1.0 - g_coef);

                // --- 適応型フィルタ ---
                // 弾いていない時は 700Hz~4kHz に絞り、弾くと 40Hz~16kHz まで開く
                let lp_freq = 4000.0 + (12000.0 * self.gate_gain.powi(2));
                let hp_freq = 300.0 * (1.0 - self.gate_gain) + 40.0;

                let lp_alpha = (2.0 * PI * lp_freq / self.sample_rate).min(0.9);
                let hp_alpha = (2.0 * PI * hp_freq / self.sample_rate).min(0.9);

                let lp_s = &mut self.lp_state[ch_idx % 2];
                *lp_s = *lp_s * (1.0 - lp_alpha) + (*sample * lp_alpha);

                let hp_s = &mut self.hp_state[ch_idx % 2];
                *hp_s = *hp_s * (1.0 - hp_alpha) + (*lp_s * hp_alpha);

                // フィルタを通した音を入力として戻す（歪み回路へ）
                *sample = *lp_s - *hp_s;
            }
        }
    }

    /// 2. 最終出力直前に適用：音量を完全にシャットアウト
    pub fn process_post(&mut self, buffer: &mut Buffer) {
        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
                // Pre段で計算された gate_gain をそのまま利用してミュート
                // 10%以下まで閉じたら完全に0にするヒステリシス
                let final_gain = if self.gate_gain < 0.1 {
                    self.gate_gain * (self.gate_gain / 0.1) // 指数的に消音
                } else {
                    self.gate_gain
                };

                *sample *= final_gain;
            }
        }
    }
}
