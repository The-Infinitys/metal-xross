use std::sync::Arc;

use crate::editor::knob::{LinearSlider, SingleKnob, StackedKnob};
use crate::params::MetalXrossParams;

pub mod background;
pub mod equalizer;
pub mod knob;

use background::PcbBackground;
use egui::{Color32, Frame, RichText};
use equalizer::EqualizerBox;
use truce::core::Editor;
use truce_egui::EguiEditor;

fn from_hsv_vibrant(h: f32) -> Color32 {
    let hsva = egui::epaint::Hsva::new(h, 1.0, 1.0, 1.0);
    hsva.into()
}

struct KnobConfig<'a> {
    label: &'static str,
    param: KnobParam<'a>,
}

enum KnobParam<'a> {
    Stacked(&'a truce::params::FloatParam, &'a truce::params::FloatParam),
    Single(&'a truce::params::FloatParam),
}

pub fn create(params: Arc<MetalXrossParams>) -> Box<dyn Editor> {
    let editor = EguiEditor::new((800, 500), move |ctx, _state| {
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::BLACK))
            .show(ctx, |ui| {
                // 背景
                PcbBackground::draw(ui);

                ui.vertical(|ui| {
                    // 1. Noise Gate (LinearSliders)
                    ui.add_space(2.0);
                    Frame::NONE
                        .fill(Color32::from_black_alpha(150))
                        .inner_margin(2.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let sw = (ui.available_width() - 20.0) / 3.0;
                                ui.add_sized(
                                    [sw, 16.0],
                                    LinearSlider::new(
                                        &params.gate_threshold,
                                        Color32::from_rgb(0, 255, 200),
                                    ),
                                );
                                ui.add_sized(
                                    [sw, 16.0],
                                    LinearSlider::new(
                                        &params.gate_tolerance,
                                        Color32::from_rgb(0, 200, 255),
                                    ),
                                );
                                ui.add_sized(
                                    [sw, 16.0],
                                    LinearSlider::new(
                                        &params.gate_release,
                                        Color32::from_rgb(100, 150, 255),
                                    ),
                                );
                            });
                        });

                    ui.add_space(4.0);

                    // 2. Main Knobs
                    let knob_configs = [
                        KnobConfig {
                            label: "IN",
                            param: KnobParam::Stacked(&params.input_gain, &params.input_limit),
                        },
                        KnobConfig {
                            label: "DRIVE",
                            param: KnobParam::Single(&params.gain),
                        },
                        KnobConfig {
                            label: "STYLE",
                            param: KnobParam::Single(&params.style_kind),
                        },
                        KnobConfig {
                            label: "LOW",
                            param: KnobParam::Single(&params.style_low),
                        },
                        KnobConfig {
                            label: "MID",
                            param: KnobParam::Single(&params.style_mid),
                        },
                        KnobConfig {
                            label: "HIGH",
                            param: KnobParam::Single(&params.style_high),
                        },
                        KnobConfig {
                            label: "OUT",
                            param: KnobParam::Stacked(&params.output_gain, &params.output_limit),
                        },
                    ];

                    ui.horizontal(|ui| {
                        let total = knob_configs.len();
                        let spacing = ui.spacing().item_spacing.x;
                        let knob_w =
                            (ui.available_width() - (spacing * (total - 1) as f32)) / total as f32;

                        for (i, config) in knob_configs.iter().enumerate() {
                            ui.vertical(|ui| {
                                ui.set_width(knob_w);
                                ui.vertical_centered(|ui| {
                                    let color = from_hsv_vibrant(i as f32 / total as f32);
                                    ui.label(
                                        RichText::new(config.label)
                                            .size(11.0)
                                            .color(Color32::WHITE),
                                    );
                                    match config.param {
                                        KnobParam::Single(p) => ui.add(SingleKnob::new(p, color)),
                                        KnobParam::Stacked(p1, p2) => ui.add(StackedKnob::new(
                                            p1,
                                            p2,
                                            color,
                                            color.linear_multiply(0.7),
                                        )),
                                    };
                                });
                            });
                        }
                    });

                    ui.add_space(4.0);

                    // 3. EQ Section
                    let eq_h = ui.available_height() - 4.0;
                    if eq_h > 0.0 {
                        EqualizerBox::draw(ui, &params);
                    }
                });
            });
    });
    Box::new(editor)
}
