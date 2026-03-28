use nih_plug::params::FloatParam;
use nih_plug::prelude::*;
use nih_plug_egui::egui;
use std::f32::consts::PI;

pub struct StackedKnob<'a> {
    upper_param: &'a FloatParam,
    lower_param: &'a FloatParam,
    setter: &'a ParamSetter<'a>,
    upper_color: egui::Color32,
    lower_color: egui::Color32,
}

impl<'a> StackedKnob<'a> {
    pub fn new(
        upper_param: &'a FloatParam,
        lower_param: &'a FloatParam,
        setter: &'a ParamSetter<'a>,
        upper_color: egui::Color32,
        lower_color: egui::Color32,
    ) -> Self {
        Self {
            upper_param,
            lower_param,
            setter,
            upper_color,
            lower_color,
        }
    }

    fn draw_arc(
        painter: &egui::Painter,
        center: egui::Pos2,
        radius: f32,
        val: f32,
        color: egui::Color32,
    ) {
        let start_angle = PI * 0.75;
        let end_angle = PI * 2.25;
        if val > 0.0 {
            let n_points = 32;
            let current_n = (n_points as f32 * val).ceil() as usize;
            let points: Vec<egui::Pos2> = (0..=current_n)
                .map(|i| {
                    let a = start_angle + (i as f32 / n_points as f32) * (end_angle - start_angle);
                    center + egui::vec2(a.cos(), a.sin()) * radius
                })
                .collect();
            painter.add(egui::Shape::line(points, egui::Stroke::new(3.0, color)));
        }
    }

    // クロージャからメソッドに変更して所有権問題を回避
    #[allow(clippy::too_many_arguments)]
    fn draw_value_edit(
        &self,
        ui: &mut egui::Ui,
        center: egui::Pos2,
        p: &FloatParam,
        y_off: f32,
        outer_radius: f32,
        edit_id: egui::Id,
        color: egui::Color32,
    ) -> bool {
        let mut is_editing = ui.memory(|m| m.data.get_temp::<bool>(edit_id).unwrap_or(false));
        let rect_center = center + egui::vec2(0.0, outer_radius + y_off);
        let text_rect = egui::Rect::from_center_size(rect_center, egui::vec2(70.0, 16.0));
        let font_id = egui::FontId::proportional(11.0);
        let bg_black = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200);
        let painter = ui.painter();

        painter.rect_filled(text_rect.expand(1.0), 2.0, bg_black);
        painter.circle_filled(text_rect.left_center() + egui::vec2(6.0, 0.0), 3.0, color);

        let area_res = ui.interact(text_rect, edit_id, egui::Sense::click());
        if area_res.clicked() {
            is_editing = true;
            ui.memory_mut(|m| m.data.insert_temp(edit_id, true));
        }

        if is_editing {
            let mut val_str = ui.memory(|m| {
                m.data
                    .get_temp::<String>(edit_id.with("s"))
                    .unwrap_or_else(|| p.to_string())
            });

            let output = ui.put(
                text_rect,
                egui::TextEdit::singleline(&mut val_str)
                    .font(font_id)
                    .frame(false)
                    .horizontal_align(egui::Align::Center),
            );

            if output.changed() {
                ui.memory_mut(|m| m.data.insert_temp(edit_id.with("s"), val_str.clone()));
            }

            if output.lost_focus() {
                if let Ok(v) = val_str.parse::<f32>() {
                    self.setter.begin_set_parameter(p);
                    self.setter
                        .set_parameter_normalized(p, p.preview_normalized(v));
                    self.setter.end_set_parameter(p);
                }
                ui.memory_mut(|m| {
                    m.data.insert_temp(edit_id, false);
                    m.data.remove::<String>(edit_id.with("s"));
                });
                return false;
            } else {
                output.request_focus();
            }
        } else {
            ui.painter().text(
                rect_center,
                egui::Align2::CENTER_CENTER,
                format!("{}: {}", p.name(), p),
                font_id,
                egui::Color32::WHITE,
            );
        }
        is_editing
    }
}

impl<'a> egui::Widget for StackedKnob<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = egui::vec2(90.0, 140.0);
        let (rect, response) = ui.allocate_at_least(desired_size, egui::Sense::click_and_drag());

        let center = rect.center() + egui::vec2(0.0, -25.0);
        let inner_radius = 20.0;
        let outer_radius = 32.0;

        let target_id = response.id.with("target");
        let edit_upper_id = response.id.with("edit_upper");
        let edit_lower_id = response.id.with("edit_lower");

        let mut active_target = ui.memory(|mem| mem.data.get_temp::<u8>(target_id).unwrap_or(0));

        // --- インタラクション ---
        if response.drag_started() {
            let pos = response.interact_pointer_pos().unwrap_or(center);
            active_target = if pos.distance(center) <= inner_radius + 4.0 {
                1
            } else {
                2
            };
            ui.memory_mut(|mem| mem.data.insert_temp(target_id, active_target));

            let p = if active_target == 1 {
                self.upper_param
            } else {
                self.lower_param
            };
            self.setter.begin_set_parameter(p);
        }

        if response.double_clicked() {
            let pos = response.interact_pointer_pos().unwrap_or(center);
            let target = if pos.distance(center) <= inner_radius + 4.0 {
                self.upper_param
            } else {
                self.lower_param
            };
            self.setter.begin_set_parameter(target);
            self.setter
                .set_parameter_normalized(target, target.default_normalized_value());
            self.setter.end_set_parameter(target);
        }

        if response.dragged() && active_target != 0 {
            let p = if active_target == 1 {
                self.upper_param
            } else {
                self.lower_param
            };
            let delta = -response.drag_delta().y * 0.005;
            let val = (p.unmodulated_normalized_value() + delta).clamp(0.0, 1.0);
            self.setter.set_parameter_normalized(p, val);
        }

        if response.drag_stopped() {
            let p = if active_target == 1 {
                self.upper_param
            } else {
                self.lower_param
            };
            self.setter.end_set_parameter(p);
            ui.memory_mut(|mem| mem.data.insert_temp(target_id, 0u8));
        }

        // --- 描画 ---
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let start_angle = PI * 0.75;
            let end_angle = PI * 2.25;

            // 下段
            painter.circle_filled(center, outer_radius, egui::Color32::from_rgb(15, 15, 15));
            painter.circle_stroke(
                center,
                outer_radius,
                egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            );
            Self::draw_arc(
                painter,
                center,
                outer_radius + 4.0,
                self.lower_param.unmodulated_normalized_value(),
                self.lower_color,
            );

            // 上段
            painter.circle_filled(center, inner_radius, egui::Color32::from_rgb(5, 5, 5));
            painter.circle_stroke(
                center,
                inner_radius,
                egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
            );
            Self::draw_arc(
                painter,
                center,
                inner_radius + 3.0,
                self.upper_param.unmodulated_normalized_value(),
                self.upper_color,
            );

            // 指針
            let draw_needle = |p: &FloatParam, r_min: f32, r_max: f32, color: egui::Color32| {
                let ang =
                    start_angle + p.unmodulated_normalized_value() * (end_angle - start_angle);
                painter.line_segment(
                    [
                        center + egui::vec2(ang.cos(), ang.sin()) * r_min,
                        center + egui::vec2(ang.cos(), ang.sin()) * r_max,
                    ],
                    egui::Stroke::new(2.5, color),
                );
            };
            draw_needle(
                self.lower_param,
                inner_radius + 5.0,
                outer_radius - 2.0,
                self.lower_color,
            );
            draw_needle(self.upper_param, 2.0, inner_radius - 2.0, self.upper_color);
        }

        // ラベル描画とエディットモード判定
        let editing_upper = self.draw_value_edit(
            ui,
            center,
            self.upper_param,
            22.0,
            outer_radius,
            edit_upper_id,
            self.upper_color,
        );
        let editing_lower = self.draw_value_edit(
            ui,
            center,
            self.lower_param,
            42.0,
            outer_radius,
            edit_lower_id,
            self.lower_color,
        );

        if response.dragged() || editing_upper || editing_lower {
            ui.ctx().request_repaint();
        }
        response
    }
}
