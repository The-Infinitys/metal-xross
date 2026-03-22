use nih_plug::prelude::{Editor, Param};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaTheming};
use std::sync::Arc;

use crate::params::MetalXrossParams;

#[derive(Lens)]
pub struct Data {
    pub params: Arc<MetalXrossParams>,
}

impl Model for Data {}

// 1. 背景パーツ: リサイズされるたびに真っ黒を描画
pub struct PcbBackground;

impl View for PcbBackground {
    fn element(&self) -> Option<&'static str> {
        Some("pcb-background")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let mut path = vg::Path::new();
        path.rect(bounds.x, bounds.y, bounds.width(), bounds.height());
        canvas.fill_path(&path, &vg::Paint::color(vg::Color::black()));
    }
}

// 2. 上段パーツで使うカスタムノブ: knob.svgのデザインを再現
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
            ParamWidgetBase::build_view(params, params_to_param, move |_cx, _param_data| {
                // ここでは内部にViewを持たず、CustomKnob自体のdrawで描画する
                // ラベルや値表示が必要ならここに追加できる
            }),
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

        // 1. 円 (fill="#333", stroke="#666")
        let mut circle_path = vg::Path::new();
        circle_path.circle(center_x, center_y, radius);
        let mut circle_paint = vg::Paint::color(vg::Color::hex("333333"));
        canvas.fill_path(&circle_path, &circle_paint);

        circle_paint.set_color(vg::Color::hex("666666"));
        circle_paint.set_line_width(2.0);
        canvas.stroke_path(&circle_path, &circle_paint);

        // 2. 針 (stroke="#f00")
        // -135度から135度（270度）の範囲で回転
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

// 3. 下段パーツ: ヴィジュアルイコライザーを描画するボックス
pub struct EqualizerBox;

impl View for EqualizerBox {
    fn element(&self) -> Option<&'static str> {
        Some("equalizer-box")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let mut path = vg::Path::new();
        path.rect(bounds.x, bounds.y, bounds.width(), bounds.height());

        // 暗い背景
        let mut paint = vg::Paint::color(vg::Color::hex("111111"));
        canvas.fill_path(&path, &paint);

        // 枠線
        paint.set_color(vg::Color::hex("444444"));
        paint.set_line_width(1.0);
        canvas.stroke_path(&path, &paint);

        // とりあえずグリッド的なものを描画
        paint.set_color(vg::Color::hex("222222"));
        for i in 1..4 {
            let x = bounds.x + (bounds.width() / 4.0) * i as f32;
            let mut grid_path = vg::Path::new();
            grid_path.move_to(x, bounds.y);
            grid_path.line_to(x, bounds.y + bounds.height());
            canvas.stroke_path(&grid_path, &paint);
        }
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
                // 背景パーツ
                PcbBackground.build(cx, |_| {});

                // UIレイアウト
                VStack::new(cx, |cx| {
                    // 上段: ノブ3つ
                    HStack::new(cx, |cx| {
                        VStack::new(cx, |cx| {
                            Label::new(cx, "GAIN");
                            CustomKnob::new(cx, Data::params, |p| &p.gain)
                                .width(Pixels(80.0))
                                .height(Pixels(80.0));
                        })
                        .child_space(Stretch(1.0));

                        VStack::new(cx, |cx| {
                            Label::new(cx, "STYLE");
                            CustomKnob::new(cx, Data::params, |p| &p.style)
                                .width(Pixels(80.0))
                                .height(Pixels(80.0));
                        })
                        .child_space(Stretch(1.0));

                        VStack::new(cx, |cx| {
                            Label::new(cx, "LEVEL");
                            CustomKnob::new(cx, Data::params, |p| &p.level)
                                .width(Pixels(80.0))
                                .height(Pixels(80.0));
                        })
                        .child_space(Stretch(1.0));
                    })
                    .height(Pixels(150.0))
                    .width(Stretch(1.0))
                    .child_top(Stretch(1.0))
                    .child_bottom(Stretch(1.0));

                    // 下段: イコライザーボックス
                    VStack::new(cx, |cx| {
                        Label::new(cx, "EQUALIZER");
                        EqualizerBox.build(cx, |_| {})
                            .width(Stretch(1.0))
                            .height(Stretch(1.0));
                    })
                    .width(Stretch(1.0))
                    .height(Stretch(1.0))
                    .child_space(Pixels(10.0));
                })
                .row_between(Pixels(20.0))
                .child_space(Pixels(20.0));
            });
        },
    )
}
