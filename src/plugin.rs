use nih_plug::prelude::*;
use std::num::NonZeroU32;

use crate::MetalXross;

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
        crate::editor::create(self.params())
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
