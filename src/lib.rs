use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::Arc;

// EQのBiquadフィルタの状態
#[derive(Clone, Copy)]
struct Biquad {
    a1: f32, a2: f32, b0: f32, b1: f32, b2: f32,
    z1: f32, z2: f32,
}

impl Biquad {
    fn new() -> Self {
        Self { a1: 0.0, a2: 0.0, b0: 1.0, b1: 0.0, b2: 0.0, z1: 0.0, z2: 0.0 }
    }
    fn reset(&mut self) { self.z1 = 0.0; self.z2 = 0.0; }
    fn process(&mut self, input: f32) -> f32 {
        let out = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * out + self.z2;
        self.z2 = self.b2 * input - self.a2 * out;
        out
    }

    fn set_hpf(&mut self, freq: f32, sample_rate: f32, q: f32) {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);
        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        self.b0 = b0 / a0; self.b1 = b1 / a0; self.b2 = b2 / a0; self.a1 = a1 / a0; self.a2 = a2 / a0;
    }

    fn set_lpf(&mut self, freq: f32, sample_rate: f32, q: f32) {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);
        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        self.b0 = b0 / a0; self.b1 = b1 / a0; self.b2 = b2 / a0; self.a1 = a1 / a0; self.a2 = a2 / a0;
    }

    fn set_low_shelf(&mut self, freq: f32, sample_rate: f32, gain_db: f32, q: f32) {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let alpha = w0.sin() / 2.0 * (1.0 / q).sqrt();
        let cos_w0 = w0.cos();
        let a_sqrt_2_alpha = 2.0 * a.sqrt() * alpha;
        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + a_sqrt_2_alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - a_sqrt_2_alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + a_sqrt_2_alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - a_sqrt_2_alpha;
        self.b0 = b0 / a0; self.b1 = b1 / a0; self.b2 = b2 / a0; self.a1 = a1 / a0; self.a2 = a2 / a0;
    }

    fn set_high_shelf(&mut self, freq: f32, sample_rate: f32, gain_db: f32, q: f32) {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let alpha = w0.sin() / 2.0 * (1.0 / q).sqrt();
        let cos_w0 = w0.cos();
        let a_sqrt_2_alpha = 2.0 * a.sqrt() * alpha;
        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + a_sqrt_2_alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - a_sqrt_2_alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + a_sqrt_2_alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - a_sqrt_2_alpha;
        self.b0 = b0 / a0; self.b1 = b1 / a0; self.b2 = b2 / a0; self.a1 = a1 / a0; self.a2 = a2 / a0;
    }

    fn set_peaking(&mut self, freq: f32, sample_rate: f32, gain_db: f32, q: f32) {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();
        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;
        self.b0 = b0 / a0; self.b1 = b1 / a0; self.b2 = b2 / a0; self.a1 = a1 / a0; self.a2 = a2 / a0;
    }
}

pub struct MetalXross {
    params: Arc<MetalXrossParams>,
    filters: [[Biquad; 5]; 2],
    sample_rate: f32,
}

#[derive(Params, Lens)]
pub struct MetalXrossParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    #[id = "dist"] pub dist: FloatParam,
    #[id = "level"] pub level: FloatParam,
    #[id = "style"] pub style: FloatParam,
    
    #[id = "hpf_f"] pub hpf_freq: FloatParam,
    #[id = "hpf_q"] pub hpf_q: FloatParam,
    #[id = "low_f"] pub low_freq: FloatParam,
    #[id = "low_g"] pub low_gain: FloatParam,
    #[id = "low_q"] pub low_q: FloatParam,
    #[id = "mid_f"] pub mid_freq: FloatParam,
    #[id = "mid_g"] pub mid_gain: FloatParam,
    #[id = "mid_q"] pub mid_q: FloatParam,
    #[id = "high_f"] pub high_freq: FloatParam,
    #[id = "high_g"] pub high_gain: FloatParam,
    #[id = "high_q"] pub high_q: FloatParam,
    #[id = "lpf_f"] pub lpf_freq: FloatParam,
    #[id = "lpf_q"] pub lpf_q: FloatParam,
}

#[derive(Lens)]
struct EditorData {
    params: Arc<MetalXrossParams>,
}

impl Model for EditorData {}

impl Default for MetalXross {
    fn default() -> Self {
        Self {
            params: Arc::new(MetalXrossParams::default()),
            filters: [[Biquad::new(); 5]; 2],
            sample_rate: 44100.0,
        }
    }
}

impl Default for MetalXrossParams {
    fn default() -> Self {
        Self {
            editor_state: ViziaState::new(|| (800, 500)),
            dist: FloatParam::new("Distortion", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            level: FloatParam::new("Level", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            style: FloatParam::new("Style", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            hpf_freq: FloatParam::new("HPF Freq", 20.0, FloatRange::Skewed { min: 20.0, max: 2000.0, factor: FloatRange::skew_factor(-1.0) }),
            hpf_q: FloatParam::new("HPF Slope", 0.707, FloatRange::Linear { min: 0.1, max: 2.0 }),
            low_freq: FloatParam::new("Low Freq", 100.0, FloatRange::Skewed { min: 60.0, max: 250.0, factor: FloatRange::skew_factor(-1.0) }),
            low_gain: FloatParam::new("Low Gain", 0.0, FloatRange::Linear { min: -20.0, max: 20.0 }),
            low_q: FloatParam::new("Low Q", 0.707, FloatRange::Linear { min: 0.1, max: 2.0 }),
            mid_freq: FloatParam::new("Mid Freq", 1000.0, FloatRange::Skewed { min: 200.0, max: 5000.0, factor: FloatRange::skew_factor(-1.0) }),
            mid_gain: FloatParam::new("Mid Gain", 0.0, FloatRange::Linear { min: -20.0, max: 20.0 }),
            mid_q: FloatParam::new("Mid Q", 1.0, FloatRange::Linear { min: 0.1, max: 10.0 }),
            high_freq: FloatParam::new("High Freq", 5000.0, FloatRange::Skewed { min: 3000.0, max: 10000.0, factor: FloatRange::skew_factor(-1.0) }),
            high_gain: FloatParam::new("High Gain", 0.0, FloatRange::Linear { min: -20.0, max: 20.0 }),
            high_q: FloatParam::new("High Q", 0.707, FloatRange::Linear { min: 0.1, max: 2.0 }),
            lpf_freq: FloatParam::new("LPF Freq", 20000.0, FloatRange::Skewed { min: 1000.0, max: 20000.0, factor: FloatRange::skew_factor(-1.0) }),
            lpf_q: FloatParam::new("LPF Slope", 0.707, FloatRange::Linear { min: 0.1, max: 2.0 }),
        }
    }
}

impl Plugin for MetalXross {
    const NAME: &'static str = "Metal Xross";
    const VENDOR: &'static str = "Your Name";
    const URL: &'static str = "https://example.com";
    const EMAIL: &'static str = "your@email.com";
    const VERSION: &'static str = "0.1.0";
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[ AudioIOLayout { main_input_channels: std::num::NonZeroU32::new(2), main_output_channels: std::num::NonZeroU32::new(2), ..AudioIOLayout::const_default() } ];
    type SysExMessage = ();
    type BackgroundTask = ();
    fn params(&self) -> Arc<dyn Params> { self.params.clone() }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        create_vizia_editor(self.params.editor_state.clone(), ViziaTheming::Custom, move |cx, _| {
            EditorData { params: params.clone() }.build(cx);

            VStack::new(cx, |cx| {
                Label::new(cx, "Metal Xross").font_size(30.0).min_bottom(Pixels(20.0));
                
                HStack::new(cx, |cx| {
                    VStack::new(cx, |cx| { Label::new(cx, "DIST"); ParamSlider::new(cx, EditorData::params, |p| &p.dist); });
                    VStack::new(cx, |cx| { Label::new(cx, "LEVEL"); ParamSlider::new(cx, EditorData::params, |p| &p.level); });
                    VStack::new(cx, |cx| { Label::new(cx, "STYLE"); ParamSlider::new(cx, EditorData::params, |p| &p.style); });
                }).col_between(Pixels(20.0)).height(Pixels(100.0));

                Label::new(cx, "EQ & Filters").min_top(Pixels(20.0)).font_size(20.0);
                
                HStack::new(cx, |cx| {
                    VStack::new(cx, |cx| {
                        Label::new(cx, "HPF");
                        ParamSlider::new(cx, EditorData::params, |p| &p.hpf_freq);
                        ParamSlider::new(cx, EditorData::params, |p| &p.hpf_q);
                    }).width(Stretch(1.0));
                    VStack::new(cx, |cx| {
                        Label::new(cx, "LOW");
                        ParamSlider::new(cx, EditorData::params, |p| &p.low_freq);
                        ParamSlider::new(cx, EditorData::params, |p| &p.low_gain);
                        ParamSlider::new(cx, EditorData::params, |p| &p.low_q);
                    }).width(Stretch(1.0));
                    VStack::new(cx, |cx| {
                        Label::new(cx, "MID");
                        ParamSlider::new(cx, EditorData::params, |p| &p.mid_freq);
                        ParamSlider::new(cx, EditorData::params, |p| &p.mid_gain);
                        ParamSlider::new(cx, EditorData::params, |p| &p.mid_q);
                    }).width(Stretch(1.0));
                    VStack::new(cx, |cx| {
                        Label::new(cx, "HIGH");
                        ParamSlider::new(cx, EditorData::params, |p| &p.high_freq);
                        ParamSlider::new(cx, EditorData::params, |p| &p.high_gain);
                        ParamSlider::new(cx, EditorData::params, |p| &p.high_q);
                    }).width(Stretch(1.0));
                    VStack::new(cx, |cx| {
                        Label::new(cx, "LPF");
                        ParamSlider::new(cx, EditorData::params, |p| &p.lpf_freq);
                        ParamSlider::new(cx, EditorData::params, |p| &p.lpf_q);
                    }).width(Stretch(1.0));
                }).min_top(Pixels(15.0)).col_between(Pixels(15.0));
            }).child_space(Pixels(20.0));
        })
    }

    fn initialize(&mut self, _audio_io_layout: &AudioIOLayout, buffer_config: &BufferConfig, _context: &mut impl InitContext<Self>) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        for ch in &mut self.filters { for f in ch { f.reset(); } }
        true
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, _context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let sr = self.sample_rate;
        let p = &self.params;
        for ch in 0..buffer.channels() {
            self.filters[ch][0].set_hpf(p.hpf_freq.value(), sr, p.hpf_q.value());
            self.filters[ch][1].set_low_shelf(p.low_freq.value(), sr, p.low_gain.value(), p.low_q.value());
            self.filters[ch][2].set_peaking(p.mid_freq.value(), sr, p.mid_gain.value(), p.mid_q.value());
            self.filters[ch][3].set_high_shelf(p.high_freq.value(), sr, p.high_gain.value(), p.high_q.value());
            self.filters[ch][4].set_lpf(p.lpf_freq.value(), sr, p.lpf_q.value());
        }
        for channel_samples in buffer.iter_samples() {
            for (i, sample) in channel_samples.into_iter().enumerate() {
                let mut x = *sample * (1.0 + p.dist.value() * 20.0);
                let style = p.style.value();
                x = if style < 0.25 { (x * 0.5).tanh() * 2.0 } else if style < 0.5 { x.tanh() } else if style < 0.75 { x.clamp(-0.8, 0.8) } else { if x > 0.0 { 1.0 - (-x.abs()).exp() } else { - (1.0 - (-x.abs()).exp()) } };
                for f in &mut self.filters[i] { x = f.process(x); }
                *sample = x * p.level.value();
            }
        }
        ProcessStatus::Normal
    }
}

impl Vst3Plugin for MetalXross {
    const VST3_CLASS_ID: [u8; 16] = *b"MetalXrossPlugin";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}
impl ClapPlugin for MetalXross {
    const CLAP_ID: &'static str = "com.yourname.metal-xross";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Distortion with HPF/LPF and 3-band EQ");
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Distortion];
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
}
nih_export_vst3!(MetalXross);
nih_export_clap!(MetalXross);
