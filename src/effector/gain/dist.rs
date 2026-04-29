use super::XrossGainProcessor;
use crate::params::MetalXrossParams;
use std::sync::Arc;

#[derive(Default, Clone)]
struct DistChannelState {
    dc_block: f32,
    filter_h: f32,
    filter_l: f32,
    prev_input: f32,
}

pub struct XrossDistSystem {
    params: Arc<MetalXrossParams>,
    states: Vec<DistChannelState>,
    sample_rate: f32,
}

impl XrossDistSystem {
    pub fn new(params: Arc<MetalXrossParams>) -> Self {
        Self {
            params,
            states: Vec::new(),
            sample_rate: 44100.0,
        }
    }

    /// ダイオード・クリッピングのシミュレーション
    /// 特定のしきい値を超えると atan 曲線で急激に丸め込む（ハード・クリップに近い挙動）
    #[inline(always)]
    fn diode_clipper(x: f32, threshold: f32) -> f32 {
        if x > threshold {
            threshold + (x - threshold).atan() * 0.15
        } else if x < -threshold {
            -threshold + (x + threshold).atan() * 0.15
        } else {
            x
        }
    }

    fn drive_core(
        state: &mut DistChannelState,
        input: f32,
        gain: f32,
        s_low: f32,
        s_mid: f32,
        s_high: f32,
    ) -> f32 {
        // 1. PRE-FILTER (ディストーションのキャラクター決定)
        // Style Low が低いほど、歪む前に大胆に低域をカット（タイトにする）
        let pre_hp = 0.025 + (1.0 - s_low).powi(2) * 0.06;
        state.filter_h += pre_hp * (input - state.filter_h);
        let mut x = input - state.filter_h;

        // 2. MID BUMP & GAIN STAGE
        // 800Hz〜1kHz 付近の押し出し（s_mid）と強烈なゲイン
        let mid_push = 1.0 + (s_mid * 2.5);
        let drive = (5.0 + (gain * 120.0)) * mid_push;
        x *= drive;

        // 3. ASYMMETRIC DIODE CLIPPING
        // わずかにしきい値を非対称にすることで偶数次倍音を加える
        let clip_thresh_p = 0.45;
        let clip_thresh_n = 0.50;

        let x_clipped = if x > 0.0 {
            Self::diode_clipper(x, clip_thresh_p)
        } else {
            Self::diode_clipper(x, clip_thresh_n)
        };

        // 4. POST-FILTER (Style High に連動するトーン)
        // 歪みで発生したジャリジャリ感を音楽的に整える
        let post_lp = 0.08 + (s_high * 0.45);
        state.filter_l += post_lp * (x_clipped - state.filter_l);
        let mut out = state.filter_l;

        // 5. HARD LIMITER / COMPRESSION
        // ディストーションらしい圧縮感。少しレベルを持ち上げてから固める
        out = (out * 1.5).clamp(-0.98, 0.98);

        out
    }

    fn process_sample(&mut self, input: f32, ch_idx: usize) -> f32 {
        let g = self.params.gain.value();
        let sl = self.params.style_low.value();
        let sm = self.params.style_mid.value();
        let sh = self.params.style_high.value();

        let state = &mut self.states[ch_idx];

        // 4x Oversampling
        let os_factor = 4;
        let inv_os = 1.0 / os_factor as f32;
        let mut output_sum = 0.0;

        for i in 0..os_factor {
            let fraction = i as f32 * inv_os;
            let sub_sample = state.prev_input + (input - state.prev_input) * fraction;
            output_sum += Self::drive_core(state, sub_sample, g, sl, sm, sh);
        }
        state.prev_input = input;

        let out = output_sum * inv_os;

        // DC Block
        let dc_fix = out - state.dc_block;
        state.dc_block = out + 0.996 * (state.dc_block - out);

        // 最終音量の調整（歪ませたあとの実効音圧が高いため少し絞る）
        dc_fix * 0.85
    }
}

impl XrossGainProcessor for XrossDistSystem {
    fn initialize(&mut self, sample_rate: f32, num_channels: usize) {
        self.sample_rate = sample_rate;
        self.states = vec![DistChannelState::default(); num_channels];
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
