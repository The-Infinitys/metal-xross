use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{ViziaState, ViziaTheming, assets, create_vizia_editor};
use std::num::NonZeroU32;
use std::sync::Arc;

use crate::MetalXross;
use crate::params::MetalXrossParams;

#[derive(Lens)]
struct Data {
    params: Arc<MetalXrossParams>,
}

impl Model for Data {}

impl Plugin for MetalXross {
    const NAME: &'static str = "Metal Xross";

    const VENDOR: &'static str = "The Infinitys";

    const URL: &'static str = "https://github.com/The-Infinitys/metal-xross";

    const EMAIL: &'static str = "contact@theinfinitys.com";

    const VERSION: &'static str = "2.0.0";

    const AUDIO_IO_LAYOUTS: &'static [nih_plug::prelude::AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> std::sync::Arc<dyn nih_plug::prelude::Params> {
        self.params()
    }

    fn process(
        &mut self,
        buffer: &mut nih_plug::prelude::Buffer,
        aux: &mut nih_plug::prelude::AuxiliaryBuffers,
        context: &mut impl nih_plug::prelude::ProcessContext<Self>,
    ) -> nih_plug::prelude::ProcessStatus {
        self.process(buffer, aux, context)
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params();
        create_vizia_editor(
            params.editor_state.clone(),
            ViziaTheming::Custom,
            move |cx, _| {
                assets::register_noto_sans_light(cx);

                Data {
                    params: params.clone(),
                }
                .build(cx);

                VStack::new(cx, |cx| {
                    Label::new(cx, "Metal Xross")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_size(40.0)
                        .height(Pixels(60.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));

                    Label::new(cx, "Main Parameters").font_size(20.0);
                    HStack::new(cx, |cx| {
                        VStack::new(cx, |cx| {
                            Label::new(cx, "Gain");
                            ParamSlider::new(cx, Data::params, |params| &params.gain);
                            Label::new(cx, "Level");
                            ParamSlider::new(cx, Data::params, |params| &params.level);
                        })
                        .row_between(Pixels(5.0));
                        VStack::new(cx, |cx| {
                            Label::new(cx, "Style");
                            ParamSlider::new(cx, Data::params, |params| &params.style);
                            Label::new(cx, "Tight");
                            ParamSlider::new(cx, Data::params, |params| &params.tight);
                            Label::new(cx, "Bright");
                            ParamSlider::new(cx, Data::params, |params| &params.bright);
                        })
                        .row_between(Pixels(5.0));
                    })
                    .col_between(Pixels(20.0));

                    Label::new(cx, "Equalizer").font_size(20.0);
                    HStack::new(cx, |cx| {
                        VStack::new(cx, |cx| {
                            Label::new(cx, "Low");
                            ParamSlider::new(cx, Data::params, |params| &params.eq.low.gain);
                            ParamSlider::new(cx, Data::params, |params| &params.eq.low.freq);
                        })
                        .row_between(Pixels(5.0));
                        VStack::new(cx, |cx| {
                            Label::new(cx, "Mid");
                            ParamSlider::new(cx, Data::params, |params| &params.eq.mid.gain);
                            ParamSlider::new(cx, Data::params, |params| &params.eq.mid.freq);
                        })
                        .row_between(Pixels(5.0));
                        VStack::new(cx, |cx| {
                            Label::new(cx, "High");
                            ParamSlider::new(cx, Data::params, |params| &params.eq.high.gain);
                            ParamSlider::new(cx, Data::params, |params| &params.eq.high.freq);
                        })
                        .row_between(Pixels(5.0));
                    })
                    .col_between(Pixels(20.0));
                })
                .row_between(Pixels(10.0))
                .child_space(Pixels(20.0));
            },
        )
    }
}
// 2. VST3固有の設定
impl Vst3Plugin for MetalXross {
    const VST3_CLASS_ID: [u8; 16] = *b"MetalXross-ver2s";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}

// 3. Clap固有の設定（必要なら）
impl ClapPlugin for MetalXross {
    const CLAP_ID: &'static str = "org.infinite.metalxross";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("High-gain distortion plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::AudioEffect, ClapFeature::Distortion];
}

nih_export_clap!(MetalXross);
nih_export_vst3!(MetalXross);
