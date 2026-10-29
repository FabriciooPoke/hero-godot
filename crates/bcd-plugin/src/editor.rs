//! Interface do plugin: usa o desenho compartilhado de `bcd-ui`
//! (o mesmo do app separado).

use crate::{BcDrums, BcDrumsParams, Shared, Task};
use bcd_core::default_packs_dir;
use bcd_ui::{HitMark, KnobInput, Layout};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, RichText, Vec2};
use nih_plug_egui::{create_egui_editor, widgets};
use std::sync::atomic::Ordering;
use std::sync::Arc;

struct UiState {
    needle: f32,
}

pub fn create(params: Arc<BcDrumsParams>, shared: Arc<Shared>, executor: AsyncExecutor<BcDrums>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        UiState { needle: -30.0 },
        |ctx, _| bcd_ui::style_buttons(ctx),
        move |ctx, setter, st| {
            ctx.request_repaint();
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(bcd_ui::PANEL)).show(ctx, |ui| {
                draw(ui, &params, &shared, setter, st, &executor);
            });
        },
    )
}

fn apply_knob(input: KnobInput, param: &FloatParam, setter: &ParamSetter) {
    if input.double_clicked {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, param.default_plain_value());
        setter.end_set_parameter(param);
        return;
    }
    if input.drag_started {
        setter.begin_set_parameter(param);
    }
    if let Some(v) = input.new_value {
        if input.dragging {
            setter.set_parameter_normalized(param, v);
        } else {
            // roda do mouse
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, v);
            setter.end_set_parameter(param);
        }
    }
    if input.drag_stopped {
        setter.end_set_parameter(param);
    }
}

fn draw(ui: &mut egui::Ui, params: &BcDrumsParams, shared: &Shared, setter: &ParamSetter, st: &mut UiState, executor: &AsyncExecutor<BcDrums>) {
    let full = ui.max_rect();
    let l = Layout::new(full);
    let p = ui.painter().clone();
    bcd_ui::chassis(&p, full, &l, "BC STUDIO · LR STEREO");

    let sr = shared.sample_rate.load(Ordering::Relaxed).max(1) as f32;
    let meters = shared.meters.read().ok().and_then(|m| m.clone());
    let (now, marks, out) = match meters.as_ref() {
        Some(m) => {
            let now = m.position.load(Ordering::Relaxed) as f32 / sr;
            let marks: Vec<HitMark> =
                m.recent_hits().into_iter().map(|(t, piece, velocity)| HitMark { time_s: t as f32 / sr, piece, velocity }).collect();
            (now, marks, m.out_level())
        }
        None => (0.0, Vec::new(), 0.0),
    };
    let style_name = shared.style_name(params.style.value());
    bcd_ui::transient_view(
        &p,
        l.screen,
        &marks,
        now - 3.0,
        now + 1.0,
        Some(now),
        &format!("PLAY  {:02}:{:06.3}", (now / 60.0) as u32, now % 60.0),
        &format!("{} · TD {:.0}% · MIX {:.0}%", style_name.to_uppercase(), params.transient.value(), params.mix.value()),
    );
    bcd_ui::vu_ballistics(&mut st.needle, out);
    bcd_ui::vu(&p, l.vu, st.needle, "VU · SAÍDA");

    // estilo
    let names = {
        let n = shared.style_names.read().map(|n| n.clone()).unwrap_or_default();
        if n.is_empty() { vec!["Natural".to_string()] } else { n }
    };
    if let Some(i) = bcd_ui::style_selector(ui, &p, l.style_center, &names, params.style.value().max(0) as usize) {
        setter.begin_set_parameter(&params.style);
        setter.set_parameter(&params.style, i as i32);
        setter.end_set_parameter(&params.style);
    }

    // knobs
    let t = &params.transient;
    let input = bcd_ui::knob(ui, &p, l.transient_center, "TRANSIENT DESIGN", &format!("{:.0}%", t.value()), t.unmodulated_normalized_value(), None);
    apply_knob(input, t, setter);
    let m = &params.mix;
    let input = bcd_ui::knob(ui, &p, l.mix_center, "MIX", &format!("{:.0}%", m.value()), m.unmodulated_normalized_value(), Some(("DRY", "WET")));
    apply_knob(input, m, setter);

    // ações
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(l.actions));
    child.vertical(|ui| {
        ui.label(RichText::new("PACKS").color(bcd_ui::CREAM).size(13.0));
        ui.add_space(6.0);
        let reloading = shared.reloading.load(Ordering::Relaxed);
        let txt = if reloading { "Carregando…" } else { "Recarregar" };
        if ui.add_enabled(!reloading, egui::Button::new(txt).min_size(Vec2::new(150.0, 34.0))).clicked() {
            shared.reloading.store(true, Ordering::Relaxed);
            executor.execute_background(Task::ReloadPacks);
        }
        if ui.add(egui::Button::new("Abrir pasta").min_size(Vec2::new(150.0, 34.0))).clicked() {
            open_packs_folder();
        }
        ui.add_space(10.0);
        ui.label(RichText::new("SAÍDA").color(bcd_ui::DIM).size(11.0));
        ui.add(widgets::ParamSlider::for_param(&params.output, setter).with_width(150.0));
        ui.add_space(8.0);
        let status = shared.status.read().map(|s| s.clone()).unwrap_or_default();
        ui.label(RichText::new(status).color(bcd_ui::DIM).size(10.0));
    });
}

fn open_packs_folder() {
    let dir = default_packs_dir();
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let cmd = "xdg-open";
    let _ = std::process::Command::new(cmd).arg(dir).spawn();
}
