use crate::params::{MetalXrossParams, PeqBandParams};
use nih_plug::params::Param;
use nih_plug::prelude::ParamSetter;
use nih_plug_egui::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke};
use std::f32::consts::PI;
use std::sync::Arc;

// --- 周波数変換ユーティリティ ---
fn norm_to_freq(norm: f32) -> f32 {
    let min_log = 20.0f32.ln();
    let max_log = 20000.0f32.ln();
    (min_log + norm * (max_log - min_log)).exp()
}

fn freq_to_norm(freq: f32) -> f32 {
    let min_log = 20.0f32.ln();
    let max_log = 20000.0f32.ln();
    ((freq.clamp(20.0, 20000.0)).ln() - min_log) / (max_log - min_log)
}

pub enum FilterType {
    LowShelf,
    Peaking,
    HighShelf,
}

fn get_filter_gain(f: f32, band: &PeqBandParams, filter_type: FilterType, sample_rate: f32) -> f32 {
    let gain_db = band.gain.value();
    if gain_db.abs() < 0.01 {
        return 0.0;
    }
    let f0 = band.freq.value();
    let q = band.q.value();
    let a = 10.0f32.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * f0 / sample_rate;
    let cos_w0 = w0.cos();
    let alpha = w0.sin() / (2.0 * q);
    let w = 2.0 * PI * f / sample_rate;
    let cos_w = w.cos();

    let (b0, b1, b2, a0, a1, a2) = match filter_type {
        FilterType::LowShelf => {
            let a_plus = a + 1.0;
            let a_minus = a - 1.0;
            let s = 2.0 * a.sqrt() * alpha;
            (
                a * ((a_plus - a_minus * cos_w0) + s),
                2.0 * a * (a_minus - a_plus * cos_w0),
                a * ((a_plus - a_minus * cos_w0) - s),
                a_plus + a_minus * cos_w0 + s,
                -2.0 * (a_minus + a_plus * cos_w0),
                a_plus + a_minus * cos_w0 - s,
            )
        }
        FilterType::Peaking => (
            1.0 + alpha * a,
            -2.0 * cos_w0,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cos_w0,
            1.0 - alpha / a,
        ),
        FilterType::HighShelf => {
            let a_plus = a + 1.0;
            let a_minus = a - 1.0;
            let s = 2.0 * a.sqrt() * alpha;
            (
                a * ((a_plus + a_minus * cos_w0) + s),
                -2.0 * a * (a_minus + a_plus * cos_w0),
                a * ((a_plus + a_minus * cos_w0) - s),
                a_plus - a_minus * cos_w0 + s,
                2.0 * (a_minus - a_plus * cos_w0),
                a_plus - a_minus * cos_w0 - s,
            )
        }
    };
    let n_re = b0 + b1 * cos_w + b2 * (2.0 * w).cos();
    let n_im = -(b1 * w.sin() + b2 * (2.0 * w).sin());
    let d_re = a0 + a1 * cos_w + a2 * (2.0 * w).cos();
    let d_im = -(a1 * w.sin() + a2 * (2.0 * w).sin());
    10.0 * ((n_re.powi(2) + n_im.powi(2)) / (d_re.powi(2) + d_im.powi(2)).max(1e-10)).log10()
}

pub struct EqualizerBox;

impl EqualizerBox {
    pub fn draw(ui: &mut egui::Ui, params: &Arc<MetalXrossParams>, setter: &ParamSetter) {
        let label_w = 35.0;
        let bottom_h = 20.0;
        let margin_r = 10.0;

        let full_rect = ui.available_rect_before_wrap();
        let graph_rect = Rect::from_min_size(
            full_rect.min + egui::vec2(label_w, 10.0),
            egui::vec2(
                (full_rect.width() - label_w - margin_r).max(100.0),
                (full_rect.height() - bottom_h - 20.0).max(100.0),
            ),
        );

        let painter = ui.painter();

        // 1. 背景のベース描画（0,0,0,128を重ねる）
        painter.rect_filled(
            graph_rect,
            2.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 150),
        );

        // ラベル背面の座布団（視認性向上）
        let left_strip = Rect::from_min_max(
            Pos2::new(full_rect.left(), graph_rect.top()),
            Pos2::new(graph_rect.left(), graph_rect.bottom()),
        );
        let bottom_strip = Rect::from_min_max(
            Pos2::new(graph_rect.left(), graph_rect.bottom()),
            Pos2::new(graph_rect.right(), full_rect.bottom()),
        );
        painter.rect_filled(
            left_strip,
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        );
        painter.rect_filled(
            bottom_strip,
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        );

        let stroke_grid = Stroke::new(1.0, Color32::from_gray(60));
        let font_grid = FontId::proportional(10.0);

        // 2. グリッドとラベル
        for db in [-18, -12, -6, 0, 6, 12, 18] {
            let y_norm = 1.0 - (db as f32 + 20.0) / 40.0;
            let y = graph_rect.top() + y_norm * graph_rect.height();
            painter.line_segment(
                [
                    Pos2::new(graph_rect.left(), y),
                    Pos2::new(graph_rect.right(), y),
                ],
                stroke_grid,
            );
            painter.text(
                Pos2::new(graph_rect.left() - 5.0, y),
                Align2::RIGHT_CENTER,
                db.to_string(),
                font_grid.clone(),
                Color32::WHITE,
            );
        }

        let f_points = [50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0];
        for &f in &f_points {
            let x = graph_rect.left() + freq_to_norm(f) * graph_rect.width();
            painter.line_segment(
                [
                    Pos2::new(x, graph_rect.top()),
                    Pos2::new(x, graph_rect.bottom()),
                ],
                stroke_grid,
            );
            let txt = if f >= 1000.0 {
                format!("{:.0}k", f / 1000.0)
            } else {
                f.to_string()
            };
            painter.text(
                Pos2::new(x, graph_rect.bottom() + 10.0),
                Align2::CENTER_CENTER,
                txt,
                font_grid.clone(),
                Color32::WHITE,
            );
        }

        // 3. EQカーブ描画
        let curve_painter = painter.with_clip_rect(graph_rect);
        let steps = (graph_rect.width() as usize / 2).max(120);
        let points: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let x_norm = i as f32 / steps as f32;
                let f = norm_to_freq(x_norm);
                let g = get_filter_gain(f, &params.eq.low, FilterType::LowShelf, 44100.0)
                    + get_filter_gain(f, &params.eq.mid, FilterType::Peaking, 44100.0)
                    + get_filter_gain(f, &params.eq.high, FilterType::HighShelf, 44100.0);
                let y_norm = 1.0 - (g.clamp(-20.0, 20.0) + 20.0) / 40.0;
                Pos2::new(
                    graph_rect.left() + x_norm * graph_rect.width(),
                    graph_rect.top() + y_norm * graph_rect.height(),
                )
            })
            .collect();
        curve_painter.add(egui::Shape::line(
            points,
            Stroke::new(2.5, Color32::from_rgb(0, 255, 255)),
        ));

        // 4. バンド描画
        Self::draw_band(
            ui,
            graph_rect,
            &params.eq.low,
            setter,
            Color32::from_rgb(255, 165, 0),
            "LOW",
        );
        Self::draw_band(
            ui,
            graph_rect,
            &params.eq.mid,
            setter,
            Color32::from_rgb(0, 255, 127),
            "MID",
        );
        Self::draw_band(
            ui,
            graph_rect,
            &params.eq.high,
            setter,
            Color32::from_rgb(180, 100, 255),
            "HIGH",
        );

        ui.allocate_rect(graph_rect, egui::Sense::hover());
    }

    fn draw_band(
        ui: &mut egui::Ui,
        rect: Rect,
        band: &PeqBandParams,
        setter: &ParamSetter,
        color: Color32,
        label: &str,
    ) {
        let x_norm = freq_to_norm(band.freq.value());
        let y_norm = 1.0 - (band.gain.value() + 20.0) / 40.0;
        let pos = Pos2::new(
            (rect.left() + x_norm * rect.width()).clamp(rect.left(), rect.right()),
            (rect.top() + y_norm * rect.height()).clamp(rect.top(), rect.bottom()),
        );

        let id = ui.make_persistent_id(label);
        let popup_id = id.with("popup");
        let resp = ui.interact(
            Rect::from_center_size(pos, egui::vec2(30.0, 30.0)),
            id,
            egui::Sense::click_and_drag(),
        );

        if resp.double_clicked() {
            for p in [&band.freq, &band.gain, &band.q] {
                setter.begin_set_parameter(p);
                setter.set_parameter(p, p.default_plain_value());
                setter.end_set_parameter(p);
            }
        } else if resp.clicked() {
            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
        }

        if resp.dragged() {
            let delta = resp.drag_delta();
            let new_f = (band.freq.value().ln()
                + (delta.x / rect.width()) * (20000.0f32.ln() - 20.0f32.ln()))
            .exp();
            let new_g = (band.gain.value() - (delta.y / rect.height()) * 40.0).clamp(-20.0, 20.0);
            setter.begin_set_parameter(&band.freq);
            setter.set_parameter(&band.freq, new_f.clamp(20.0, 20000.0));
            setter.end_set_parameter(&band.freq);
            setter.begin_set_parameter(&band.gain);
            setter.set_parameter(&band.gain, new_g);
            setter.end_set_parameter(&band.gain);
        }

        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let new_q = (band.q.value() + scroll / 250.0).clamp(0.1, 10.0);
                setter.begin_set_parameter(&band.q);
                setter.set_parameter(&band.q, new_q);
                setter.end_set_parameter(&band.q);
            }
        }

        if ui.memory(|mem| mem.is_popup_open(popup_id)) {
            egui::Area::new(popup_id)
                .order(egui::Order::Foreground)
                .fixed_pos(pos + egui::vec2(15.0, -40.0))
                .show(ui.ctx(), |ui| {
                    egui::Frame::window(ui.style())
                        .fill(Color32::from_rgba_unmultiplied(20, 20, 25, 250))
                        .stroke(Stroke::new(1.0, color))
                        .show(ui, |ui| {
                            ui.set_width(120.0);
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(label).color(color).strong());
                                let p_data = [
                                    (&band.freq, "F", 20.0..=20000.0, 1.0, "Hz"),
                                    (&band.gain, "G", -20.0..=20.0, 0.1, "dB"),
                                    (&band.q, "Q", 0.1..=10.0, 0.01, ""),
                                ];
                                for (p, l, r, s, suf) in p_data {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}:", l));
                                        let mut v = p.value();
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut v)
                                                    .suffix(suf)
                                                    .range(r)
                                                    .speed(s),
                                            )
                                            .changed()
                                        {
                                            setter.begin_set_parameter(p);
                                            setter.set_parameter(p, v);
                                            setter.end_set_parameter(p);
                                        }
                                    });
                                }
                            });
                        });
                });
        }

        let is_active =
            resp.hovered() || resp.dragged() || ui.memory(|mem| mem.is_popup_open(popup_id));
        let painter = ui.painter().with_clip_rect(rect);

        // Qガイド描画
        let q_val = band.q.value();
        let q_radius = ((rect.width() * 0.1) / q_val.sqrt()).clamp(10.0, rect.width() / 4.0);
        painter.circle_filled(
            pos,
            q_radius,
            color.linear_multiply(if is_active { 0.2 } else { 0.05 }),
        );
        painter.circle_stroke(pos, q_radius, Stroke::new(1.0, color.linear_multiply(0.4)));

        // メイン点
        painter.circle_filled(pos, 8.0, color);
        painter.circle_stroke(
            pos,
            8.0,
            Stroke::new(
                2.0,
                if is_active {
                    Color32::WHITE
                } else {
                    Color32::BLACK
                },
            ),
        );

        if is_active && !ui.memory(|mem| mem.is_popup_open(popup_id)) {
            let label_text = format!(
                "{}\n{:.0}Hz\n{:.1}dB\nQ:{:.2}",
                label,
                band.freq.value(),
                band.gain.value(),
                q_val
            );
            let galley =
                ui.painter()
                    .layout_no_wrap(label_text, FontId::proportional(11.0), Color32::WHITE);
            let text_rect = Rect::from_center_size(
                pos + egui::vec2(0.0, -45.0),
                galley.size() + egui::vec2(8.0, 4.0),
            );
            ui.painter()
                .rect_filled(text_rect, 2.0, Color32::from_black_alpha(200));
            ui.painter()
                .galley(text_rect.min + egui::vec2(4.0, 2.0), galley, Color32::WHITE);
        }
    }
}
