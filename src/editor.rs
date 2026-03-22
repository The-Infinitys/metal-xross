use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::{ViziaTheming, assets, create_vizia_editor};
use std::sync::Arc;

use crate::params::MetalXrossParams;

pub mod background;
pub mod equalizer;
pub mod knob;

use background::PcbBackground;
use equalizer::EqualizerBox;
use knob::CustomKnob;

#[derive(Lens)]
pub struct Data {
    pub params: Arc<MetalXrossParams>,
    // 現在のウィンドウサイズを追跡（詳細度変更用）
    pub width: f32,
    pub height: f32,
}

impl Model for Data {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _| match window_event {
            WindowEvent::GeometryChanged(geo) => {
                if geo.contains(GeoChanged::WIDTH_CHANGED)
                    || geo.contains(GeoChanged::HEIGHT_CHANGED)
                {
                    self.width = cx.cache.get_width(cx.current());
                    self.height = cx.cache.get_height(cx.current());
                }
            }
            _ => {}
        });
    }
}

pub fn create(params: Arc<MetalXrossParams>) -> Option<Box<dyn nih_plug::prelude::Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            assets::register_noto_sans_light(cx);

            Data {
                params: params.clone(),
                width: 800.0,
                height: 500.0,
            }
            .build(cx);

            ZStack::new(cx, |cx| {
                PcbBackground::new(cx);

                VStack::new(cx, |cx| {
                    // 上段: ノブ3つ（横並び固定）
                    HStack::new(cx, |cx| {
                        knob_with_label(cx, "GAIN", 0);
                        knob_with_label(cx, "STYLE", 1);
                        knob_with_label(cx, "LEVEL", 2);
                    })
                    .height(Pixels(150.0))
                    .width(Stretch(1.0))
                    .col_between(Stretch(1.0)) // リサイズ時にノブの間隔が広がるように
                    .child_left(Stretch(1.0))
                    .child_right(Stretch(1.0));

                    // 下段: イコライザー
                    VStack::new(cx, |cx| {
                        Label::new(cx, "VISUALIZER").font_size(14.0);
                        EqualizerBox::new(cx)
                            .width(Stretch(1.0))
                            .height(Stretch(1.0));
                    })
                    .width(Stretch(1.0))
                    .height(Stretch(1.0))
                    .row_between(Pixels(10.0));
                })
                .child_space(Pixels(20.0))
                .row_between(Pixels(20.0));
            });
        },
    )
}

fn knob_with_label(cx: &mut Context, label: &'static str, index: usize) {
    VStack::new(cx, move |cx| {
        Label::new(cx, label).font_size(12.0);
        match index {
            0 => CustomKnob::new(cx, Data::params, |p| &p.gain),
            1 => CustomKnob::new(cx, Data::params, |p| &p.style),
            _ => CustomKnob::new(cx, Data::params, |p| &p.level),
        };
    })
    .width(Pixels(150.0))
    .height(Pixels(150.0))
    .child_space(Stretch(1.0));
}
