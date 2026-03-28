use nih_plug::prelude::*;

pub struct XrossNoiseGate {
    sample_rate: f32,
    gate_gain: f32,

    // 帯域別のエネルギー状態 (指数移動平均)
    low_energy: f32,  // 150Hz以下 (電源ハム等)
    mid_energy: f32,  // 500Hz~2kHz (ギターの美味しい帯域)
    high_energy: f32, // 4kHz以上 (高域ノイズ)

    // フィルタ状態保持
    lp_state: [f32; 2],
    hp_state: [f32; 2],

    hold_timer: f32,
}

impl XrossNoiseGate {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            gate_gain: 1.0,
            low_energy: 0.0,
            mid_energy: 0.0,
            high_energy: 0.0,
            lp_state: [0.0; 2],
            hp_state: [0.0; 2],
            hold_timer: 0.0,
        }
    }

    pub fn initialize(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn process_pre(&mut self, buffer: &mut Buffer) {
        let alpha_atk = 0.1;
        let alpha_rel = 0.0005; // かなりゆっくり戻す（サステイン維持）

        for (ch_idx, channel_samples) in buffer.iter_samples().enumerate() {
            let ch = ch_idx % 2;
            for sample in channel_samples {
                let input = *sample;

                // --- 1. 帯域分離 (簡易クロスオーバー) ---
                // Lowパス (500Hz)
                self.lp_state[ch] += 0.1 * (input - self.lp_state[ch]);
                let low_mid_comp = self.lp_state[ch];
                let high_comp = input - low_mid_comp;

                // Highパス (150Hz)
                self.hp_state[ch] += 0.05 * (low_mid_comp - self.hp_state[ch]);
                let mid_comp = low_mid_comp - self.hp_state[ch];
                let low_comp = self.hp_state[ch];

                // --- 2. 各帯域のエネルギー追従 ---
                let update_env = |env: &mut f32, val: f32| {
                    let v_abs = val.abs();
                    if v_abs > *env {
                        *env += alpha_atk * (v_abs - *env);
                    } else {
                        *env += alpha_rel * (v_abs - *env);
                    }
                };

                update_env(&mut self.low_energy, low_comp);
                update_env(&mut self.mid_energy, mid_comp);
                update_env(&mut self.high_energy, high_comp);

                // --- 3. ゲート判断ロジック ---
                // 「中域（ギターの芯）」が閾値を超えていれば、演奏中とみなす
                let mid_threshold = 0.004;
                if self.mid_energy > mid_threshold {
                    self.hold_timer = 0.05 * self.sample_rate; // 50msホールド
                    self.gate_gain = self.gate_gain * 0.9 + 1.0 * 0.1;
                } else if self.hold_timer > 0.0 {
                    self.hold_timer -= 1.0;
                } else {
                    self.gate_gain *= 0.9998; // 非常に緩やかに閉じる
                }

                // --- 4. インテリジェント・イコライジング ---
                // ギターの芯に対してノイズがどれくらい大きいか
                let noise_level = self.high_energy + self.low_energy * 0.5;
                let signal_to_noise = self.mid_energy / (noise_level + 0.0001);

                // 芯が弱くなるにつれて、ノイズの多い帯域(Low/High)を削る
                // 完全に音が消える前でも、耳障りな成分だけを先に減衰させる
                let filter_strength = (signal_to_noise * 2.0).clamp(0.1, 1.0);

                // ゲートが閉じ始めていたら(gate_gain < 1.0)、さらにフィルタを強める
                let dynamic_low = low_comp * self.gate_gain * filter_strength;
                let dynamic_high = high_comp * self.gate_gain * filter_strength;

                // 中域の芯は極力生かす
                *sample = dynamic_low + mid_comp * self.gate_gain + dynamic_high;
            }
        }
    }

    pub fn process_post(&mut self, buffer: &mut Buffer) {
        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
                // 最後の最後、ノイズが完全に支配的になった瞬間だけシャットアウト
                // gate_gainが0.05 (-26dB程度) までは、ほぼそのまま通す
                let out_gain = if self.gate_gain > 0.05 {
                    1.0
                } else {
                    (self.gate_gain / 0.05).powi(2)
                };

                *sample *= out_gain;
            }
        }
    }
}
