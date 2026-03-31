use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossMetalSystem {
    params: Arc<MetalXrossParams>,
    pre_hp: [f32; 2],
    pre_hp_fast: [f32; 2],
    slew_state: [f32; 2],
    dc_block: [f32; 2],
    prev_input: [f32; 2],
    envelope: [f32; 2],
}

impl XrossMetalSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            pre_hp: [0.0; 2],
            pre_hp_fast: [0.0; 2],
            slew_state: [0.0; 2],
            dc_block: [0.0; 2],
            prev_input: [0.0; 2],
            envelope: [0.0; 2],
        }
    }

    fn drive_core(
        &mut self,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
        ch: usize,
    ) -> f32 {
        // --- 1. PRE-DYNAMICS (入力感知) ---
        let abs_in = input.abs();
        self.envelope[ch] += (abs_in - self.envelope[ch]) * 0.05;

        // --- 2. PRE-FILTER (低域のダブつきを徹底排除) ---
        // Style Lowが低いほど、タイトに（400Hz付近までカット）
        let hp_freq = 0.05 + (1.0 - s_low) * 0.15 + (self.envelope[ch] * 0.1);
        self.pre_hp[ch] += hp_freq * (input - self.pre_hp[ch]);
        let mut x = input - self.pre_hp[ch];

        // --- 3. MONSTER GAIN STAGE (歪みの核) ---
        // ゲインを指数関数的に増幅 (最大 +80dB 以上のイメージ)
        let drive_amount = (gain * 6.0).exp() * 20.0;
        x *= drive_amount;

        // --- 4. TRIPLE-STAGE SHAPING (粘りとキレの両立) ---

        // Step A: 非対称サチュレーション (偶数次倍音 = 粘り)
        // 上下でクリップの深さを変え、波形を「いびつ」にする
        x = if x > 0.0 {
            (x * 1.5).tanh()
        } else {
            (x * 0.8).atan() * 1.2
        };

        // Step B: ミッド・ブースト & 2nd 歪み
        // Style Midで「ゴンッ」という芯を作る
        let mid_push = s_mid * 2.0;
        x = (x + (x * x * x) * mid_push).clamp(-1.0, 1.0);

        // Step C: ハード・エッジ (矩形波に近づける)
        // 0.1を超えたら急激に潰す
        let hard_gain = 4.0;
        x = (x * hard_gain).clamp(-0.95, 0.95);

        // --- 5. SLEW RATE & BITE (高域のキレ) ---
        // スルーレートを Style High で開放。最大時はほぼ「素通し」でエッジを立たせる
        let max_step = 0.05 + (s_high * 0.9);
        let diff = x - self.slew_state[ch];
        self.slew_state[ch] += diff.clamp(-max_step, max_step);
        x = self.slew_state[ch];

        x
    }

    fn process_sample(
        &mut self,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
        ch: usize,
    ) -> f32 {
        // OSをあえて通さず、エイリアシングによる「汚さ」を味方につける (Dirty Metal)
        // もしノイズが酷すぎる場合は drive_core の前後で 2倍OSを検討
        let output = self.drive_core(input, gain, s_low, s_mid, s_high, ch);

        // DC Block (必須)
        let dc_fix = output - self.dc_block[ch];
        self.dc_block[ch] = output + 0.995 * (self.dc_block[ch] - output);

        // 最終メイクアップ (これでもデカすぎる場合は 0.5 に下げてください)
        dc_fix * 0.7
    }
}

impl XrossGainProcessor for XrossMetalSystem {
    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<crate::MetalXross>,
    ) -> bool {
        // 固定配列を使用しているのでシンプルに初期化
        self.pre_hp = [0.0; 2];
        self.pre_hp_fast = [0.0; 2];
        self.slew_state = [0.0; 2];
        self.dc_block = [0.0; 2];
        self.prev_input = [0.0; 2];
        self.envelope = [0.0; 2];
        true
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        let g = self.params.general.gain.value();
        let sl = self.params.style.low.value();
        let sm = self.params.style.mid.value();
        let sh = self.params.style.high.value();

        // ch_idx が 2チャンネル以上の場合の安全策
        let ch = ch_idx % 2;

        for sample in slice {
            *sample = self.process_sample(*sample, g, sl, sm, sh, ch);
        }
    }
}
