use nih_plug::prelude::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaTheming};
use std::sync::Arc;

use crate::params::MetalXrossParams;

#[derive(Lens)]
pub struct Data {
    pub params: Arc<MetalXrossParams>,
}

impl Model for Data {}

pub struct PcbBackground;

impl View for PcbBackground {
    fn element(&self) -> Option<&'static str> {
        Some("pcb-background")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let vw = bounds.width();
        let vh = bounds.height();
        let vmin = vw.min(vh);

        let mut paint = vg::Paint::color(vg::Color::hex("888888"));
        paint.set_line_width(1.1);
        paint.set_anti_alias(true);

        let bundles = ["topLeft", "bottomRight", "leftBottom", "topRight"];
        for &bundle_type in &bundles {
            let count = if bundle_type == "topLeft" || bundle_type == "bottomRight" { 3 } else { 2 };
            for i in 0..count {
                self.draw_bundle(canvas, &paint, bundle_type, i, vw, vh, vmin);
            }
        }
    }
}

impl PcbBackground {
    fn draw_bundle(
        &self,
        canvas: &mut Canvas,
        paint: &vg::Paint,
        bundle_type: &str,
        cluster_idx: usize,
        vw: f32,
        vh: f32,
        vmin: f32,
    ) {
        let line_count = if bundle_type == "topLeft" || bundle_type == "bottomRight" { 4 } else { 5 };
        let spacing = 12.0;
        let iheight = (0.5 * vmin) / 2.0;

        let (start_x, start_y) = match bundle_type {
            "topLeft" => (0.0, vh * 0.05 + cluster_idx as f32 * vh * 0.2),
            "bottomRight" => (vw, vh * 0.95 - cluster_idx as f32 * vh * 0.2),
            "leftBottom" => (0.0, vh * 0.7 + cluster_idx as f32 * vh * 0.2),
            "topRight" => (vw, vh * 0.3 - cluster_idx as f32 * vh * 0.2),
            _ => (0.0, 0.0),
        };

        let y_ratio = start_y / vh;
        let weight = if bundle_type.contains("left") { 1.0 - y_ratio } else { y_ratio };
        let base_len = vw * 0.35 * weight;

        for j in 0..line_count {
            let step = j as f32 * spacing;
            let x1 = start_x;
            let y1 = if bundle_type.contains("Right") { start_y - step } else { start_y + step };
            let (mut x2, mut y2, mut x3, mut y3) = (0.0, 0.0, 0.0, 0.0);
            let (mut x4, mut y4) = (None, None);

            let diag = vh * 0.12;
            let lim;

            match bundle_type {
                "topLeft" => {
                    x2 = base_len - step;
                    y2 = y1;
                    x3 = x2 + diag;
                    y3 = y2 + diag;
                    lim = (vh - iheight) / 2.0 - spacing;
                    if y3 < lim {
                        let d = vmin * 0.12;
                        x4 = Some(x3 + (d + step) * 2.0);
                        y4 = Some(y3);
                    }
                }
                "bottomRight" => {
                    x2 = vw - base_len + step;
                    y2 = y1;
                    x3 = x2 - diag;
                    y3 = y2 - diag;
                    lim = (vh + iheight) / 2.0 + spacing;
                    if y3 > lim {
                        let d = vmin * 0.12;
                        x4 = Some(x3 - (d + step) * 2.0);
                        y4 = Some(y3);
                    }
                }
                "leftBottom" => {
                    x2 = base_len * 0.5 - step;
                    y2 = y1;
                    y3 = vh;
                    x3 = x2 + (y3 - y2);
                }
                "topRight" => {
                    x2 = vw - base_len * 0.5 + step;
                    y2 = y1;
                    y3 = 0.0;
                    x3 = x2 - (y2 - y3);
                }
                _ => {}
            }

            let mut path = vg::Path::new();
            path.move_to(x1, y1);
            path.line_to(x2, y2);
            path.line_to(x3, y3);
            if let (Some(x4_val), Some(y4_val)) = (x4, y4) {
                path.line_to(x4_val, y4_val);
            }
            canvas.stroke_path(&path, paint);

            let mut dot_path = vg::Path::new();
            dot_path.circle(x3, y3, 1.3);
            canvas.fill_path(&dot_path, paint);
        }
    }
}

pub struct Logo;

impl View for Logo {
    fn element(&self) -> Option<&'static str> {
        Some("logo")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let vw = bounds.width();
        let vh = bounds.height();
        let size = vw.min(vh);
        let center_x = bounds.x + vw / 2.0;
        let center_y = bounds.y + vh / 2.0;

        // SVG Path: M45 45 l-5 -5 h-15 l-5 5 v10 l5 5 h15 l 20 -20 h15 l5 5 v10 l-5 5 h-15 l-5 -5
        // Normalized (0-100) to centered size
        let scale = size / 100.0;
        let offset_x = center_x - 50.0 * scale;
        let offset_y = center_y - 50.0 * scale;

        let mut path = vg::Path::new();
        path.move_to(offset_x + 45.0 * scale, offset_y + 45.0 * scale);
        path.line_to(offset_x + 40.0 * scale, offset_y + 40.0 * scale);
        path.line_to(offset_x + 25.0 * scale, offset_y + 40.0 * scale);
        path.line_to(offset_x + 20.0 * scale, offset_y + 45.0 * scale);
        path.line_to(offset_x + 20.0 * scale, offset_y + 55.0 * scale);
        path.line_to(offset_x + 25.0 * scale, offset_y + 60.0 * scale);
        path.line_to(offset_x + 40.0 * scale, offset_y + 60.0 * scale);
        path.line_to(offset_x + 60.0 * scale, offset_y + 40.0 * scale);
        path.line_to(offset_x + 75.0 * scale, offset_y + 40.0 * scale);
        path.line_to(offset_x + 80.0 * scale, offset_y + 45.0 * scale);
        path.line_to(offset_x + 80.0 * scale, offset_y + 55.0 * scale);
        path.line_to(offset_x + 75.0 * scale, offset_y + 60.0 * scale);
        path.line_to(offset_x + 60.0 * scale, offset_y + 60.0 * scale);
        path.line_to(offset_x + 55.0 * scale, offset_y + 55.0 * scale);

        let mut paint = vg::Paint::color(vg::Color::hex("888888"));
        paint.set_line_width(4.0 * scale);
        paint.set_anti_alias(true);
        canvas.stroke_path(&path, &paint);
    }
}

pub fn create(params: Arc<MetalXrossParams>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            assets::register_noto_sans_light(cx);

            Data {
                params: params.clone(),
            }
            .build(cx);

            ZStack::new(cx, |cx| {
                PcbBackground.build(cx, |_| {});

                Logo.build(cx, |_| {})
                    .width(Percentage(50.0))
                    .height(Percentage(50.0))
                    .left(Stretch(1.0))
                    .right(Stretch(1.0))
                    .top(Stretch(1.0))
                    .bottom(Stretch(1.0));

                VStack::new(cx, |cx| {
                    Label::new(cx, "Metal Xross")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_size(40.0)
                        .height(Pixels(60.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));

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
                    .col_between(Pixels(20.0))
                    .height(Auto);

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
                    .col_between(Pixels(20.0))
                    .height(Auto);
                })
                .row_between(Pixels(10.0))
                .child_space(Pixels(20.0))
                .left(Stretch(1.0))
                .right(Stretch(1.0))
                .top(Stretch(1.0))
                .bottom(Stretch(1.0))
                .pointer_events(PointerEvents::None);
            });
        },
    )
}
