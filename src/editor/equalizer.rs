use crate::params::{MetalXrossParams, PeqBandParams};
use nih_plug::prelude::Param;
use nih_plug::prelude::ParamSetter;
use nih_plug_egui::egui::{self, Color32, Pos2, Rect, Stroke};
use std::sync::Arc;

// --- 周波数変換ユーティリティ ---
// 画面上のx座標 (0.0 ~ 1.0) を周波数 (20Hz ~ 20000Hz) に対数的に変換
fn norm_to_freq(norm: f32) -> f32 {
    let min_log = 20.0f32.ln();
    let max_log = 20000.0f32.ln();
    (min_log + norm * (max_log - min_log)).exp()
}

// 周波数 (20Hz ~ 20000Hz) を画面上のx座標 (0.0 ~ 1.0) に対数的に変換
fn freq_to_norm(freq: f32) -> f32 {
    let min_log = 20.0f32.ln();
    let max_log = 20000.0f32.ln();
    ((freq.max(20.0).min(20000.0)).ln() - min_log) / (max_log - min_log)
}

// 簡易的なピーキングEQのゲイン計算（Q値も反映）
fn get_band_gain(freq: f32, band: &PeqBandParams) -> f32 {
    let center_freq = band.freq.value();
    let gain_db = band.gain.value();
    let q = band.q.value();

    if gain_db == 0.0 {
        return 0.0;
    }

    let v_gain = 10.0f32.powf(gain_db / 20.0);
    let w0 = freq / center_freq;

    // ピーキングフィルターのレスポンス計算
    let den = 1.0 + (q * (w0 - 1.0 / w0)).powi(2);
    let num = (1.0 + v_gain * (den - 1.0)) / den;

    10.0 * num.log10()
}

pub struct EqualizerBox;

impl EqualizerBox {
    pub fn draw(ui: &mut egui::Ui, params: &Arc<MetalXrossParams>, setter: &ParamSetter) {
        ui.add_space(10.0);

        // --- レイアウト計算 (ラベルエリアの確保) ---
        let available_size = ui.available_size();
        let label_w = 45.0; // 左側のdBラベル用
        let label_h = 20.0; // 下側の周波数ラベル用
        let margin = 5.0;

        let graph_rect = Rect::from_min_size(
            ui.cursor().min + egui::vec2(label_w + margin, margin),
            egui::vec2(
                (available_size.x - label_w - margin * 2.0).max(100.0),
                (available_size.y - label_h - margin * 2.0).max(100.0),
            ),
        );

        // エリア確保（警告回避のため _rect）
        let (_rect, _response) = ui.allocate_exact_size(available_size, egui::Sense::hover());
        let painter = ui.painter();

        // 1. 背景と枠線
        painter.rect_filled(graph_rect, 2.0, Color32::from_rgb(5, 5, 5));
        painter.rect_stroke(
            graph_rect,
            2.0,
            Stroke::new(1.0, Color32::from_rgb(40, 40, 40)),
            egui::StrokeKind::Middle,
        );

        // 2. グリッドとラベルの描画
        let grid_stroke = Stroke::new(1.0, Color32::from_rgb(25, 25, 25));
        let text_color = Color32::from_rgb(120, 120, 120);
        let font_id = egui::FontId::proportional(11.0);

        // --- 周波数軸 (垂直グリッド) ---
        let freqs = [
            20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0,
        ];
        for &f in &freqs {
            let x_norm = freq_to_norm(f);
            let x = graph_rect.left() + x_norm * graph_rect.width();

            painter.line_segment(
                [
                    Pos2::new(x, graph_rect.top()),
                    Pos2::new(x, graph_rect.bottom()),
                ],
                grid_stroke,
            );

            let label = if f >= 1000.0 {
                format!("{:.0}k", f / 1000.0)
            } else {
                format!("{:.0}", f)
            };
            painter.text(
                Pos2::new(x, graph_rect.bottom() + 5.0),
                egui::Align2::CENTER_TOP,
                label,
                font_id.clone(),
                text_color,
            );
        }

        // --- ゲイン軸 (水平グリッド: +-20dB) ---
        let gains = [-20, -10, 0, 10, 20];
        for &g in &gains {
            let y_norm = 1.0 - (g as f32 + 20.0) / 40.0;
            let y = graph_rect.top() + y_norm * graph_rect.height();

            painter.line_segment(
                [
                    Pos2::new(graph_rect.left(), y),
                    Pos2::new(graph_rect.right(), y),
                ],
                grid_stroke,
            );

            let label = if g == 0 {
                "0".to_string()
            } else {
                format!("{:+}dB", g)
            };
            painter.text(
                Pos2::new(graph_rect.left() - 5.0, y),
                egui::Align2::RIGHT_CENTER,
                label,
                font_id.clone(),
                text_color,
            );
        }

        // 3. 総合EQカーブの描画
        {
            let curve_stroke =
                Stroke::new(2.0, Color32::from_rgb(0, 255, 255).linear_multiply(0.8));
            let steps = 150;
            let points: Vec<Pos2> = (0..=steps)
                .map(|i| {
                    let x_norm = i as f32 / steps as f32;
                    let freq = norm_to_freq(x_norm);

                    let gain_total = get_band_gain(freq, &params.eq.low)
                        + get_band_gain(freq, &params.eq.mid)
                        + get_band_gain(freq, &params.eq.high);

                    let y_norm = 1.0 - (gain_total.clamp(-20.0, 20.0) + 20.0) / 40.0;
                    let y = graph_rect.top() + y_norm * graph_rect.height();
                    Pos2::new(graph_rect.left() + x_norm * graph_rect.width(), y)
                })
                .collect();

            painter.add(egui::Shape::line(points, curve_stroke));
        }

        // 4. 各バンドの操作（点）
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
            Color32::from_rgb(138, 43, 226),
            "HIGH",
        );
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
            rect.left() + x_norm * rect.width(),
            rect.top() + y_norm * rect.height(),
        );

        let id = ui.make_persistent_id(label);
        let dot_response = ui.interact(
            Rect::from_center_size(pos, egui::vec2(30.0, 30.0)),
            id,
            egui::Sense::click_and_drag(),
        );

        if dot_response.dragged() {
            let delta = dot_response.drag_delta();

            let min_log = 20.0f32.ln();
            let max_log = 20000.0f32.ln();
            let freq_log = band.freq.value().max(20.0).min(20000.0).ln();
            let new_freq_log =
                (freq_log + delta.x / rect.width() * (max_log - min_log)).clamp(min_log, max_log);
            let new_freq = new_freq_log.exp();

            let new_gain = (band.gain.value() - delta.y / rect.height() * 40.0).clamp(-20.0, 20.0);

            setter.begin_set_parameter(&band.freq);
            setter.set_parameter(&band.freq, new_freq);
            setter.end_set_parameter(&band.freq);

            setter.begin_set_parameter(&band.gain);
            setter.set_parameter(&band.gain, new_gain);
            setter.end_set_parameter(&band.gain);
        }

        if dot_response.hovered() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                let new_q = (band.q.value() + scroll_delta / 50.0).clamp(0.1, 10.0);
                setter.begin_set_parameter(&band.q);
                setter.set_parameter(&band.q, new_q);
                setter.end_set_parameter(&band.q);
            }
        }

        if dot_response.double_clicked() {
            setter.begin_set_parameter(&band.freq);
            setter.set_parameter(&band.freq, band.freq.default_plain_value());
            setter.end_set_parameter(&band.freq);

            setter.begin_set_parameter(&band.gain);
            setter.set_parameter(&band.gain, band.gain.default_plain_value());
            setter.end_set_parameter(&band.gain);

            setter.begin_set_parameter(&band.q);
            setter.set_parameter(&band.q, band.q.default_plain_value());
            setter.end_set_parameter(&band.q);
        }

        let painter = ui.painter();

        // 1. Q値の視覚化 (広さに応じた円)
        let q_val = band.q.value();
        let q_visual_radius = (rect.width() / 15.0) / q_val.sqrt();
        let q_alpha = if dot_response.hovered() || dot_response.dragged() {
            0.2
        } else {
            0.1
        };

        painter.circle_filled(
            pos,
            q_visual_radius.clamp(10.0, rect.width() / 3.0),
            color.linear_multiply(q_alpha),
        );
        painter.circle_stroke(
            pos,
            q_visual_radius.clamp(10.0, rect.width() / 3.0),
            Stroke::new(1.0, color.linear_multiply(q_alpha * 2.0)),
        );

        // 2. 操作ドット
        let stroke_color = if dot_response.hovered() || dot_response.dragged() {
            Color32::WHITE
        } else {
            color
        };

        painter.circle_filled(pos, 8.0, color.linear_multiply(0.6));
        painter.circle_stroke(pos, 6.0, Stroke::new(2.0, stroke_color));

        // 3. ラベル
        let font_color = if dot_response.hovered() || dot_response.dragged() {
            Color32::WHITE
        } else {
            Color32::from_rgb(200, 200, 200)
        };

        let f_val = band.freq.value();
        let f_str = if f_val >= 1000.0 {
            format!("{:.1}kHz", f_val / 1000.0)
        } else {
            format!("{:.0}Hz", f_val)
        };

        painter.text(
            pos + egui::vec2(0.0, -18.0),
            egui::Align2::CENTER_BOTTOM,
            format!("{}: {}, {:+.1}dB", label, f_str, band.gain.value()),
            egui::FontId::proportional(12.0),
            font_color,
        );
    }
}
