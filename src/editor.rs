use nih_plug::params::FloatParam;
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Color32, Frame, RichText},
};
use std::sync::Arc;

use crate::editor::knob::{LinearSlider, SingleKnob, StackedKnob};
use crate::params::MetalXrossParams; // LinearSliderをインポート

pub mod background;
pub mod equalizer;
pub mod knob;

use background::PcbBackground;
use equalizer::EqualizerBox;

/// 彩度を最大化した鮮やかな色を生成する
fn from_hsv_vibrant(h: f32) -> Color32 {
    // S=1.0, V=1.0 にすることで、最も彩度が高い状態を維持する
    let hsva = egui::epaint::Hsva::new(h, 1.0, 1.0, 1.0);
    hsva.into()
}

struct KnobConfig<'a> {
    label: &'static str,
    param: KnobParam<'a>,
}

enum KnobParam<'a> {
    Stacked(&'a FloatParam, &'a FloatParam),
    Single(&'a FloatParam),
}
pub fn create(params: Arc<MetalXrossParams>) -> Option<Box<dyn nih_plug::prelude::Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |_cx, _state| {},
        move |egui_ctx, setter, _state| {
            egui::CentralPanel::default()
                .frame(Frame::NONE.fill(Color32::BLACK))
                .show(egui_ctx, |ui| {
                    // 背景描画
                    PcbBackground::draw(ui);

                    ui.vertical(|ui| {
                        // 1. 最上段: NOISE GATE (1行に凝縮)
                        ui.add_space(2.0);
                        let gate_frame = Frame::NONE
                            .fill(Color32::from_black_alpha(150))
                            .corner_radius(2.0)
                            .inner_margin(2.0);

                        gate_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let sw = (ui.available_width() - 20.0) / 3.0;
                                ui.add_sized(
                                    [sw, 16.0],
                                    LinearSlider::new(
                                        &params.noise_gate.threshold,
                                        setter,
                                        Color32::from_rgb(0, 255, 200),
                                    ),
                                );
                                ui.add_sized(
                                    [sw, 16.0],
                                    LinearSlider::new(
                                        &params.noise_gate.tolerance,
                                        setter,
                                        Color32::from_rgb(0, 200, 255),
                                    ),
                                );
                                ui.add_sized(
                                    [sw, 16.0],
                                    LinearSlider::new(
                                        &params.noise_gate.release,
                                        setter,
                                        Color32::from_rgb(100, 150, 255),
                                    ),
                                );
                            });
                        });

                        ui.add_space(4.0);

                        // 2. 中段: MAIN KNOBS (allocate_uiを使わず直接横並び)
                        let knob_configs = [
                            KnobConfig {
                                label: "IN",
                                param: KnobParam::Stacked(
                                    &params.general.input.gain,
                                    &params.general.input.limit,
                                ),
                            },
                            KnobConfig {
                                label: "GAIN",
                                param: KnobParam::Single(&params.general.gain),
                            },
                            KnobConfig {
                                label: "STYLE",
                                param: KnobParam::Single(&params.style.kind),
                            },
                            KnobConfig {
                                label: "LOW",
                                param: KnobParam::Single(&params.style.low),
                            },
                            KnobConfig {
                                label: "MID",
                                param: KnobParam::Single(&params.style.mid),
                            },
                            KnobConfig {
                                label: "HIGH",
                                param: KnobParam::Single(&params.style.high),
                            },
                            KnobConfig {
                                label: "OUT",
                                param: KnobParam::Stacked(
                                    &params.general.output.gain,
                                    &params.general.output.limit,
                                ),
                            },
                        ];

                        ui.horizontal(|ui| {
                            let total = knob_configs.len();
                            let spacing = ui.spacing().item_spacing.x;
                            let knob_w = (ui.available_width() - (spacing * (total - 1) as f32))
                                / total as f32;

                            for (i, config) in knob_configs.iter().enumerate() {
                                ui.vertical(|ui| {
                                    ui.set_width(knob_w);
                                    ui.vertical_centered(|ui| {
                                        let color = from_hsv_vibrant(i as f32 / total as f32);
                                        ui.label(
                                            RichText::new(config.label)
                                                .size(12.0)
                                                .color(Color32::WHITE),
                                        );

                                        // ノブ本体
                                        match config.param {
                                            KnobParam::Single(p) => {
                                                ui.add(SingleKnob::new(p, setter, color));
                                            }
                                            KnobParam::Stacked(p1, p2) => {
                                                ui.add(StackedKnob::new(
                                                    p1, p2, setter, color, color,
                                                ));
                                            }
                                        }
                                    });
                                });
                            }
                        });

                        ui.add_space(2.0);

                        // 3. 下段: EQUALIZER (タイトルを消して全開放)
                        // 残りの高さを計算（安全のために数ピクセル引く）
                        let eq_h = ui.available_height() - 4.0;
                        if eq_h > 0.0 {
                            EqualizerBox::draw(ui, &params, setter);
                        }
                    });
                });
        },
    )
}
