//! BC Drums — app separado, com a mesma interface analógica do plugin.
//!
//! Fluxo: Escolher WAV → (análise da música inteira) → Processar → Ouvir
//! (A/B original x BC Drums) → Baixar. Nada é salvo antes de você aprovar.

mod player;

use bcd_core::{analyze_song, default_packs_dir, pack::read_wav, process_offline, scan_library, Engine, Hit};
use bcd_ui::{HitMark, Layout};
use eframe::egui::{self, RichText, Sense, Vec2};
use player::Player;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

struct Loaded {
    path: PathBuf,
    frames: Arc<Vec<[f32; 2]>>,
    sr: u32,
    hits: Vec<Hit>,
}

enum JobResult {
    Loaded(Result<Loaded, String>),
    Rendered(Result<Vec<[f32; 2]>, String>),
    Saved(Result<PathBuf, String>),
}

#[derive(Clone, Copy, PartialEq)]
struct Settings {
    style: usize,
    transient: f32,
    mix: f32,
}

struct App {
    loaded: Option<Loaded>,
    rendered: Option<(Arc<Vec<[f32; 2]>>, Settings)>,
    styles: Vec<String>,
    s: Settings,
    needle: f32,
    status: String,
    busy: Option<&'static str>,
    jobs: Vec<Receiver<JobResult>>,
    player: Player,
}

impl Default for App {
    fn default() -> Self {
        let mut a = Self {
            loaded: None,
            rendered: None,
            styles: Vec::new(),
            s: Settings { style: 0, transient: 0.65, mix: 1.0 },
            needle: -30.0,
            status: String::new(),
            busy: None,
            jobs: Vec::new(),
            player: Player::new(),
        };
        a.reload();
        if let Some(e) = &a.player.error {
            a.status = format!("Sem saída de áudio para ouvir: {e}");
        }
        if let Some(p) = std::env::args().nth(1).map(PathBuf::from).filter(|p| p.exists()) {
            a.open(p);
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

fn load(path: PathBuf) -> Result<Loaded, String> {
    let (frames, sr) = read_wav(&path).map_err(|e| e.to_string())?;
    let mono: Vec<f32> = frames.iter().map(|f| 0.5 * (f[0] + f[1])).collect();
    let hits = analyze_song(&mono, sr as f32);
    Ok(Loaded { path, frames: Arc::new(frames), sr, hits })
}

fn render(frames: &[[f32; 2]], sr: u32, hits: Vec<Hit>, style: String, s: Settings) -> Result<Vec<[f32; 2]>, String> {
    let lib = Arc::new(scan_library(&default_packs_dir(), sr));
    let mut eng = Engine::new(sr as f32, lib.clone());
    eng.params.style = lib.packs.iter().position(|p| p.name == style).unwrap_or(0);
    eng.params.transient = s.transient;
    eng.params.mix = s.mix;
    eng.set_hits(hits);
    let mut out = process_offline(&mut eng, frames);
    out.truncate(frames.len() + (sr as usize) * 2); // até 2 s de cauda do reverb
    let peak = out.iter().map(|f| f[0].abs().max(f[1].abs())).fold(0.0, f32::max);
    if peak > 0.98 {
        let g = 0.98 / peak;
        for f in out.iter_mut() {
            f[0] *= g;
            f[1] *= g;
        }
    }
    Ok(out)
}

impl App {
    fn reload(&mut self) {
        let _ = std::fs::create_dir_all(default_packs_dir());
        self.styles = pack_names();
        if self.s.style >= self.styles.len() {
            self.s.style = 0;
        }
        self.status = if self.styles.is_empty() {
            format!("Nenhum pack em {}", default_packs_dir().display())
        } else {
            format!("{} estilo(s) carregado(s)", self.styles.len())
        };
    }

    fn spawn(&mut self, label: &'static str, f: impl FnOnce() -> JobResult + Send + 'static) {
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(f());
        });
        self.jobs.push(rx);
        self.busy = Some(label);
    }

    fn open(&mut self, p: PathBuf) {
        self.player.stop();
        self.loaded = None;
        self.rendered = None;
        self.status = "Analisando a música inteira…".into();
        self.spawn("Analisando…", move || JobResult::Loaded(load(p)));
    }

    fn process(&mut self) {
        let (Some(l), Some(style)) = (self.loaded.as_ref(), self.styles.get(self.s.style).cloned()) else { return };
        let (frames, sr, hits, s) = (l.frames.clone(), l.sr, l.hits.clone(), self.s);
        self.player.stop();
        self.status = "Processando…".into();
        self.spawn("Processando…", move || JobResult::Rendered(render(&frames, sr, hits, style, s)));
    }

    fn save(&mut self) {
        let (Some(l), Some((out, s))) = (self.loaded.as_ref(), self.rendered.as_ref()) else { return };
        let style = self.styles.get(s.style).cloned().unwrap_or_default();
        let stem = l.path.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "bateria".into());
        let mut dlg = rfd::FileDialog::new().add_filter("WAV", &["wav"]).set_file_name(format!("{stem} - BC Drums {style}.wav"));
        if let Some(dir) = l.path.parent() {
            dlg = dlg.set_directory(dir);
        }
        if let Some(dest) = dlg.save_file() {
            let (out, sr) = (out.clone(), l.sr);
            self.status = "Salvando…".into();
            self.spawn("Salvando…", move || JobResult::Saved(write_wav(&dest, &out, sr).map(|_| dest)));
        }
    }

    fn poll(&mut self) {
        let mut done = Vec::new();
        let mut results = Vec::new();
        for (i, rx) in self.jobs.iter().enumerate() {
            if let Ok(r) = rx.try_recv() {
                done.push(i);
                results.push(r);
            }
        }
        for i in done.into_iter().rev() {
            self.jobs.remove(i);
        }
        for r in results {
            self.busy = None;
            match r {
                JobResult::Loaded(Ok(l)) => {
                    self.status = format!("{} batidas reconhecidas. Escolha o estilo e clique em Processar.", l.hits.len());
                    self.loaded = Some(l);
                }
                JobResult::Loaded(Err(e)) => self.status = format!("Erro ao abrir: {e}"),
                JobResult::Rendered(Ok(out)) => {
                    let out = Arc::new(out);
                    if let Some(l) = &self.loaded {
                        self.player.load(l.frames.as_ref().clone(), out.as_ref().clone(), l.sr);
                        self.player.set_processed(true);
                        self.player.toggle();
                    }
                    self.rendered = Some((out, self.s));
                    self.status = "Tocando o resultado. Use A/B para comparar; se aprovar, clique em Baixar.".into();
                }
                JobResult::Rendered(Err(e)) => self.status = format!("Erro ao processar: {e}"),
                JobResult::Saved(Ok(p)) => self.status = format!("Salvo: {}", p.display()),
                JobResult::Saved(Err(e)) => self.status = format!("Erro ao salvar: {e}"),
            }
        }
    }

    /// nível do trecho que está tocando, para o VU
    fn play_level(&self) -> f32 {
        if !self.player.is_playing() {
            return 0.0;
        }
        let buf = if self.player.processed_on() {
            self.rendered.as_ref().map(|r| r.0.clone())
        } else {
            self.loaded.as_ref().map(|l| l.frames.clone())
        };
        let (Some(b), Some(l)) = (buf, self.loaded.as_ref()) else { return 0.0 };
        let i = (self.player.position_seconds() * l.sr as f32) as usize;
        let n = (l.sr as usize) / 20;
        b.get(i..(i + n).min(b.len()))
            .map(|w| (w.iter().map(|f| 0.5 * (f[0] * f[0] + f[1] * f[1])).sum::<f32>() / w.len().max(1) as f32).sqrt() * 2.0)
            .unwrap_or(0.0)
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if let Some(p) = dropped.into_iter().find(|p| p.extension().map(|e| e.eq_ignore_ascii_case("wav")).unwrap_or(false)) {
            self.open(p);
        }
        self.poll();
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) && self.rendered.is_some() {
            self.player.toggle();
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(30));

        let stale = self.rendered.as_ref().map(|r| r.1 != self.s).unwrap_or(false);

        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(bcd_ui::PANEL)).show(ctx, |ui| {
            let full = ui.max_rect();
            let l = Layout::new(full);
            let p = ui.painter().clone();
            bcd_ui::chassis(&p, full, &l, "BC STUDIO · APP");

            // tela de transientes: música inteira + cursor de play
            let (marks, dur, name) = match &self.loaded {
                Some(ld) => (
                    ld.hits.iter().map(|h| HitMark { time_s: h.time as f32 / ld.sr as f32, piece: h.piece, velocity: h.velocity }).collect::<Vec<_>>(),
                    ld.frames.len() as f32 / ld.sr as f32,
                    ld.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                ),
                None => (Vec::new(), 4.0, "ARRASTE AQUI A LR DA BATERIA (WAV) OU CLIQUE PARA ESCOLHER".to_string()),
            };
            let style_name = self.styles.get(self.s.style).cloned().unwrap_or_else(|| "—".into());
            let ab = if self.rendered.is_none() { "" } else if self.player.processed_on() { " · OUVINDO: BC DRUMS" } else { " · OUVINDO: ORIGINAL" };
            let playhead = self.rendered.as_ref().map(|_| self.player.position_seconds());
            bcd_ui::transient_view(
                &p,
                l.screen,
                &marks,
                0.0,
                dur.max(0.1),
                playhead,
                &name,
                &format!("{} · TD {:.0}% · MIX {:.0}%{ab}", style_name.to_uppercase(), self.s.transient * 100.0, self.s.mix * 100.0),
            );
            let sr = ui.interact(l.screen, ui.id().with("screen"), Sense::click());
            if sr.clicked() {
                match (&self.loaded, &self.rendered, sr.interact_pointer_pos()) {
                    (Some(_), Some(_), Some(pos)) => {
                        // clicar na tela leva o play para aquele ponto
                        self.player.seek_seconds((pos.x - l.screen.left()) / l.screen.width() * dur);
                    }
                    (None, _, _) => {
                        if let Some(f) = rfd::FileDialog::new().add_filter("WAV", &["wav"]).pick_file() {
                            self.open(f);
                        }
                    }
                    _ => {}
                }
            }
            if sr.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            let lvl = self.play_level();
            bcd_ui::vu_ballistics(&mut self.needle, lvl);
            bcd_ui::vu(&p, l.vu, self.needle, "VU · SAÍDA");

            let names = if self.styles.is_empty() { vec!["Natural".to_string()] } else { self.styles.clone() };
            if let Some(i) = bcd_ui::style_selector(ui, &p, l.style_center, &names, self.s.style) {
                self.s.style = i;
            }
            let k = bcd_ui::knob(ui, &p, l.transient_center, "TRANSIENT DESIGN", &format!("{:.0}%", self.s.transient * 100.0), self.s.transient, None);
            if k.double_clicked {
                self.s.transient = 0.65;
            } else if let Some(v) = k.new_value {
                self.s.transient = v;
            }
            let k = bcd_ui::knob(ui, &p, l.mix_center, "MIX", &format!("{:.0}%", self.s.mix * 100.0), self.s.mix, Some(("DRY", "WET")));
            if k.double_clicked {
                self.s.mix = 1.0;
            } else if let Some(v) = k.new_value {
                self.s.mix = v;
            }

            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(l.actions.expand2(Vec2::new(0.0, 10.0))));
            child.vertical(|ui| {
                let w = Vec2::new(150.0, 30.0);
                let free = self.busy.is_none();
                if ui.add_enabled(free, egui::Button::new("Escolher WAV…").min_size(w)).clicked() {
                    if let Some(f) = rfd::FileDialog::new().add_filter("WAV", &["wav"]).pick_file() {
                        self.open(f);
                    }
                }
                let proc_label = match self.busy {
                    Some(b) => b,
                    None if stale => "Processar de novo",
                    None => "1. Processar",
                };
                let can = free && self.loaded.is_some() && !self.styles.is_empty();
                if ui.add_enabled(can, egui::Button::new(RichText::new(proc_label).color(bcd_ui::ACCENT)).min_size(w)).clicked() {
                    self.process();
                }
                let has = self.rendered.is_some();
                ui.horizontal(|ui| {
                    let play_lbl = if self.player.is_playing() { "Pausar" } else { "2. Ouvir" };
                    if ui.add_enabled(has, egui::Button::new(play_lbl).min_size(Vec2::new(72.0, 30.0))).clicked() {
                        self.player.toggle();
                    }
                    let ab_lbl = if self.player.processed_on() { "A/B: BC" } else { "A/B: Orig" };
                    if ui.add_enabled(has, egui::Button::new(ab_lbl).min_size(Vec2::new(72.0, 30.0))).clicked() {
                        self.player.set_processed(!self.player.processed_on());
                    }
                });
                if ui.add_enabled(has && free, egui::Button::new(RichText::new("3. Baixar WAV").color(bcd_ui::ACCENT)).min_size(w)).clicked() {
                    self.save();
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("Recarregar").min_size(Vec2::new(72.0, 26.0))).clicked() {
                        self.reload();
                    }
                    if ui.add(egui::Button::new("Packs").min_size(Vec2::new(72.0, 26.0))).clicked() {
                        let dir = default_packs_dir();
                        let _ = std::fs::create_dir_all(&dir);
                        #[cfg(target_os = "macos")]
                        let _ = std::process::Command::new("open").arg(&dir).spawn();
                        #[cfg(not(target_os = "macos"))]
                        let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
                    }
                });
                ui.add_space(4.0);
                let msg = if stale && self.busy.is_none() { "Ajustes mudaram: clique em Processar de novo para ouvir." } else { self.status.as_str() };
                ui.label(RichText::new(msg).color(bcd_ui::DIM).size(10.5));
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
