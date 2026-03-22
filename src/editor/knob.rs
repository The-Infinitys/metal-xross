use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;

pub struct CustomKnob {
    param_base: ParamWidgetBase,
}

impl CustomKnob {
    pub fn new<L, Params, P, FMap>(cx: &mut Context, params: L, params_to_param: FMap) -> Handle<'_, Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
    {
        Self {
            param_base: ParamWidgetBase::new(cx, params.clone(), params_to_param),
        }
        .build(
            cx,
            ParamWidgetBase::build_view(params, params_to_param, move |_cx, _param_data| {}),
        )
    }
}

impl View for CustomKnob {
    fn element(&self) -> Option<&'static str> {
        Some("custom-knob")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.param_base.begin_set_parameter(cx);
                cx.capture();
                cx.set_active(true);
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                self.param_base.end_set_parameter(cx);
                cx.release();
                cx.set_active(false);
                meta.consume();
            }
            WindowEvent::MouseMove(x, y) => {
                if cx.is_active() {
                    let scale = 0.005;
                    let delta_x = *x - cx.mouse().previous_cursorx;
                    let delta_y = cx.mouse().previous_cursory - *y;
                    let delta = delta_x + delta_y;
                    let new_value = (self.param_base.unmodulated_normalized_value() + delta * scale)
                        .clamp(0.0, 1.0);
                    self.param_base.set_normalized_value(cx, new_value);
                }
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let center_x = bounds.x + bounds.width() / 2.0;
        let center_y = bounds.y + bounds.height() / 2.0;
        let radius = bounds.width().min(bounds.height()) * 0.45;

        let value = self.param_base.unmodulated_normalized_value();

        let mut circle_path = vg::Path::new();
        circle_path.circle(center_x, center_y, radius);
        let mut circle_paint = vg::Paint::color(vg::Color::hex("333333"));
        canvas.fill_path(&circle_path, &circle_paint);

        circle_paint.set_color(vg::Color::hex("666666"));
        circle_paint.set_line_width(2.0);
        canvas.stroke_path(&circle_path, &circle_paint);

        let angle = (value * 270.0 - 135.0).to_radians();
        let indicator_len = radius * 0.8;
        let target_x = center_x + angle.sin() * indicator_len;
        let target_y = center_y - angle.cos() * indicator_len;

        let mut indicator_path = vg::Path::new();
        indicator_path.move_to(center_x, center_y);
        indicator_path.line_to(target_x, target_y);

        let mut indicator_paint = vg::Paint::color(vg::Color::hex("ff0000"));
        indicator_paint.set_line_width(4.0);
        indicator_paint.set_line_cap(vg::LineCap::Round);
        canvas.stroke_path(&indicator_path, &indicator_paint);
    }
}
