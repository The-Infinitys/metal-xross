use nih_plug_egui::{create_egui_editor, egui};
use std::sync::Arc;

use crate::params::MetalXrossParams;

pub mod background;
pub mod equalizer;
pub mod knob;

use background::PcbBackground;
use equalizer::EqualizerBox;
use knob::SingleKnob;

pub fn create(params: Arc<MetalXrossParams>) -> Option<Box<dyn nih_plug::prelude::Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |_cx, _state| {},
        move |egui_ctx, setter, _state| {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(egui::Color32::BLACK))
                .show(egui_ctx, |ui| {
                    // 背景のグリッド描画
                    PcbBackground::draw(ui);

                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);

                        // 上段: ノブ3つ
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 40.0;
                            ui.columns(3, |columns| {
                                columns[0].vertical_centered(|ui| {
                                    ui.label(
                                        egui::RichText::new("GAIN")
                                            .size(14.0)
                                            .color(egui::Color32::WHITE),
                                    );
                                    ui.add(SingleKnob::new(
                                        &params.gain,
                                        setter,
                                        egui::Color32::from_rgb(255, 255, 0),
                                    ));
                                });
                                columns[1].vertical_centered(|ui| {
                                    ui.label(
                                        egui::RichText::new("STYLE")
                                            .size(14.0)
                                            .color(egui::Color32::WHITE),
                                    );
                                    ui.add(SingleKnob::new(
                                        &params.style,
                                        setter,
                                        egui::Color32::from_rgb(0, 255, 255),
                                    ));
                                });
                                columns[2].vertical_centered(|ui| {
                                    ui.label(
                                        egui::RichText::new("LEVEL")
                                            .size(14.0)
                                            .color(egui::Color32::WHITE),
                                    );
                                    ui.add(SingleKnob::new(
                                        &params.level,
                                        setter,
                                        egui::Color32::from_rgb(255, 0, 255),
                                    ));
                                });
                            });
                        });

                        ui.add_space(30.0);

                        // 下段: イコライザー/ビジュアライザー
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("VISUALIZER")
                                    .size(14.0)
                                    .color(egui::Color32::WHITE),
                            );
                            ui.add_space(10.0);
                            EqualizerBox::draw(ui);
                        });
                    });
                });
        },
    )
}
