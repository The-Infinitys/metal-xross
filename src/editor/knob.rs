use nih_plug::params::FloatParam;
use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui;

pub struct CustomKnob<'a> {
    param: &'a FloatParam,
    setter: &'a ParamSetter<'a>,
}

impl<'a> CustomKnob<'a> {
    pub fn new(param: &'a FloatParam, setter: &'a ParamSetter<'a>) -> Self {
        Self { param, setter }
    }
}

impl<'a> egui::Widget for CustomKnob<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = egui::vec2(80.0, 80.0);
        let (rect, response) = ui.allocate_at_least(desired_size, egui::Sense::drag());

        if response.dragged() {
            let delta = response.drag_delta().x - response.drag_delta().y;
            let current_value = self.param.unmodulated_normalized_value();
            let new_value = (current_value + delta * 0.005).clamp(0.0, 1.0);
            self.setter.set_parameter_normalized(self.param, new_value);
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let center = rect.center();
            let radius = rect.width().min(rect.height()) * 0.45;

            let value = self.param.unmodulated_normalized_value();

            // ノブの本体
            painter.circle_filled(center, radius, egui::Color32::from_rgb(51, 51, 51));
            painter.circle_stroke(center, radius, egui::Stroke::new(2.0, egui::Color32::from_rgb(102, 102, 102)));

            // インジケーター
            let angle = (value * 270.0 - 135.0).to_radians();
            let indicator_len = radius * 0.8;
            let target = center + egui::vec2(angle.sin(), -angle.cos()) * indicator_len;

            painter.line_segment(
                [center, target],
                egui::Stroke::new(4.0, egui::Color32::from_rgb(255, 0, 0)),
            );
        }

        response
    }
}
