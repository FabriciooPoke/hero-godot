//! BC Drums — app separado, com a mesma interface analógica do plugin.
//! Arraste a LR da bateria (WAV) na tela de transientes (ou clique nela),
//! escolha o Estilo, ajuste Transient Design e Mix e exporte. O arquivo pronto
//! é salvo ao lado do original.

use bcd_core::{default_packs_dir, pack::read_wav, process_offline, scan_library, Detector, Engine};
use bcd_ui::{HitMark, Layout};
use eframe::egui::{self, RichText, Sense, Vec2};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

struct Analysis {
    hits: Vec<HitMark>,
    duration: f32,
}

enum JobResult {
    Analyzed(Analysis),
    Processed(Result<(PathBuf, f32), String>),
}

struct App {
    input: Option<PathBuf>,
    analysis: Option<Analysis>,
    styles: Vec<String>,
    style: usize,
    transient: f32,
    mix: f32,
    needle: f32,
    out_peak: f32,
    status: String,
    busy: bool,
    jobs: Vec<Receiver<JobResult>>,
}

impl Default for App {
    fn default() -> Self {
        let mut a = Self {
            input: None,
            analysis: None,
            styles: Vec::new(),
            style: 0,
            transient: 0.65,
            mix: 1.0,
            needle: -30.0,
            out_peak: 0.0,
            status: String::new(),
            busy: false,
            jobs: Vec::new(),
        };
        a.reload();
        // "Abrir com" / linha de comando: bcdrums-app arquivo.wav
        if let Some(p) = std::env::args().nth(1).map(PathBuf::from).filter(|p| p.exists()) {
            a.set_input(p);
        }
        a
    }
}

fn pack_names() -> Vec<String> {
    let dir = default_packs_dir();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| {
                    let toml = std::fs::read_to_string(e.path().join("pack.toml")).unwrap_or_default();
                    toml.lines()
                        .find_map(|l| {
                            let l = l.trim();
                            l.strip_prefix("name").and_then(|r| r.trim().strip_prefix('=')).map(|v| v.trim().trim_matches('"').to_string())
                        })
                        .unwrap_or_else(|| e.file_name().to_string_lossy().to_string())
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

fn write_wav(path: &Path, data: &[[f32; 2]], sr: u32) -> Result<(), String> {
    let spec = hound::WavSpec { channels: 2, sample_rate: sr, bits_per_sample: 24, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for f in data {
        for c in f {
            w.write_sample((c.clamp(-1.0, 1.0) * 8_388_607.0) as i32).map_err(|e| e.to_string())?;
        }
    }
    w.finalize().map_err(|e| e.to_string())
}

fn analyze(input: &Path) -> Result<Analysis, String> {
    let (frames, sr) = read_wav(input).map_err(|e| e.to_string())?;
    let mut d = Detector::new(sr as f32);
    let mut hits = Vec::new();
    for f in &frames {
        if let Some(h) = d.process(0.5 * (f[0] + f[1])) {
            hits.push(HitMark { time_s: h.time as f32 / sr as f32, piece: h.piece, velocity: h.velocity });
        }
    }
    Ok(Analysis { hits, duration: frames.len() as f32 / sr as f32 })
}

fn process_file(input: PathBuf, style: String, transient: f32, mix: f32) -> Result<(PathBuf, f32), String> {
    let (frames, sr) = read_wav(&input).map_err(|e| e.to_string())?;
    let lib = Arc::new(scan_library(&default_packs_dir(), sr));
    let mut eng = Engine::new(sr as f32, lib.clone());
    eng.params.style = lib.packs.iter().position(|p| p.name == style).unwrap_or(0);
    eng.params.transient = transient;
    eng.params.mix = mix;
    let out = process_offline(&mut eng, &frames);
    let peak = out.iter().map(|f| f[0].abs().max(f[1].abs())).fold(0.0, f32::max);
    let g = if peak > 0.98 { 0.98 / peak } else { 1.0 };
    let out: Vec<[f32; 2]> = out.iter().map(|f| [f[0] * g, f[1] * g]).collect();
    let rms = (out.iter().map(|f| 0.5 * (f[0] * f[0] + f[1] * f[1])).sum::<f32>() / out.len().max(1) as f32).sqrt();
    let stem = input.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "bateria".into());
    let dest = input.with_file_name(format!("{stem} - BC Drums {style}.wav"));
    write_wav(&dest, &out, sr)?;
    Ok((dest, rms * 4.0))
}

impl App {
    fn reload(&mut self) {
        let _ = std::fs::create_dir_all(default_packs_dir());
        self.styles = pack_names();
        if self.style >= self.styles.len() {
            self.style = 0;
        }
        self.status = if self.styles.is_empty() {
            format!("Nenhum pack em {}", default_packs_dir().display())
        } else {
            format!("{} estilo(s) carregado(s)", self.styles.len())
        };
    }

    fn set_input(&mut self, p: PathBuf) {
        self.analysis = None;
        self.input = Some(p.clone());
        self.status = "Analisando as batidas…".into();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            if let Ok(a) = analyze(&p) {
                let _ = tx.send(JobResult::Analyzed(a));
            }
        });
        self.jobs.push(rx);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if let Some(p) = dropped.into_iter().find(|p| p.extension().map(|e| e.eq_ignore_ascii_case("wav")).unwrap_or(false)) {
            self.set_input(p);
        }
        let mut done = Vec::new();
        for (i, rx) in self.jobs.iter().enumerate() {
            if let Ok(r) = rx.try_recv() {
                done.push(i);
                match r {
                    JobResult::Analyzed(a) => {
                        self.status = format!("{} batidas detectadas", a.hits.len());
                        self.analysis = Some(a);
                    }
                    JobResult::Processed(r) => {
                        self.busy = false;
                        match r {
                            Ok((p, lvl)) => {
                                self.out_peak = lvl;
                                self.status = format!("Pronto: {}", p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
                            }
                            Err(e) => self.status = format!("Erro: {e}"),
                        }
                    }
                }
            }
        }
        for i in done.into_iter().rev() {
            self.jobs.remove(i);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(33));

        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(bcd_ui::PANEL)).show(ctx, |ui| {
            let full = ui.max_rect();
            let l = Layout::new(full);
            let p = ui.painter().clone();
            bcd_ui::chassis(&p, full, &l, "BC STUDIO · APP");

            // tela de transientes: análise do arquivo inteiro
            let name = self.input.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string());
            let (hits, dur) = match &self.analysis {
                Some(a) => (a.hits.as_slice(), a.duration.max(0.1)),
                None => (&[][..], 4.0),
            };
            let top = match &name {
                Some(n) => n.clone(),
                None => "ARRASTE AQUI A LR DA BATERIA (WAV) OU CLIQUE PARA ESCOLHER".into(),
            };
            let style_name = self.styles.get(self.style).cloned().unwrap_or_else(|| "—".into());
            bcd_ui::transient_view(
                &p,
                l.screen,
                hits,
                0.0,
                dur,
                None,
                &top,
                &format!("{} · TD {:.0}% · MIX {:.0}%", style_name.to_uppercase(), self.transient * 100.0, self.mix * 100.0),
            );
            let screen_resp = ui.interact(l.screen, ui.id().with("screen"), Sense::click());
            if screen_resp.clicked() {
                if let Some(f) = rfd::FileDialog::new().add_filter("WAV", &["wav"]).pick_file() {
                    self.set_input(f);
                }
            }
            if screen_resp.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            bcd_ui::vu_ballistics(&mut self.needle, self.out_peak);
            bcd_ui::vu(&p, l.vu, self.needle, "VU · SAÍDA");

            let names = if self.styles.is_empty() { vec!["Natural".to_string()] } else { self.styles.clone() };
            if let Some(i) = bcd_ui::style_selector(ui, &p, l.style_center, &names, self.style) {
                self.style = i;
            }
            let k = bcd_ui::knob(ui, &p, l.transient_center, "TRANSIENT DESIGN", &format!("{:.0}%", self.transient * 100.0), self.transient, None);
            if k.double_clicked {
                self.transient = 0.65;
            } else if let Some(v) = k.new_value {
                self.transient = v;
            }
            let k = bcd_ui::knob(ui, &p, l.mix_center, "MIX", &format!("{:.0}%", self.mix * 100.0), self.mix, Some(("DRY", "WET")));
            if k.double_clicked {
                self.mix = 1.0;
            } else if let Some(v) = k.new_value {
                self.mix = v;
            }

            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(l.actions));
            child.vertical(|ui| {
                ui.label(RichText::new("ARQUIVO").color(bcd_ui::CREAM).size(13.0));
                ui.add_space(6.0);
                if ui.add(egui::Button::new("Escolher WAV…").min_size(Vec2::new(150.0, 34.0))).clicked() {
                    if let Some(f) = rfd::FileDialog::new().add_filter("WAV", &["wav"]).pick_file() {
                        self.set_input(f);
                    }
                }
                let can = self.input.is_some() && !self.styles.is_empty() && !self.busy;
                let label = if self.busy { "Processando…" } else { "Processar e exportar" };
                if ui.add_enabled(can, egui::Button::new(RichText::new(label).color(bcd_ui::ACCENT)).min_size(Vec2::new(150.0, 40.0))).clicked() {
                    if let (Some(input), Some(style)) = (self.input.clone(), self.styles.get(self.style).cloned()) {
                        let (tx, rx) = channel();
                        let (t, m) = (self.transient, self.mix);
                        std::thread::spawn(move || {
                            let _ = tx.send(JobResult::Processed(process_file(input, style, t, m)));
                        });
                        self.jobs.push(rx);
                        self.busy = true;
                        self.status = "Processando…".into();
                    }
                }
                ui.add_space(12.0);
                ui.label(RichText::new("PACKS").color(bcd_ui::CREAM).size(13.0));
                ui.add_space(6.0);
                if ui.add(egui::Button::new("Recarregar").min_size(Vec2::new(150.0, 34.0))).clicked() {
                    self.reload();
                }
                if ui.add(egui::Button::new("Abrir pasta").min_size(Vec2::new(150.0, 34.0))).clicked() {
                    let dir = default_packs_dir();
                    let _ = std::fs::create_dir_all(&dir);
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("open").arg(&dir).spawn();
                    #[cfg(not(target_os = "macos"))]
                    let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
                }
                ui.add_space(8.0);
                ui.label(RichText::new(&self.status).color(bcd_ui::DIM).size(10.5));
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([bcd_ui::WIDTH, bcd_ui::HEIGHT])
            .with_resizable(false)
            .with_title("BC Drums"),
        ..Default::default()
    };
    eframe::run_native(
        "BC Drums",
        opts,
        Box::new(|cc| {
            bcd_ui::style_buttons(&cc.egui_ctx);
            Ok(Box::new(App::default()))
        }),
    )
}
