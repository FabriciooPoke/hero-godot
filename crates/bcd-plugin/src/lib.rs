// O macro de exportação VST3 do nih-plug gera um aviso só no macOS.
#![allow(unknown_lints, semicolon_in_expressions_from_non_local_macros, semicolon_in_expressions_from_macros)]

//! BC Drums — plugin.
//!
//! LR da bateria crua entra, LR pronta sai. Controles: Estilo (um por pack
//! na pasta de packs), Transient Design e Mix. "Recarregar" relê a pasta de
//! packs sem fechar o plugin.

use bcd_core::{default_packs_dir, scan_library, Engine, Library, Meters, Prepared};
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};

mod editor;

/// Número máximo de estilos selecionáveis (packs na pasta).
pub const MAX_STYLES: i32 = 32;

/// Estado compartilhado entre áudio, interface e tarefas em segundo plano.
pub struct Shared {
    /// nomes dos estilos carregados (para o seletor e para o host)
    pub style_names: RwLock<Vec<String>>,
    /// mensagens da última leitura da pasta (avisos/erros)
    pub status: RwLock<String>,
    /// biblioteca pronta esperando a thread de áudio instalar
    pub incoming: Mutex<Option<Prepared>>,
    /// biblioteca antiga devolvida pela thread de áudio (descartada fora dela)
    pub trash: Mutex<Option<Prepared>>,
    pub meters: RwLock<Option<Arc<Meters>>>,
    pub sample_rate: AtomicU32,
    pub reloading: AtomicBool,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            style_names: RwLock::new(Vec::new()),
            status: RwLock::new(String::new()),
            incoming: Mutex::new(None),
            trash: Mutex::new(None),
            meters: RwLock::new(None),
            sample_rate: AtomicU32::new(48_000),
            reloading: AtomicBool::new(false),
        }
    }
}

impl Shared {
    pub fn style_name(&self, i: i32) -> String {
        self.style_names
            .read()
            .ok()
            .and_then(|n| n.get(i.max(0) as usize).cloned())
            .unwrap_or_else(|| if i == 0 { "Natural".into() } else { format!("Estilo {}", i + 1) })
    }

    fn publish(&self, lib: &Library) {
        if let Ok(mut n) = self.style_names.write() {
            *n = lib.names();
        }
        let mut msg = if lib.packs.is_empty() {
            format!("Nenhum pack em {}", lib.dir.display())
        } else {
            format!("{} estilo(s) carregado(s)", lib.packs.len())
        };
        for e in &lib.errors {
            msg.push('\n');
            msg.push_str(e);
        }
        for p in &lib.packs {
            for w in &p.warnings {
                msg.push_str(&format!("\n{}: {w}", p.name));
            }
        }
        if let Ok(mut s) = self.status.write() {
            *s = msg;
        }
    }
}

#[derive(Params)]
pub struct BcDrumsParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "style"]
    pub style: IntParam,
    #[id = "transient"]
    pub transient: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "output"]
    pub output: FloatParam,
}

impl BcDrumsParams {
    fn new(shared: Arc<Shared>) -> Self {
        let names = shared.clone();
        Self {
            editor_state: EguiState::from_size(1040, 660),
            style: IntParam::new("Estilo", 0, IntRange::Linear { min: 0, max: MAX_STYLES - 1 })
                .with_value_to_string(Arc::new(move |v| names.style_name(v))),
            transient: FloatParam::new("Transient Design", 65.0, FloatRange::Linear { min: 0.0, max: 100.0 })
                .with_smoother(SmoothingStyle::Linear(30.0))
                .with_unit(" %")
                .with_step_size(0.5),
            mix: FloatParam::new("Mix", 100.0, FloatRange::Linear { min: 0.0, max: 100.0 })
                .with_smoother(SmoothingStyle::Linear(30.0))
                .with_unit(" %")
                .with_step_size(0.5),
            output: FloatParam::new("Saída", 0.0, FloatRange::Linear { min: -18.0, max: 12.0 })
                .with_smoother(SmoothingStyle::Linear(30.0))
                .with_unit(" dB")
                .with_step_size(0.1),
        }
    }
}

pub enum Task {
    /// relê a pasta de packs na taxa de amostragem atual
    ReloadPacks,
}

pub struct BcDrums {
    params: Arc<BcDrumsParams>,
    shared: Arc<Shared>,
    engine: Option<Engine>,
}

impl Default for BcDrums {
    fn default() -> Self {
        let shared = Arc::new(Shared::default());
        Self { params: Arc::new(BcDrumsParams::new(shared.clone())), shared, engine: None }
    }
}

fn load_prepared(shared: &Shared) -> Prepared {
    let sr = shared.sample_rate.load(Ordering::Relaxed);
    let lib = Arc::new(scan_library(&default_packs_dir(), sr));
    shared.publish(&lib);
    Prepared::new(lib, sr as f32)
}

impl Plugin for BcDrums {
    const NAME: &'static str = "BC Drums";
    const VENDOR: &'static str = "BC Studio";
    const URL: &'static str = "https://www.instagram.com/bcstudio.fabricio";
    const EMAIL: &'static str = "fabricio.bcstudio@gmail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = Task;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        let shared = self.shared.clone();
        Box::new(move |task| match task {
            Task::ReloadPacks => {
                if let Ok(mut t) = shared.trash.lock() {
                    t.take(); // descarta a biblioteca anterior aqui, fora do áudio
                }
                let prepared = load_prepared(&shared);
                if let Ok(mut slot) = shared.incoming.lock() {
                    *slot = Some(prepared);
                }
                shared.reloading.store(false, Ordering::Relaxed);
            }
        })
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.shared.clone(), async_executor)
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        let sr = buffer_config.sample_rate;
        self.shared.sample_rate.store(sr as u32, Ordering::Relaxed);
        let prepared = load_prepared(&self.shared);
        let lib = prepared.library().clone();
        let mut engine = Engine::new(sr, lib);
        engine.install(prepared);
        context.set_latency_samples(engine.latency() as u32);
        if let Ok(mut m) = self.shared.meters.write() {
            *m = Some(engine.meters());
        }
        self.engine = Some(engine);
        true
    }

    fn reset(&mut self) {
        if let Some(e) = self.engine.as_mut() {
            e.reset();
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let Some(engine) = self.engine.as_mut() else { return ProcessStatus::Normal };

        // biblioteca nova pronta? instala sem bloquear
        if let Ok(mut slot) = self.shared.incoming.try_lock() {
            if let Some(p) = slot.take() {
                let old = engine.install(p);
                if let Ok(mut t) = self.shared.trash.try_lock() {
                    *t = Some(old);
                } else {
                    // não deve acontecer; evita descartar no áudio na maior parte dos casos
                    *slot = Some(old);
                }
            }
        }

        engine.params.style = self.params.style.value().max(0) as usize;
        engine.params.transient = self.params.transient.smoothed.next_step(buffer.samples() as u32) / 100.0;
        engine.params.mix = self.params.mix.smoothed.next_step(buffer.samples() as u32) / 100.0;
        engine.params.output_db = self.params.output.smoothed.next_step(buffer.samples() as u32);

        let channels = buffer.as_slice();
        if channels.len() >= 2 {
            let (l, rest) = channels.split_at_mut(1);
            engine.process(&mut l[0][..], &mut rest[0][..]);
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for BcDrums {
    const CLAP_ID: &'static str = "br.com.bcstudio.bcdrums";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Bateria crua entra, bateria pronta sai");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::AudioEffect, ClapFeature::Stereo, ClapFeature::Drum, ClapFeature::Mixing];
}

impl Vst3Plugin for BcDrums {
    const VST3_CLASS_ID: [u8; 16] = *b"BCStudioBCDrums1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Drum, Vst3SubCategory::Dynamics];
}

nih_export_clap!(BcDrums);
nih_export_vst3!(BcDrums);
