// src/amp/metal.rs
use super::XrossGainProcessor;
use crate::params::MetalXrossParams;
use std::f32::consts::PI;
use std::sync::Arc;
use truce::params::FloatParamReadF32;

/// 2次ダイレクトフォームII転置形式 Biquad フィルター
#[derive(Default, Clone)]
struct Biquad {
    z1: f32,
    z2: f32,
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Biquad {
    #[inline(always)]
    fn process(&mut self, input: f32) -> f32 {
        let out = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * out + self.z2;
        self.z2 = self.b2 * input - self.a2 * out;
        out
    }

    /// HighPass Filter
    fn set_highpass(&mut self, cutoff: f32, q: f32, sample_rate: f32) {
        let w0 = 2.0 * PI * (cutoff / sample_rate).min(0.48);
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);

        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 + cos_w0) / 2.0) / a0;
        self.b1 = (-(1.0 + cos_w0)) / a0;
        self.b2 = ((1.0 + cos_w0) / 2.0) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    /// Peaking Filter
    fn set_peaking(&mut self, cutoff: f32, gain_db: f32, q: f32, sample_rate: f32) {
        let w0 = 2.0 * PI * (cutoff / sample_rate).min(0.48);
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);
        let a = 10.0f32.powf(gain_db / 40.0);

        let a0 = 1.0 + alpha / a;
        self.b0 = (1.0 + alpha * a) / a0;
        self.b1 = (-2.0 * cos_w0) / a0;
        self.b2 = (1.0 - alpha * a) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha / a) / a0;
    }

    /// LowPass Filter
    fn set_lowpass(&mut self, cutoff: f32, q: f32, sample_rate: f32) {
        let w0 = 2.0 * PI * (cutoff / sample_rate).min(0.48);
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);

        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 - cos_w0) / 2.0) / a0;
        self.b1 = (1.0 - cos_w0) / a0;
        self.b2 = ((1.0 - cos_w0) / 2.0) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }
}

#[derive(Default, Clone)]
struct MetalChannelState {
    // フィルター群
    pre_tight_hp: Biquad,      // 歪み前のローカット
    pre_klang_boost: Biquad,   // 3.2kHz アタック・ピッキングバイト
    post_mud_notch: Biquad,    // 450Hz ピンポイント・ノッチカット
    post_mid_core: Biquad,     // 1.5kHz 鉄板のような音の芯
    post_chug_peaking: Biquad, // 80Hz 超硬質重低音パンチ
    os_lpf: Biquad,            // 抗エイリアシング LPF

    // エンベロープ・動的状態
    fast_env: f32,
    slow_env: f32,
    slew_state: f32,
    dc_block: f32,
    feedback_state: f32,
    prev_input: f32,

    current_os_factor: usize,
}

pub struct XrossMetalSystem {
    params: Arc<MetalXrossParams>,
    states: Vec<MetalChannelState>,
    sample_rate: f32,
}

impl XrossMetalSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            states: Vec::new(),
            sample_rate: 44100.0,
        }
    }

    /// Djent / Ultra Hard-Tuned Modern High-Gain コア・ディストーションエンジン
    #[inline(always)]
    fn drive_core(
        state: &mut MetalChannelState,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
        effective_sr: f32,
    ) -> f32 {
        // === 1. EXTREME PRE-GAIN TIGHTENING (超超タイト化) ===
        // 歪み前の低域カットラインを 200Hz〜400Hz へ大幅引き上げ。ブーミーさを完全粉砕
        let hp_freq = 400.0 - (s_low * 200.0);
        state.pre_tight_hp.set_highpass(hp_freq, 0.85, effective_sr);
        let mut x = state.pre_tight_hp.process(input);

        // 3.2kHz（ピッキングの「ガリッ」という高域金属成分）を前段で強烈ブースト (Q=1.5)
        let klang_gain = 4.0 + (s_high * 6.5);
        state
            .pre_klang_boost
            .set_peaking(3200.0, klang_gain, 1.5, effective_sr);
        x = state.pre_klang_boost.process(x);

        // === 2. HEAVY SATURATION & HARD WALL CLIPPING ===
        // ゲイン倍率を 2.0 倍まで底上げ
        let drive_amt = (1.0 + (gain * 45.0) + (gain.powi(3) * 80.0)) * 2.0;
        x *= drive_amt;

        // タイトなフィードバック
        x += state.feedback_state * 0.12;

        // アグレッシブなウェーブシェイピング（より角の立ったクリッピング）
        let positive = (x * 1.6).tanh();
        let negative = (x * 1.3).atan() * 1.1;
        x = if x > 0.0 { positive } else { negative };

        // リミット幅を厳しく絞り込み、音の壁を作る（ハードコンプレッション効果）
        let hard_clip = 0.78 - (s_high * 0.25);
        x = x.clamp(-hard_clip, hard_clip);

        state.feedback_state = x;

        // === 3. HARD EQ & DYNAMIC ATTACK ===

        // A) 450Hz 中低域の泥みをシャープな Q (2.2) で深めにノッチカット
        let mud_cut_db = -4.0 - (1.0 - s_mid) * 8.0;
        state
            .post_mud_notch
            .set_peaking(450.0, mud_cut_db, 2.2, effective_sr);
        x = state.post_mud_notch.process(x);

        // B) 1.5kHz 鉄板のような芯の付加
        let mid_gain_db = (s_mid - 0.5) * 12.0;
        let core_boost_db = mid_gain_db + 2.5;
        state
            .post_mid_core
            .set_peaking(1500.0, core_boost_db, 1.4, effective_sr);
        x = state.post_mid_core.process(x);

        // C) 80Hz サブベース・アタック（パームミュート時の硬質で重いパンチ）
        let attack_transient = (state.fast_env - state.slow_env).max(0.0);
        let chug_boost = (s_low * 6.0) + (attack_transient * 18.0 * s_low);
        state
            .post_chug_peaking
            .set_peaking(80.0, chug_boost, 2.8, effective_sr);
        x = state.post_chug_peaking.process(x);

        // SLEW RATE (アタックの立ち上がりスピードを最大化)
        let max_step = 0.15 + (s_high * 1.2);
        let diff = x - state.slew_state;
        state.slew_state += diff.clamp(-max_step, max_step);

        state.slew_state
    }

    fn process_sample(&mut self, input: f32, ch_idx: usize) -> f32 {
        let g = self.params.gain.value();
        let sl = self.params.style_low.value();
        let sm = self.params.style_mid.value();
        let sh = self.params.style_high.value();

        let state = &mut self.states[ch_idx];

        // 1. 高速/低速エンベロープ
        let abs_in = input.abs();
        state.fast_env += (abs_in - state.fast_env) * 0.25; // レスポンス速度向上
        state.slow_env += (abs_in - state.slow_env) * 0.005;

        // 2. 動的オーバーサンプリング
        let os_factor = if g < 0.2 {
            1
        } else if g < 0.5 {
            2
        } else {
            4
        };

        let effective_sr = self.sample_rate * os_factor as f32;
        let inv_os = 1.0 / os_factor as f32;

        if state.current_os_factor != os_factor {
            state.os_lpf.set_lowpass(18000.0, 0.707, effective_sr);
            state.current_os_factor = os_factor;
        }

        // 3. オーバーサンプリング・ループ
        let mut output_sum = 0.0;
        for i in 0..os_factor {
            let fraction = i as f32 * inv_os;
            let sub_sample = state.prev_input + (input - state.prev_input) * fraction;

            let driven = Self::drive_core(state, sub_sample, g, sl, sm, sh, effective_sr);
            output_sum += state.os_lpf.process(driven);
        }
        state.prev_input = input;

        let raw_out = output_sum * inv_os;

        // 4. DC カット
        let dc_fix = raw_out - state.dc_block;
        state.dc_block = raw_out + 0.995 * (state.dc_block - raw_out);

        dc_fix * 0.80
    }
}

impl XrossGainProcessor for XrossMetalSystem {
    fn initialize(&mut self, sample_rate: f32, num_channels: usize) {
        self.sample_rate = sample_rate;
        self.states = vec![MetalChannelState::default(); num_channels];
        for state in &mut self.states {
            state.os_lpf.set_lowpass(18000.0, 0.707, sample_rate);
            state.current_os_factor = 1;
        }
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        if ch_idx >= self.states.len() {
            return;
        }

        for sample in slice {
            *sample = self.process_sample(*sample, ch_idx);
        }
    }
}
