use nih_plug::params::FloatParam;
use nih_plug::prelude::*;
use nih_plug_egui::egui;
use std::f32::consts::PI;

pub struct SingleKnob<'a> {
    param: &'a FloatParam,
    setter: &'a ParamSetter<'a>,
    color: egui::Color32,
}

impl<'a> SingleKnob<'a> {
    pub fn new(param: &'a FloatParam, setter: &'a ParamSetter<'a>, color: egui::Color32) -> Self {
        Self {
            param,
            setter,
            color,
        }
    }
}

impl<'a> egui::Widget for SingleKnob<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = egui::vec2(80.0, 100.0);

        // 【修正ポイント1】 Sense を click_and_drag に変更
        let (rect, response) = ui.allocate_at_least(desired_size, egui::Sense::click_and_drag());

        let id = response.id;
        let text_edit_id = id.with("text_edit");
        let edit_string_id = id.with("edit_string");

        let mut is_editing_text =
            ui.memory(|mem| mem.data.get_temp::<bool>(text_edit_id).unwrap_or(false));

        // --- 1. インタラクション (ダブルクリック / ドラッグ) ---

        // 【修正ポイント2】 ダブルクリックによるリセットを優先
        if response.double_clicked() {
            let default_val = self.param.default_normalized_value();
            self.setter.begin_set_parameter(self.param);
            self.setter
                .set_parameter_normalized(self.param, default_val);
            self.setter.end_set_parameter(self.param);

            // メモリ上の値をリセット
            ui.memory_mut(|mem| {
                mem.data.insert_temp(id, default_val);
                mem.data.insert_temp(text_edit_id, false);
                mem.data.remove::<String>(edit_string_id);
            });
            is_editing_text = false;
        }
        if response.drag_started() {
            self.setter.begin_set_parameter(self.param);
        }

        // ドラッグによる値の更新
        let visual_val = if response.dragged() && !is_editing_text {
            let mut val: f32 = ui
                .memory(|mem| mem.data.get_temp(id))
                .unwrap_or_else(|| self.param.unmodulated_normalized_value());

            let delta = -response.drag_delta().y * 0.005;
            if delta != 0.0 {
                val = (val + delta).clamp(0.0, 1.0);
                self.setter.set_parameter_normalized(self.param, val);
                ui.memory_mut(|mem| mem.data.insert_temp(id, val));
            }
            val
        } else {
            let val = self.param.unmodulated_normalized_value();
            ui.memory_mut(|mem| mem.data.insert_temp(id, val));
            val
        };

        if response.drag_stopped() {
            self.setter.end_set_parameter(self.param);
        }

        // --- 2. 描画ロジック ---
        if ui.is_rect_visible(rect) {
            let center = rect.center() + egui::vec2(0.0, -10.0);
            let radius = 30.0;

            {
                let painter = ui.painter();
                let start_angle = PI * 0.75;
                let end_angle = PI * 2.25;
                let current_angle = start_angle + (visual_val * (end_angle - start_angle));
                let angle_to_pos = |ang: f32, r: f32| center + egui::vec2(ang.cos(), ang.sin()) * r;

                painter.circle_filled(center, radius, egui::Color32::from_rgb(15, 15, 15));
                painter.circle_stroke(
                    center,
                    radius,
                    egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                );

                if visual_val > 0.0 {
                    let n_points = 40;
                    let current_n = (n_points as f32 * visual_val).ceil() as usize;
                    let val_points: Vec<egui::Pos2> = (0..=current_n)
                        .map(|i| {
                            let a = start_angle
                                + (i as f32 / n_points as f32) * (end_angle - start_angle);
                            angle_to_pos(a, radius + 5.0)
                        })
                        .collect();
                    painter.add(egui::Shape::line(
                        val_points,
                        egui::Stroke::new(3.5, self.color),
                    ));
                }

                let tip = angle_to_pos(current_angle, radius * 0.9);
                let base = angle_to_pos(current_angle, radius * 0.2);
                painter.line_segment([base, tip], egui::Stroke::new(2.5, self.color));

                painter.text(
                    center + egui::vec2(0.0, radius + 40.0),
                    egui::Align2::CENTER_CENTER,
                    self.param.name(),
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_gray(180),
                );
            }

            // --- 3. 数値エリアの処理 ---
            let text_rect = egui::Rect::from_center_size(
                center + egui::vec2(0.0, radius + 25.0),
                egui::vec2(60.0, 16.0),
            );

            // 数値部分のクリック判定
            let text_area_res = ui.interact(text_rect, id.with("text_area"), egui::Sense::click());

            if text_area_res.double_clicked() {
                // ここでもダブルクリックを拾えるようにする（数値エリアをダブルクリックした場合）
                let default_val = self.param.default_normalized_value();
                self.setter.begin_set_parameter(self.param);
                self.setter
                    .set_parameter_normalized(self.param, default_val);
                self.setter.end_set_parameter(self.param);
                is_editing_text = false;
                ui.memory_mut(|mem| {
                    mem.data.insert_temp(text_edit_id, false);
                    mem.data.remove::<String>(edit_string_id);
                });
            } else if text_area_res.clicked() {
                is_editing_text = true;
                ui.memory_mut(|mem| mem.data.insert_temp(text_edit_id, true));
            }

            if is_editing_text {
                let mut value_text = ui.memory(|mem| {
                    mem.data
                        .get_temp::<String>(edit_string_id)
                        .unwrap_or_else(|| format!("{:.2}", self.param.value()))
                });

                let output = ui.put(
                    text_rect,
                    egui::TextEdit::singleline(&mut value_text)
                        .font(egui::FontId::proportional(12.0))
                        .horizontal_align(egui::Align::Center),
                );

                if output.changed() {
                    ui.memory_mut(|mem| mem.data.insert_temp(edit_string_id, value_text.clone()));
                }

                if output.lost_focus() {
                    if let Ok(parsed) = value_text.parse::<f32>() {
                        let norm_val = self.param.preview_normalized(parsed);
                        self.setter.begin_set_parameter(self.param);
                        self.setter.set_parameter_normalized(self.param, norm_val);
                        self.setter.end_set_parameter(self.param);
                    }
                    ui.memory_mut(|mem| {
                        mem.data.insert_temp(text_edit_id, false);
                        mem.data.remove::<String>(edit_string_id);
                    });
                    is_editing_text = false;
                } else {
                    output.request_focus();
                }
            } else {
                ui.painter().text(
                    text_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    self.param.to_string(),
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );
            }
        }

        if response.dragged() || is_editing_text {
            ui.ctx().request_repaint();
        }

        response
    }
}
