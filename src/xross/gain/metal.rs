use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossMetalSystem {
    params: Arc<MetalXrossParams>,
    pre_hp: [f32; 2],
    pre_res: [f32; 2],
    slew_state: [f32; 2],
    dc_block: [f32; 2],
    envelope: [f32; 2],
    prev_input: [f32; 2],
    os_lpf: [f32; 2],
    low_resonance: [f32; 2],
}

impl XrossMetalSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            pre_hp: [0.0; 2],
            pre_res: [0.0; 2],
            slew_state: [0.0; 2],
            dc_block: [0.0; 2],
            envelope: [0.0; 2],
            prev_input: [0.0; 2],
            os_lpf: [0.0; 2],
            low_resonance: [0.0; 2],
        }
    }

    #[inline]
    fn drive_core(&mut self, input: f32, env: f32, ch: usize) -> f32 {
        let gain = self.params.general.gain.value();
        let s_low = self.params.style.low.value();
        let s_mid = self.params.style.mid.value();
        let s_high = self.params.style.high.value();
        // === Modern Metal Tight & Wide Edition ===
        // モダンメタル向けにさらにタイト（低域の締まり）＋ワイドレンジ（極端なV字ドンシャリ）に調整。
        // 高域の熱さと迫力は維持しつつ、ホワイトノイズをさらに強力に抑制。

        // 1. プリ・ハイパス（より高いカットオフで低域をタイトに）
        let hp_freq = 0.155 + (1.0 - s_low) * 0.115;
        self.pre_hp[ch] += hp_freq * (input - self.pre_hp[ch]);
        let mut x = input - self.pre_hp[ch];

        // 2. プリ・レゾナンス（中域の熱さをキープしつつノイズ蓄積を防止）
        let res_freq = 0.265;
        let res_q = 0.58 + (s_mid * 0.52);
        self.pre_res[ch] += res_freq * (x - self.pre_res[ch]);
        x = x + (x - self.pre_res[ch]) * (3.15 * res_q);
        self.pre_res[ch] *= 0.978; // さらに軽いダンピングでノイズ抑制

        // 3. 凶悪多段ハードクリッピング（モダンメタルらしい固い飽和）
        let drive = 11.5 + (gain * 57.0); // 内部ゲインを最適化（ノイズ低減）
        x *= drive;

        // Stage 1: 非対称クリッピング（攻撃性を保ちつつ滑らかに）
        x = if x > 0.0 {
            (x * 2.75).atan() * 0.43
        } else {
            (x * 2.15).tanh() * 0.47
        };

        // Stage 2: 超ハード・リミッティング（固く・クリアに）
        let hard_warp = 2.45 + (gain * 9.0);
        x = (x * hard_warp).clamp(-0.925, 0.925);

        // 4. ポスト・スクープEQ（ワイドレンジ化：中域をさらに深く削る）
        let scoop_depth = (0.67 - s_mid).max(0.0) * 1.85; // より極端なV字
        let scoop_filter = x.powi(3) - x * 0.26;
        x -= scoop_filter * scoop_depth;

        // 5. Bite & Edge（高域の金属エッジを残しつつノイズ激減）
        let bite = 0.135 + (s_high * 0.58) * (env * 0.78 + 0.20);
        let diff = x - self.slew_state[ch];
        self.slew_state[ch] += diff.clamp(-bite, bite);
        x = self.slew_state[ch];

        // 6. キャビネット・パンチ（低域を固く・重く、タイトに）
        let punch_amount = s_low * 1.28;
        self.low_resonance[ch] += 0.088 * (x - self.low_resonance[ch]); // レスポンスを速く
        x += self.low_resonance[ch] * punch_amount;

        x.clamp(-1.0, 1.0)
    }

    fn process_sample(&mut self, input: f32, ch: usize) -> f32 {
        let os_factor = 4;
        let target = input.abs();

        // エンベロープ（モダンメタルらしいタイトなアタック）
        let env_step = if target > self.envelope[ch] {
            0.29
        } else {
            0.011
        };
        self.envelope[ch] += env_step * (target - self.envelope[ch]);

        let mut output_sum = 0.0;
        let inv_os = 1.0 / os_factor as f32;

        for i in 0..os_factor {
            let fraction = i as f32 * inv_os;
            let sub_sample = self.prev_input[ch] + (input - self.prev_input[ch]) * fraction;
            output_sum += self.drive_core(sub_sample, self.envelope[ch], ch);
        }

        self.prev_input[ch] = input;
        let raw_out = output_sum * inv_os;

        // === ノイズ対策強化：OS後LPFをさらに強力に（0.62 → 0.57）===
        self.os_lpf[ch] += 0.57 * (raw_out - self.os_lpf[ch]);

        let out = self.os_lpf[ch];

        // DCブロッカー（低域ノイズもさらに抑制）
        let dc_fix = out - self.dc_block[ch];
        self.dc_block[ch] = out + 0.9945 * (self.dc_block[ch] - out);

        dc_fix * 0.84 // 最終レベルを少し上げてワイドレンジの迫力を維持
    }
}

impl XrossGainProcessor for XrossMetalSystem {
    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<crate::MetalXross>,
    ) -> bool {
        self.pre_hp = [0.0; 2];
        self.pre_res = [0.0; 2];
        self.slew_state = [0.0; 2];
        self.dc_block = [0.0; 2];
        self.envelope = [0.0; 2];
        self.prev_input = [0.0; 2];
        self.os_lpf = [0.0; 2];
        self.low_resonance = [0.0; 2];
        true
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        let ch = ch_idx % 2;
        for sample in slice {
            *sample = self.process_sample(*sample, ch);
        }
    }
}
