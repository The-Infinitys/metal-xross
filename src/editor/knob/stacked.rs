use crate::utils::FloatParamNormalizedExt;
use egui::{
    Align2, Color32, FontId, Id, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Widget, vec2,
};
use std::f32::consts::PI;

pub struct StackedKnob<'a> {
    upper_param: &'a truce::params::FloatParam, // Inner
    lower_param: &'a truce::params::FloatParam, // Outer
    upper_color: Color32,
    lower_color: Color32,
}

impl<'a> StackedKnob<'a> {
    pub fn new(
        upper_param: &'a truce::params::FloatParam,
        lower_param: &'a truce::params::FloatParam,
        upper_color: Color32,
        lower_color: Color32,
    ) -> Self {
        Self {
            upper_param,
            lower_param,
            upper_color,
            lower_color,
        }
    }

    fn draw_arc(painter: &egui::Painter, center: Pos2, radius: f32, val: f32, color: Color32) {
        let start_angle = PI * 0.75;
        let end_angle = PI * 2.25;
        if val > 0.0 {
            let n_points = 32;
            let current_n = (n_points as f32 * val).ceil() as usize;
            let points: Vec<Pos2> = (0..=current_n)
                .map(|i| {
                    let a = start_angle + (i as f32 / n_points as f32) * (end_angle - start_angle);
                    center + vec2(a.cos(), a.sin()) * radius
                })
                .collect();
            painter.add(Shape::line(points, Stroke::new(3.0, color)));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_value_edit(
        &self,
        ui: &mut Ui,
        center: Pos2,
        p: &truce::params::FloatParam,
        y_off: f32,
        outer_radius: f32,
        edit_id: Id,
        color: Color32,
    ) -> bool {
        let mut is_editing = ui.memory(|m| m.data.get_temp::<bool>(edit_id).unwrap_or(false));
        let rect_center = center + vec2(0.0, outer_radius + y_off);
        let text_rect = Rect::from_center_size(rect_center, vec2(80.0, 18.0));
        let font_id = FontId::proportional(11.0);
        let painter = ui.painter();

        let area_res = ui.interact(text_rect, edit_id, Sense::click());
        if area_res.clicked() {
            is_editing = true;
            ui.memory_mut(|m| m.data.insert_temp(edit_id, true));
        }

        if is_editing {
            let mut val_str = ui.memory(|m| {
                m.data
                    .get_temp::<String>(edit_id.with("s"))
                    .unwrap_or_else(|| format!("{:.2}", p.value()))
            });

            let output = ui.put(
                text_rect,
                egui::TextEdit::singleline(&mut val_str)
                    .font(font_id)
                    .frame(true)
                    .margin(vec2(2.0, 0.0))
                    .horizontal_align(egui::Align::Center),
            );

            if output.changed() {
                ui.memory_mut(|m| m.data.insert_temp(edit_id.with("s"), val_str.clone()));
            }

            if output.lost_focus()
                || (output.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                if let Ok(v) = val_str.parse::<f64>() {
                    p.set_value(v);
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
            painter.rect_filled(text_rect, 2.0, Color32::from_black_alpha(150));
            painter.circle_filled(text_rect.left_center() + vec2(6.0, 0.0), 3.0, color);

            painter.text(
                rect_center,
                Align2::CENTER_CENTER,
                format!("{:.1}", p.value()),
                font_id,
                Color32::WHITE,
            );
        }
        is_editing
    }
}

impl<'a> Widget for StackedKnob<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = vec2(100.0, 150.0);
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());

        let center = rect.center() + vec2(0.0, -30.0);
        let inner_radius = 20.0;
        let outer_radius = 32.0;

        let target_id = response.id.with("target");
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
        }

        if response.double_clicked() {
            let pos = response.interact_pointer_pos().unwrap_or(center);
            let target = if pos.distance(center) <= inner_radius + 4.0 {
                self.upper_param
            } else {
                self.lower_param
            };
            target.set_value(target.info.default_plain);
        }

        if response.dragged() && active_target != 0 {
            let p = if active_target == 1 {
                self.upper_param
            } else {
                self.lower_param
            };
            let delta = -response.drag_delta().y * 0.005;
            let val = (p.value_normalized() + delta as f64).clamp(0.0, 1.0);
            p.set_value_normalized(val);
        }

        if response.drag_stopped() {
            ui.memory_mut(|mem| mem.data.insert_temp(target_id, 0u8));
        }

        // --- 描画 ---
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let start_angle = PI * 0.75;
            let end_angle = PI * 2.25;

            painter.circle_filled(center, outer_radius + 6.0, Color32::BLACK);

            // 下段 (Outer)
            painter.circle_filled(center, outer_radius, Color32::from_rgb(20, 20, 20));
            Self::draw_arc(
                painter,
                center,
                outer_radius + 4.0,
                self.lower_param.value_normalized() as f32,
                self.lower_color,
            );

            // 上段 (Inner)
            painter.circle_filled(center, inner_radius, Color32::from_rgb(40, 40, 40));
            Self::draw_arc(
                painter,
                center,
                inner_radius + 3.0,
                self.upper_param.value_normalized() as f32,
                self.upper_color,
            );

            // 指針描画
            let draw_needle =
                |norm_val: f64, r_min: f32, r_max: f32, color: Color32, width: f32| {
                    let ang = start_angle + norm_val as f32 * (end_angle - start_angle);
                    painter.line_segment(
                        [
                            center + vec2(ang.cos(), ang.sin()) * r_min,
                            center + vec2(ang.cos(), ang.sin()) * r_max,
                        ],
                        Stroke::new(width, color),
                    );
                };

            draw_needle(
                self.lower_param.value_normalized(),
                inner_radius + 6.0,
                outer_radius - 2.0,
                self.lower_color,
                3.0,
            );
            draw_needle(
                self.upper_param.value_normalized(),
                2.0,
                inner_radius - 2.0,
                self.upper_color,
                2.5,
            );
        }

        // 数値表示・編集エリア
        let edit_upper = self.draw_value_edit(
            ui,
            center,
            self.upper_param,
            22.0,
            outer_radius,
            response.id.with("ed_u"),
            self.upper_color,
        );
        let edit_lower = self.draw_value_edit(
            ui,
            center,
            self.lower_param,
            44.0,
            outer_radius,
            response.id.with("ed_l"),
            self.lower_color,
        );

        if response.dragged() || edit_upper || edit_lower {
            ui.ctx().request_repaint();
        }

        response
    }
}
