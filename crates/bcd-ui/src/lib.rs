//! Interface analógica do BC Drums, desenhada com egui e usada tanto pelo
//! plugin quanto pelo app separado (mesmo visual nos dois).
//!
//! Layout (1040 × 660): cabeçalho; tela de transientes + VU de ponteiro;
//! seletor rotativo de Estilo, knobs Transient Design e Mix; coluna de ações.

use bcd_core::Piece;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2};

pub const WIDTH: f32 = 1040.0;
pub const HEIGHT: f32 = 660.0;

pub const PANEL: Color32 = Color32::from_rgb(0x2A, 0x28, 0x25);
pub const FACE: Color32 = Color32::from_rgb(0x1F, 0x1E, 0x1C);
pub const CREAM: Color32 = Color32::from_rgb(0xE9, 0xE1, 0xCC);
pub const DIM: Color32 = Color32::from_rgb(0xA3, 0x9B, 0x88);
pub const ACCENT: Color32 = Color32::from_rgb(0xE8, 0xA2, 0x3A);
pub const SCREEN: Color32 = Color32::from_rgb(0x0B, 0x11, 0x0E);
pub const PHOSPHOR: Color32 = Color32::from_rgb(0x7F, 0xD9, 0xA3);
const VU_FACE: Color32 = Color32::from_rgb(0xEF, 0xE4, 0xC6);
const VU_INK: Color32 = Color32::from_rgb(0x2A, 0x26, 0x20);
const VU_RED: Color32 = Color32::from_rgb(0xB3, 0x36, 0x2B);
const BLACK: Color32 = Color32::from_rgb(0x12, 0x11, 0x10);
const KNOB_RIM: Color32 = Color32::from_rgb(0x23, 0x22, 0x20);
const KNOB_RIDGE: Color32 = Color32::from_rgb(0x3D, 0x3B, 0x37);
const KNOB_CAP: Color32 = Color32::from_rgb(0x2E, 0x2C, 0x29);

/// Posições fixas dos elementos (as mesmas no plugin e no app).
pub struct Layout {
    pub header: Rect,
    pub screen: Rect,
    pub vu: Rect,
    pub style_center: Pos2,
    pub transient_center: Pos2,
    pub mix_center: Pos2,
    pub actions: Rect,
}

impl Layout {
    pub fn new(full: Rect) -> Self {
        let o = full.min.to_vec2();
        let at = |x: f32, y: f32| Pos2::new(x, y) + o;
        Self {
            header: Rect::from_min_size(full.min, Vec2::new(full.width(), 60.0)),
            screen: Rect::from_min_size(at(36.0, 98.0), Vec2::new(640.0, 196.0)),
            vu: Rect::from_min_size(at(704.0, 98.0), Vec2::new(300.0, 196.0)),
            style_center: at(170.0, 478.0),
            transient_center: at(445.0, 478.0),
            mix_center: at(690.0, 478.0),
            actions: Rect::from_min_size(at(852.0, 352.0), Vec2::new(160.0, 290.0)),
        }
    }
}

/// Ponto em coordenadas polares: 0° = para cima, sentido horário.
pub fn polar(c: Pos2, r: f32, deg: f32) -> Pos2 {
    let a = (deg - 90.0).to_radians();
    Pos2::new(c.x + r * a.cos(), c.y + r * a.sin())
}

fn text(p: &egui::Painter, pos: Pos2, s: &str, size: f32, color: Color32, align: Align2) {
    p.text(pos, align, s, FontId::proportional(size), color);
}

fn screw(p: &egui::Painter, c: Pos2) {
    p.circle_filled(c, 6.0, Color32::from_rgb(0x3A, 0x38, 0x35));
    p.circle_stroke(c, 6.0, Stroke::new(1.0_f32, BLACK));
    p.line_segment([c - Vec2::new(3.5, 0.0), c + Vec2::new(3.5, 0.0)], Stroke::new(1.5_f32, BLACK));
}

/// Fundo, parafusos e cabeçalho.
pub fn chassis(p: &egui::Painter, full: Rect, l: &Layout, right_note: &str) {
    p.rect_filled(full, 0.0, PANEL);
    p.rect_filled(l.header, 0.0, FACE);
    p.line_segment([l.header.left_bottom(), l.header.right_bottom()], Stroke::new(2.0_f32, BLACK));
    text(p, l.header.left_center() + Vec2::new(40.0, 0.0), "BC DRUMS", 30.0, CREAM, Align2::LEFT_CENTER);
    text(p, l.header.left_center() + Vec2::new(216.0, 3.0), "AI DRUM PROCESSOR", 12.0, ACCENT, Align2::LEFT_CENTER);
    let lamp = l.header.right_center() - Vec2::new(250.0, 0.0);
    p.circle_filled(lamp, 9.0, ACCENT.gamma_multiply(0.25));
    p.circle_filled(lamp, 5.0, ACCENT);
    text(p, l.header.right_center() - Vec2::new(40.0, 0.0), right_note, 12.0, DIM, Align2::RIGHT_CENTER);
    for c in [
        full.left_top() + Vec2::new(14.0, 14.0),
        full.right_top() + Vec2::new(-14.0, 14.0),
        full.left_bottom() + Vec2::new(14.0, -14.0),
        full.right_bottom() + Vec2::new(-14.0, -14.0),
    ] {
        screw(p, c);
    }
}

/// Uma batida para a tela de transientes.
#[derive(Clone, Copy, Debug)]
pub struct HitMark {
    pub time_s: f32,
    pub piece: Piece,
    pub velocity: f32,
}

/// Tela de transientes (fósforo). Mostra as batidas entre `t0` e `t1`
/// segundos; `playhead` desenha o cursor de play.
#[allow(clippy::too_many_arguments)]
pub fn transient_view(
    p: &egui::Painter,
    r: Rect,
    hits: &[HitMark],
    t0: f32,
    t1: f32,
    playhead: Option<f32>,
    top_left: &str,
    bottom_right: &str,
) {
    text(p, r.left_top() - Vec2::new(0.0, 10.0), "TRANSIENT VIEW", 13.0, CREAM, Align2::LEFT_BOTTOM);
    text(p, r.right_top() - Vec2::new(0.0, 10.0), "K bumbo · S caixa · T tons", 11.0, DIM, Align2::RIGHT_BOTTOM);
    p.rect_filled(r.expand(3.0), 10.0, BLACK);
    p.rect_filled(r, 8.0, SCREEN);
    let span = (t1 - t0).max(0.001);
    let x_of = |t: f32| r.left() + (t - t0) / span * r.width();
    // grade
    let step = if span > 20.0 { 5.0 } else if span > 6.0 { 1.0 } else { 0.5 };
    let mut g = (t0 / step).ceil() * step;
    while g < t1 {
        let x = x_of(g);
        let mut y = r.top() + 30.0;
        while y < r.bottom() - 12.0 {
            p.line_segment([Pos2::new(x, y), Pos2::new(x, (y + 3.0).min(r.bottom() - 12.0))], Stroke::new(1.0_f32, Color32::from_rgb(0x2E, 0x4A, 0x3A)));
            y += 7.0;
        }
        g += step;
    }
    let mid = r.center().y + 12.0;
    let bar_gap = (6.0f32).min(r.width() / span * 0.012).max(2.5);
    let mut last_label = f32::MIN;
    for h in hits {
        let x = x_of(h.time_s);
        if x < r.left() + 4.0 || x > r.right() - 4.0 {
            continue;
        }
        let height = 18.0 + 112.0 * h.velocity.clamp(0.0, 1.0);
        let col = if matches!(h.piece, Piece::Snare) { ACCENT } else { PHOSPHOR };
        for k in 0..5 {
            let xk = x + k as f32 * bar_gap * 1.4;
            if xk > r.right() - 3.0 {
                break;
            }
            let hk = height * 0.6f32.powi(k);
            p.rect_filled(Rect::from_center_size(Pos2::new(xk, mid), Vec2::new(bar_gap.min(3.5), hk)), 1.5, col.gamma_multiply(1.0 - 0.15 * k as f32));
        }
        let tri_y = r.top() + 26.0;
        if x - last_label < 12.0 {
            continue; // muito perto da marca anterior: só a barra
        }
        last_label = x;
        p.add(Shape::convex_polygon(
            vec![Pos2::new(x - 4.5, tri_y - 5.0), Pos2::new(x + 4.5, tri_y - 5.0), Pos2::new(x, tri_y + 2.0)],
            ACCENT,
            Stroke::NONE,
        ));
        text(p, Pos2::new(x, tri_y - 13.0), h.piece.short(), 10.0, ACCENT, Align2::CENTER_CENTER);
    }
    if let Some(ph) = playhead {
        let x = x_of(ph);
        p.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], Stroke::new(2.0_f32, CREAM.gamma_multiply(0.85)));
    }
    text(p, r.left_bottom() + Vec2::new(12.0, -8.0), top_left, 11.0, PHOSPHOR, Align2::LEFT_BOTTOM);
    text(p, r.right_bottom() - Vec2::new(12.0, 8.0), bottom_right, 11.0, PHOSPHOR, Align2::RIGHT_BOTTOM);
}

const VU_MARKS: [(f32, f32); 10] =
    [(-20.0, -48.0), (-10.0, -34.0), (-7.0, -24.0), (-5.0, -15.0), (-3.0, -5.0), (-1.0, 6.0), (0.0, 14.0), (1.0, 24.0), (2.0, 34.0), (3.0, 46.0)];

fn vu_angle(vu_db: f32) -> f32 {
    if vu_db <= VU_MARKS[0].0 {
        return VU_MARKS[0].1 - 4.0;
    }
    for w in VU_MARKS.windows(2) {
        if vu_db <= w[1].0 {
            let t = (vu_db - w[0].0) / (w[1].0 - w[0].0);
            return w[0].1 + t * (w[1].1 - w[0].1);
        }
    }
    50.0
}

/// Balística de VU (~300 ms). `peak_lin`: nível de pico linear; 0 VU ≈ -14 dBFS.
pub fn vu_ballistics(needle_vu_db: &mut f32, peak_lin: f32) {
    let target = 20.0 * peak_lin.max(1e-6).log10() + 14.0;
    *needle_vu_db += (target - *needle_vu_db) * 0.18;
}

/// VU de ponteiro.
pub fn vu(p: &egui::Painter, r: Rect, needle_vu_db: f32, title: &str) {
    text(p, r.left_top() - Vec2::new(0.0, 10.0), title, 13.0, CREAM, Align2::LEFT_BOTTOM);
    p.rect_filled(r.expand(3.0), 10.0, BLACK);
    let p = &p.with_clip_rect(r);
    p.rect_filled(r, 8.0, VU_FACE);
    let pivot = Pos2::new(r.center().x, r.bottom() + 16.0);
    // arco da escala
    let arc: Vec<Pos2> = (0..=60).map(|i| polar(pivot, 132.0, -48.0 + 62.0 * i as f32 / 60.0)).collect();
    p.add(Shape::line(arc, Stroke::new(1.5_f32, VU_INK)));
    let red: Vec<Pos2> = (0..=30).map(|i| polar(pivot, 133.0, 14.0 + 32.0 * i as f32 / 30.0)).collect();
    p.add(Shape::line(red, Stroke::new(5.0_f32, VU_RED)));
    for (v, d) in VU_MARKS {
        let col = if v > 0.0 { VU_RED } else { VU_INK };
        p.line_segment([polar(pivot, 132.0, d), polar(pivot, 144.0, d)], Stroke::new(2.0_f32, col));
        let s = if v > 0.0 { format!("+{v}") } else { format!("{v}") };
        text(p, polar(pivot, 158.0, d), &s, 12.0, col, Align2::CENTER_CENTER);
    }
    text(p, Pos2::new(r.center().x, r.top() + 112.0), "VU", 26.0, VU_INK, Align2::CENTER_CENTER);
    text(p, Pos2::new(r.center().x, r.top() + 136.0), "BC DRUMS · OUTPUT", 9.0, Color32::from_rgb(0x5A, 0x52, 0x46), Align2::CENTER_CENTER);
    let deg = vu_angle(needle_vu_db);
    p.line_segment([pivot, polar(pivot, 150.0, deg)], Stroke::new(2.0_f32, Color32::from_rgb(0x1A, 0x17, 0x14)));
    let base = Rect::from_min_max(Pos2::new(r.left(), r.bottom() - 24.0), r.right_bottom());
    p.rect_filled(base, egui::CornerRadius { nw: 0, ne: 0, sw: 8, se: 8 }, PANEL);
}

/// O que aconteceu com um knob neste quadro.
#[derive(Default, Debug, Clone, Copy)]
pub struct KnobInput {
    pub drag_started: bool,
    pub drag_stopped: bool,
    pub double_clicked: bool,
    /// arrastando neste quadro
    pub dragging: bool,
    /// novo valor normalizado 0..1, se mudou
    pub new_value: Option<f32>,
}

fn knurled_knob(p: &egui::Painter, c: Pos2, r: f32, angle: f32) {
    // saia com marcador
    p.circle_filled(c, r + 14.0, Color32::from_rgb(0x14, 0x13, 0x12));
    p.circle_stroke(c, r + 14.0, Stroke::new(1.5_f32, Color32::from_rgb(0x05, 0x05, 0x05)));
    p.line_segment([polar(c, r + 5.0, angle), polar(c, r + 13.0, angle)], Stroke::new(4.0_f32, ACCENT));
    // corpo serrilhado
    p.circle_filled(c, r, KNOB_RIM);
    for i in 0..48 {
        let d = i as f32 * 7.5;
        p.line_segment([polar(c, r - 6.0, d), polar(c, r - 0.5, d)], Stroke::new(2.0_f32, KNOB_RIDGE));
    }
    p.circle_stroke(c, r, Stroke::new(2.0_f32, Color32::from_rgb(0x0E, 0x0D, 0x0C)));
    // tampa
    p.circle_filled(c, r - 11.0, KNOB_CAP);
    p.circle_stroke(c, r - 11.0, Stroke::new(1.0_f32, Color32::from_rgb(0x3A, 0x37, 0x33)));
    // ponteiro
    p.line_segment([polar(c, 8.0, angle), polar(c, r - 4.0, angle)], Stroke::new(4.5_f32, CREAM));
}

/// Knob analógico grande com escala impressa. `value` 0..1.
/// `ends` troca os números 0–10 por rótulos nas pontas (ex.: DRY/WET).
pub fn knob(
    ui: &mut egui::Ui,
    p: &egui::Painter,
    c: Pos2,
    title: &str,
    readout: &str,
    value: f32,
    ends: Option<(&str, &str)>,
) -> KnobInput {
    let r = 50.0;
    let rect = Rect::from_center_size(c, Vec2::splat(2.0 * (r + 18.0)));
    let id = ui.id().with(("knob", title));
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let mut input = KnobInput { drag_started: resp.drag_started(), drag_stopped: resp.drag_stopped(), double_clicked: resp.double_clicked(), dragging: resp.dragged(), new_value: None };
    if resp.dragged() {
        let d = -resp.drag_delta().y / 220.0 + resp.drag_delta().x / 440.0;
        if d != 0.0 {
            input.new_value = Some((value + d).clamp(0.0, 1.0));
        }
    }
    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            input.new_value = Some((value + scroll / 600.0).clamp(0.0, 1.0));
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    let v = input.new_value.unwrap_or(value);

    text(p, c - Vec2::new(0.0, r + 58.0), title, 16.0, CREAM, Align2::CENTER_CENTER);
    for i in 0..=10 {
        let d = -135.0 + 27.0 * i as f32;
        let long = i % 5 == 0;
        p.line_segment(
            [polar(c, r + 20.0, d), polar(c, r + if long { 30.0 } else { 26.0 }, d)],
            Stroke::new(if long { 2.5_f32 } else { 1.5_f32 }, CREAM),
        );
        if ends.is_none() {
            text(p, polar(c, r + 41.0, d), &i.to_string(), 12.0, DIM, Align2::CENTER_CENTER);
        }
    }
    if let Some((a, b)) = ends {
        text(p, polar(c, r + 44.0, -135.0), a, 12.0, DIM, Align2::CENTER_CENTER);
        text(p, polar(c, r + 44.0, 135.0), b, 12.0, DIM, Align2::CENTER_CENTER);
    }
    knurled_knob(p, c, r, -135.0 + 270.0 * v);
    text(p, c + Vec2::new(0.0, r + 52.0), readout, 13.0, CREAM, Align2::CENTER_CENTER);
    input
}

/// Seletor rotativo de Estilo ("cabeça de galinha") com os nomes ao redor.
/// Devolve o índice clicado (no nome ou no knob, que avança para o próximo).
pub fn style_selector(ui: &mut egui::Ui, p: &egui::Painter, c: Pos2, names: &[String], current: usize) -> Option<usize> {
    let n = names.len().max(1);
    let cur = current.min(n - 1);
    text(p, c - Vec2::new(0.0, 108.0), "ESTILO", 16.0, CREAM, Align2::CENTER_CENTER);
    let span = if n <= 1 { 0.0 } else { 250.0f32.min(36.0 * (n as f32 - 1.0)) };
    let angle = |i: usize| if n <= 1 { 0.0 } else { -span / 2.0 + span * i as f32 / (n as f32 - 1.0) };
    let mut picked = None;
    for (i, name) in names.iter().enumerate().take(n) {
        let d = angle(i);
        let on = i == cur;
        p.line_segment([polar(c, 54.0, d), polar(c, 64.0, d)], Stroke::new(2.5_f32, if on { ACCENT } else { CREAM }));
        let pos = polar(c, 86.0, d);
        let hit = Rect::from_center_size(pos, Vec2::new(84.0, 20.0));
        let resp = ui.interact(hit, ui.id().with(("style", i)), Sense::click());
        let hover = resp.hovered();
        text(p, pos, &name.to_uppercase(), if on { 13.0 } else { 12.0 }, if on { ACCENT } else if hover { CREAM } else { DIM }, Align2::CENTER_CENTER);
        if resp.clicked() {
            picked = Some(i);
        }
    }
    if n <= 1 {
        text(p, c + Vec2::new(0.0, 74.0), "1 estilo instalado", 11.0, DIM, Align2::CENTER_CENTER);
    }
    let knob_rect = Rect::from_center_size(c, Vec2::splat(96.0));
    let resp = ui.interact(knob_rect, ui.id().with("style_knob"), Sense::click());
    if resp.clicked() && n > 1 {
        picked = Some((cur + 1) % n);
    }
    // knob cabeça de galinha
    let d = angle(cur).to_radians();
    p.circle_filled(c, 44.0, Color32::from_rgb(0x14, 0x13, 0x12));
    p.circle_stroke(c, 44.0, Stroke::new(1.5_f32, Color32::from_rgb(0x05, 0x05, 0x05)));
    let rot = |x: f32, y: f32| Pos2::new(c.x + x * d.cos() - y * d.sin(), c.y + x * d.sin() + y * d.cos());
    let body: Vec<Pos2> = [(-15.0, 36.0), (-11.0, -50.0), (-5.0, -58.0), (5.0, -58.0), (11.0, -50.0), (15.0, 36.0), (8.0, 44.0), (-8.0, 44.0)]
        .iter()
        .map(|&(x, y)| rot(x, y))
        .collect();
    p.add(Shape::convex_polygon(body, Color32::from_rgb(0x28, 0x26, 0x23), Stroke::new(2.0_f32, Color32::from_rgb(0x0E, 0x0D, 0x0C))));
    p.line_segment([rot(0.0, -52.0), rot(0.0, -24.0)], Stroke::new(4.5_f32, CREAM));
    p.circle_filled(c, 6.0, Color32::from_rgb(0x3A, 0x38, 0x35));
    picked
}

/// Estilo dos botões da coluna de ações (cor de equipamento).
pub fn style_buttons(ctx: &egui::Context) {
    ctx.style_mut(|s| {
        let w = &mut s.visuals.widgets;
        w.inactive.weak_bg_fill = Color32::from_rgb(0x1E, 0x1D, 0x1B);
        w.inactive.bg_fill = Color32::from_rgb(0x1E, 0x1D, 0x1B);
        w.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x45, 0x42, 0x3D));
        w.inactive.fg_stroke = Stroke::new(1.0_f32, CREAM);
        w.hovered.weak_bg_fill = Color32::from_rgb(0x33, 0x30, 0x2C);
        w.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
        w.hovered.fg_stroke = Stroke::new(1.0_f32, CREAM);
        w.active.weak_bg_fill = Color32::from_rgb(0x3A, 0x2C, 0x18);
        w.active.fg_stroke = Stroke::new(1.0_f32, ACCENT);
        s.visuals.override_text_color = None;
    });
}
