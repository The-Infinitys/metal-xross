use nih_plug::params::FloatParam;
use nih_plug::prelude::*;
use nih_plug_egui::egui;

pub struct LinearSlider<'a> {
    param: &'a FloatParam,
    setter: &'a ParamSetter<'a>,
    color: egui::Color32,
}

impl<'a> LinearSlider<'a> {
    pub fn new(param: &'a FloatParam, setter: &'a ParamSetter<'a>, color: egui::Color32) -> Self {
        Self {
            param,
            setter,
            color,
        }
    }
}

impl<'a> egui::Widget for LinearSlider<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = egui::vec2(120.0, 24.0);
        let (rect, response) = ui.allocate_at_least(desired_size, egui::Sense::click_and_drag());

        let id = response.id;
        let text_edit_id = id.with("text_edit");
        let edit_string_id = id.with("edit_string");

        let mut is_editing_text =
            ui.memory(|mem| mem.data.get_temp::<bool>(text_edit_id).unwrap_or(false));

        let is_vertical = rect.height() > rect.width();

        // --- 1. インタラクション (変更なし) ---
        if response.double_clicked() {
            self.setter.begin_set_parameter(self.param);
            self.setter
                .set_parameter_normalized(self.param, self.param.default_normalized_value());
            self.setter.end_set_parameter(self.param);

            is_editing_text = false;
            ui.memory_mut(|mem| {
                mem.data.insert_temp(text_edit_id, false);
                mem.data.remove::<String>(edit_string_id);
            });
        }

        if response.drag_started() {
            self.setter.begin_set_parameter(self.param);
        }

        if response.dragged() && !is_editing_text {
            let val = self.param.unmodulated_normalized_value();
            let delta = if is_vertical {
                -response.drag_delta().y / rect.height()
            } else {
                response.drag_delta().x / rect.width()
            };
            if delta != 0.0 {
                let new_val = (val + delta).clamp(0.0, 1.0);
                self.setter.set_parameter_normalized(self.param, new_val);
            }
        }

        if response.drag_stopped() {
            self.setter.end_set_parameter(self.param);
        }

        // --- 2. 描画とテキスト編集 ---
        if ui.is_rect_visible(rect) {
            let visual_val = self.param.unmodulated_normalized_value();
            let bar_color = self.color.linear_multiply(0.6);
            let text_rect = rect.shrink(2.0);

            // A. 背景・バー・枠の描画 (Painterを一時的に借りてすぐ返す)
            {
                let painter = ui.painter();
                painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(5, 5, 5));

                let fill_rect = if is_vertical {
                    let y_pos = rect.bottom() - (visual_val * rect.height());
                    egui::Rect::from_min_max(egui::pos2(rect.left(), y_pos), rect.right_bottom())
                } else {
                    let x_pos = rect.left() + (visual_val * rect.width());
                    egui::Rect::from_min_max(rect.left_top(), egui::pos2(x_pos, rect.bottom()))
                };
                painter.rect_filled(fill_rect, 1.0, bar_color);

                painter.rect_stroke(
                    rect,
                    2.0,
                    egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                    egui::StrokeKind::Inside,
                );
            } // ここで painter (不変借用) がドロップされる

            // B. テキストエリアの判定
            let text_res = ui.interact(text_rect, id.with("text_area"), egui::Sense::click());
            if text_res.clicked() {
                is_editing_text = true;
                ui.memory_mut(|mem| mem.data.insert_temp(text_edit_id, true));
            }

            // C. 編集モードまたは通常テキストの描画
            if is_editing_text {
                let mut value_text = ui.memory(|mem| {
                    mem.data
                        .get_temp::<String>(edit_string_id)
                        .unwrap_or_else(|| format!("{:.2}", self.param.value()))
                });

                let output = ui.put(
                    text_rect,
                    egui::TextEdit::singleline(&mut value_text)
                        .font(egui::FontId::proportional(11.0))
                        .text_color(egui::Color32::WHITE)
                        .horizontal_align(egui::Align::Center)
                        .frame(false),
                );

                if output.changed() {
                    ui.memory_mut(|mem| mem.data.insert_temp(edit_string_id, value_text.clone()));
                }

                if output.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
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
                // 通常テキスト描画 (もう一度 Painter を借りる)
                let painter = ui.painter();
                let text = format!("{}: {}", self.param.name(), self.param);
                let font_id = egui::FontId::proportional(11.0);
                let text_pos = rect.center();

                let fill_rect = if is_vertical {
                    let y_pos = rect.bottom() - (visual_val * rect.height());
                    egui::Rect::from_min_max(egui::pos2(rect.left(), y_pos), rect.right_bottom())
                } else {
                    let x_pos = rect.left() + (visual_val * rect.width());
                    egui::Rect::from_min_max(rect.left_top(), egui::pos2(x_pos, rect.bottom()))
                };

                painter.text(
                    text_pos + egui::vec2(1.0, 1.0),
                    egui::Align2::CENTER_CENTER,
                    &text,
                    font_id.clone(),
                    egui::Color32::from_black_alpha(200),
                );
                painter.with_clip_rect(rect).text(
                    text_pos,
                    egui::Align2::CENTER_CENTER,
                    &text,
                    font_id.clone(),
                    egui::Color32::from_gray(180),
                );
                painter.with_clip_rect(fill_rect).text(
                    text_pos,
                    egui::Align2::CENTER_CENTER,
                    &text,
                    font_id,
                    egui::Color32::WHITE,
                );
            }

            // D. ハンドルの描画 (最後にもう一度借用)
            {
                let painter = ui.painter();
                let handle_rect = if is_vertical {
                    let y = (rect.bottom() - (visual_val * rect.height()))
                        .clamp(rect.top() + 1.0, rect.bottom() - 1.0);
                    egui::Rect::from_center_size(
                        egui::pos2(rect.center().x, y),
                        egui::vec2(rect.width(), 2.0),
                    )
                } else {
                    let x = (rect.left() + (visual_val * rect.width()))
                        .clamp(rect.left() + 1.0, rect.right() - 1.0);
                    egui::Rect::from_center_size(
                        egui::pos2(x, rect.center().y),
                        egui::vec2(2.0, rect.height()),
                    )
                };
                painter.rect_filled(handle_rect, 0.0, egui::Color32::WHITE);
            }
        }

        if response.dragged() || is_editing_text {
            ui.ctx().request_repaint();
        }

        response
    }
}
