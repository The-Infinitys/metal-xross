use crate::MetalXross;
use crate::params::MetalXrossParams;
use crate::xross::gain::XrossGainProcessor;
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct XrossCrunchSystem {
    params: Arc<MetalXrossParams>,
    low_cut_state: Vec<f32>,
    high_shelf_state: Vec<f32>,
    dc_block_state: Vec<[f32; 2]>,
    bias_env: Vec<f32>,
    // 物理的なサギング（Sag）を模倣するための時定数保持用
    sag_state: Vec<f32>,
}

impl XrossCrunchSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            low_cut_state: vec![0.0; 2],
            high_shelf_state: vec![0.0; 2],
            dc_block_state: vec![[0.0; 2]; 2],
            bias_env: vec![0.0; 2],
            sag_state: vec![1.0; 2], // 1.0 = 電圧フル、ここから下がる
        }
    }

    /// 真空管の非対称サチュレーション（グリッド電流とカットオフの再現）
    #[inline(always)]
    fn tube_stage(&self, x: f32, bias: f32, sag: f32) -> f32 {
        // sagによって全体のヘッドルームがわずかに狭まる
        let input = (x * sag) + bias;

        if input > 0.0 {
            // クリーン〜ソフトサチュレーション領域
            input.tanh()
        } else {
            // マイナス側は「カットオフ」に向かってより急激に飽和
            // atanを使いつつ、バイアスが深いほど「ブチブチ」とした質感に
            let k = 1.6 + bias.abs() * 2.0;
            (input * k).atan() * (1.0 / k.atan())
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
        // 1. TIGHTNESS (Pre-HPF)
        // クランチは低域が多すぎると濁るため、s_lowに応じて80Hz-250Hzを可変カット
        let hp_freq = 0.02 + (1.0 - s_low).powf(1.5) * 0.15;
        let filtered = input - self.low_cut_state[ch];
        self.low_cut_state[ch] = input * hp_freq + self.low_cut_state[ch] * (1.0 - hp_freq);

        // 2. DYNAMIC SAG & BIAS (アンプの呼吸)
        // 信号が強いほど電源電圧が落ちる(sag)、かつ動作点がずれる(bias)
        let abs_in = filtered.abs();

        // Sag: 強い信号でヘッドルームが収縮し、コンプレッション感が出る
        let target_sag = 1.0 - (abs_in * 0.2 * gain);
        self.sag_state[ch] = self.sag_state[ch] * 0.99 + target_sag * 0.01;

        // Bias: 非対称性を生み、偶数次倍音（リッチな響き）を付加
        let target_bias = -(abs_in * 0.15 * gain) + (s_mid * 0.05);
        self.bias_env[ch] = self.bias_env[ch] * 0.995 + target_bias * 0.005;

        // 3. DRIVE STAGE
        // Gain 0.0 = 真空管を通しただけのクリーン / 1.0 = 激しいピッキングで歪むクランチ
        let drive = 1.0 + (gain * 8.0);

        // 多段処理で深みを出す
        let stage1 = self.tube_stage(filtered * drive, self.bias_env[ch], self.sag_state[ch]);
        let stage2 = self.tube_stage(stage1 * 1.5, self.bias_env[ch] * 0.5, 1.0);

        // 4. POST-FILTER (Style High)
        let lp_freq = 0.15 + (s_high * 0.5);
        let out = stage2 * lp_freq + self.high_shelf_state[ch] * (1.0 - lp_freq);
        self.high_shelf_state[ch] = out;

        out
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
        if gain < 0.001 {
            return input;
        }

        // クランチは繊細な倍音が命なので、常に2倍オーバーサンプリング
        let mut output = 0.0;
        for _ in 0..2 {
            output += self.drive_core(input, gain, s_low, s_mid, s_high, ch);
        }
        output *= 0.5;

        // Dry Mix: Gainが低い時の「弾力のあるクリーン」を維持
        let dry_mix = (1.0 - gain).powf(1.5) * 0.5;
        let mixed = output * (1.0 - dry_mix) + input * dry_mix;

        // DC Block (バイアス変動によるノイズ除去)
        let final_out = mixed - self.dc_block_state[ch][0] + (0.995 * self.dc_block_state[ch][1]);
        self.dc_block_state[ch][0] = mixed;
        self.dc_block_state[ch][1] = final_out;

        final_out * 0.8
    }
}

impl XrossGainProcessor for XrossCrunchSystem {
    fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        _config: &BufferConfig,
        _context: &mut impl InitContext<MetalXross>,
    ) -> bool {
        let num_channels = layout.main_output_channels.map(|n| n.get()).unwrap_or(2) as usize;
        self.low_cut_state = vec![0.0; num_channels];
        self.high_shelf_state = vec![0.0; num_channels];
        self.dc_block_state = vec![[0.0; 2]; num_channels];
        self.bias_env = vec![0.0; num_channels];
        self.sag_state = vec![1.0; num_channels];
        true
    }

    fn process_channel(&mut self, slice: &mut [f32], ch_idx: usize) {
        let g = self.params.general.gain.value();
        let sl = self.params.style.low.value();
        let sm = self.params.style.mid.value();
        let sh = self.params.style.high.value();

        for sample in slice {
            *sample = self.process_sample(*sample, g, sl, sm, sh, ch_idx);
        }
    }
}
