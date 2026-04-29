use crate::params::MetalXrossParams;
use egui::{
    Align2, Area, Color32, DragValue, FontId, Frame, Order, Pos2, Rect, Sense, Shape, Stroke, Ui,
    vec2,
};
use std::f32::consts::PI;

// --- 周波数変換ユーティリティ (変更なし) ---
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

// ヘルパー：truceのParamから値を取得して計算
fn get_filter_gain_truce(
    f: f32,
    freq: f32,
    q: f32,
    gain_db: f32,
    filter_type: FilterType,
    sample_rate: f32,
) -> f32 {
    if gain_db.abs() < 0.01 {
        return 0.0;
    }
    let a = 10.0f32.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * freq / sample_rate;
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
    pub fn draw(ui: &mut Ui, params: &MetalXrossParams) {
        let label_w = 30.0;
        let bottom_h = 15.0;
        let full_rect = ui.available_rect_before_wrap();

        // 背景
        ui.painter()
            .rect_filled(full_rect, 4.0, Color32::from_black_alpha(200));

        let graph_rect = Rect::from_min_max(
            full_rect.min + vec2(label_w, 5.0),
            full_rect.max - vec2(10.0, bottom_h),
        );

        let painter = ui.painter().with_clip_rect(graph_rect);
        let stroke_main = Stroke::new(1.0, Color32::from_gray(80));
        let stroke_sub = Stroke::new(0.5, Color32::from_gray(40));
        let font_grid = FontId::proportional(9.0);

        // dBグリッド描画 (省略せず同様に実装)
        for db in [-18, -12, -6, 0, 6, 12, 18] {
            let y = graph_rect.top() + (1.0 - (db as f32 + 20.0) / 40.0) * graph_rect.height();
            painter.line_segment(
                [
                    Pos2::new(graph_rect.left(), y),
                    Pos2::new(graph_rect.right(), y),
                ],
                stroke_sub,
            );
            ui.painter().text(
                Pos2::new(graph_rect.left() - 4.0, y),
                Align2::RIGHT_CENTER,
                db.to_string(),
                font_grid.clone(),
                Color32::GRAY,
            );
        }

        // 周波数グリッド描画 (省略せず同様に実装)
        let main_freqs = [
            20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0,
        ];
        for &f in &main_freqs {
            let x = graph_rect.left() + freq_to_norm(f) * graph_rect.width();
            painter.line_segment(
                [
                    Pos2::new(x, graph_rect.top()),
                    Pos2::new(x, graph_rect.bottom()),
                ],
                stroke_main,
            );
            let txt = if f >= 1000.0 {
                format!("{:.0}k", f / 1000.0)
            } else {
                f.to_string()
            };
            ui.painter().text(
                Pos2::new(x, graph_rect.bottom() + 8.0),
                Align2::CENTER_CENTER,
                txt,
                font_grid.clone(),
                Color32::GRAY,
            );
        }

        // --- 合成カーブ ---
        let steps = (graph_rect.width() as usize / 2).max(120);
        let points: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let x_norm = i as f32 / steps as f32;
                let f = norm_to_freq(x_norm);
                let g = get_filter_gain_truce(
                    f,
                    params.eq_lo_freq.value(),
                    params.eq_lo_q.value(),
                    params.eq_lo_gain.value(),
                    FilterType::LowShelf,
                    44100.0,
                ) + get_filter_gain_truce(
                    f,
                    params.eq_mi_freq.value(),
                    params.eq_mi_q.value(),
                    params.eq_mi_gain.value(),
                    FilterType::Peaking,
                    44100.0,
                ) + get_filter_gain_truce(
                    f,
                    params.eq_hi_freq.value(),
                    params.eq_hi_q.value(),
                    params.eq_hi_gain.value(),
                    FilterType::HighShelf,
                    44100.0,
                );
                let y_norm = 1.0 - (g.clamp(-20.0, 20.0) + 20.0) / 40.0;
                Pos2::new(
                    graph_rect.left() + x_norm * graph_rect.width(),
                    graph_rect.top() + y_norm * graph_rect.height(),
                )
            })
            .collect();

        painter.add(Shape::line(
            points,
            Stroke::new(2.0, Color32::from_rgb(0, 255, 255)),
        ));

        // --- 各バンドの描画 ---
        // P::識別子を使わず、MetalXrossParamsのフィールドを直接渡す
        Self::draw_band(
            ui,
            graph_rect,
            &params.eq_lo_freq,
            &params.eq_lo_gain,
            &params.eq_lo_q,
            Color32::from_rgb(255, 165, 0),
            "LOW",
        );
        Self::draw_band(
            ui,
            graph_rect,
            &params.eq_mi_freq,
            &params.eq_mi_gain,
            &params.eq_mi_q,
            Color32::from_rgb(0, 255, 127),
            "MID",
        );
        Self::draw_band(
            ui,
            graph_rect,
            &params.eq_hi_freq,
            &params.eq_hi_gain,
            &params.eq_hi_q,
            Color32::from_rgb(180, 100, 255),
            "HIGH",
        );
    }

    fn draw_band(
        ui: &mut Ui,
        rect: Rect,
        p_freq: &truce::params::FloatParam,
        p_gain: &truce::params::FloatParam,
        p_q: &truce::params::FloatParam,
        color: Color32,
        label: &str,
    ) {
        let pos = Pos2::new(
            rect.left() + freq_to_norm(p_freq.value()) * rect.width(),
            rect.top() + (1.0 - (p_gain.value() + 20.0) / 40.0) * rect.height(),
        );

        let id = ui.make_persistent_id(label);
        let popup_id = id.with("popup");
        let resp = ui.interact(
            Rect::from_center_size(pos, vec2(16.0, 16.0)),
            id,
            Sense::click_and_drag(),
        );

        // ダブルクリックでリセット
        if resp.double_clicked() {
            p_freq.set_value(p_freq.info.default_plain);
            p_gain.set_value(p_gain.info.default_plain);
            p_q.set_value(p_q.info.default_plain);
        } else if resp.clicked() {
            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
        }

        // ドラッグ操作
        if resp.dragged() {
            let delta = resp.drag_delta();
            let new_f = (p_freq.value().ln()
                + (delta.x / rect.width()) * (20000.0f32.ln() - 20.0f32.ln()))
            .exp();
            let new_g = (p_gain.value() - (delta.y / rect.height()) * 40.0).clamp(-20.0, 20.0);
            let new_f = new_f.clamp(20.0, 20000.0) as f64;
            let new_g = new_g as f64;
            p_freq.set_value(new_f);
            p_gain.set_value(new_g);
        }

        // スクロールでQ値を変更
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let new_q = (p_q.value() + scroll / 500.0).clamp(0.1, 10.0);
                let new_q = new_q as f64;
                p_q.set_value(new_q);
            }
        }

        // 数値入力ポップアップ
        if ui.memory(|mem| mem.is_popup_open(popup_id)) {
            Area::new(popup_id)
                .order(Order::Foreground)
                .fixed_pos(pos + vec2(10.0, 10.0))
                .show(ui.ctx(), |ui| {
                    Frame::window(ui.style())
                        .fill(Color32::from_black_alpha(240))
                        .stroke(Stroke::new(1.0, color))
                        .show(ui, |ui| {
                            ui.set_width(100.0);
                            let mut f = p_freq.value() as f64;
                            if ui
                                .add(DragValue::new(&mut f).suffix("Hz").range(20.0..=20000.0))
                                .changed()
                            {
                                p_freq.set_value(f);
                            }
                            let mut g = p_gain.value() as f64;
                            if ui
                                .add(DragValue::new(&mut g).suffix("dB").range(-20.0..=20.0))
                                .changed()
                            {
                                p_gain.set_value(g);
                            }
                            let mut q = p_q.value() as f64;
                            if ui
                                .add(DragValue::new(&mut q).prefix("Q ").range(0.1..=10.0))
                                .changed()
                            {
                                p_q.set_value(q);
                            }
                        });
                });
        }

        let painter = ui.painter().with_clip_rect(rect);
        let is_active =
            resp.hovered() || resp.dragged() || ui.memory(|mem| mem.is_popup_open(popup_id));

        // 描画
        let q_radius = ((rect.width() * 0.08) / p_q.value().sqrt()).clamp(8.0, rect.width() / 4.0);
        painter.circle_stroke(pos, q_radius, Stroke::new(0.5, color.linear_multiply(0.3)));
        painter.circle_filled(pos, 5.0, if is_active { Color32::WHITE } else { color });

        if is_active && !ui.memory(|mem| mem.is_popup_open(popup_id)) {
            let label_text = format!(
                "{}\n{:.0}Hz\n{:.1}dB",
                label,
                p_freq.value(),
                p_gain.value()
            );
            let galley =
                ui.painter()
                    .layout_no_wrap(label_text, FontId::proportional(10.0), Color32::WHITE);
            let text_rect =
                Rect::from_center_size(pos + vec2(0.0, -35.0), galley.size() + vec2(8.0, 4.0));
            ui.painter()
                .rect_filled(text_rect, 2.0, Color32::from_black_alpha(180));
            ui.painter()
                .galley(text_rect.min + vec2(4.0, 2.0), galley, Color32::WHITE);
        }
    }
}
