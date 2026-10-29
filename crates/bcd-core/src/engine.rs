//! Motor do BC Drums: LR crua entra, LR pronta sai.
//!
//! Caminho do áudio (por amostra):
//! 1. o detector analisa a entrada e encontra bumbo/caixa/tons;
//! 2. a entrada é atrasada pela latência do detector (informada ao host);
//! 3. o original atrasado passa pela EQ do estilo;
//! 4. cada batida dispara o sample do pack (camada pela força, alternando);
//! 5. original + samples → Transient Design (só graves/médios, abaixo de
//!    4 kHz, onde estão bumbo, caixa e tons) → compressão + paralela →
//!    reverb → saturação;
//! 6. Mix entre o original atrasado e o resultado.

use crate::detect::{Detector, Hit};
use crate::dsp::*;
use crate::mixbus::{Limiter, MixBus};
use crate::pack::{ChainSettings, Library, Pack, Piece};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// índice do estilo (pack) na biblioteca
    pub style: usize,
    /// Transient Design 0..1
    pub transient: f32,
    /// Mix 0..1 (0 = original, 1 = BC Drums)
    pub mix: f32,
    pub output_db: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self { style: 0, transient: 0.5, mix: 1.0, output_db: 0.0 }
    }
}

/// Medidores compartilhados com a interface (lidos sem trava).
pub struct Meters {
    in_rms: AtomicU32,
    out_rms: AtomicU32,
    /// fila circular das últimas batidas: (tempo << 8) | peça << 5 | velocity(0..31)
    hits: [AtomicU64; 64],
    hit_head: AtomicU32,
    /// posição atual da saída (amostras) — para a tela rolar junto com o play
    pub position: AtomicU64,
}

impl Default for Meters {
    fn default() -> Self {
        Self {
            in_rms: AtomicU32::new(0),
            out_rms: AtomicU32::new(0),
            hits: std::array::from_fn(|_| AtomicU64::new(0)),
            hit_head: AtomicU32::new(0),
            position: AtomicU64::new(0),
        }
    }
}

impl Meters {
    pub fn in_level(&self) -> f32 {
        f32::from_bits(self.in_rms.load(Ordering::Relaxed))
    }
    pub fn out_level(&self) -> f32 {
        f32::from_bits(self.out_rms.load(Ordering::Relaxed))
    }
    fn push_hit(&self, h: &Hit) {
        let v = ((h.velocity * 31.0) as u64).min(31);
        let packed = (h.time << 8) | ((h.piece.index() as u64) << 5) | v;
        let i = self.hit_head.fetch_add(1, Ordering::Relaxed) as usize % 64;
        self.hits[i].store(packed | (1u64 << 63), Ordering::Relaxed);
    }
    /// Últimas batidas (tempo, peça, velocity), da mais antiga para a mais nova.
    pub fn recent_hits(&self) -> Vec<(u64, Piece, f32)> {
        let head = self.hit_head.load(Ordering::Relaxed) as usize;
        let n = head.min(64);
        (head - n..head)
            .filter_map(|i| {
                let p = self.hits[i % 64].load(Ordering::Relaxed);
                if p == 0 {
                    return None;
                }
                let p = p & !(1u64 << 63);
                let piece = Piece::ALL.get(((p >> 5) & 0x7) as usize).copied()?;
                Some((p >> 8, piece, (p & 31) as f32 / 31.0))
            })
            .collect()
    }
}

struct Voice {
    piece: Piece,
    layer: usize,
    pos: usize,
    gain: f32,
}

struct Chain {
    bus: MixBus,
    limiter: Limiter,
    glue: Compressor,
    smash: Compressor,
    reverb: Reverb,
    s: ChainSettings,
    orig_gain: f32,
    samples_gain: f32,
    out_gain: f32,
}

impl Chain {
    fn new(sr: f32, s: &ChainSettings) -> Self {
        Self {
            bus: MixBus::new(sr, s),
            limiter: Limiter::new(sr, s.ceiling_db),
            glue: Compressor::new(sr, s.comp_threshold_db, s.comp_ratio, s.comp_attack_ms, s.comp_release_ms, 0.0),
            smash: Compressor::new(sr, -32.0, 10.0, 0.5, 70.0, 18.0),
            reverb: Reverb::new(sr, s.reverb_decay_s, s.reverb_predelay_ms, s.reverb_damping_hz),
            orig_gain: db_to_lin(s.original_db),
            samples_gain: db_to_lin(s.samples_db),
            out_gain: db_to_lin(s.output_db),
            s: s.clone(),
        }
    }
}

/// Biblioteca já preparada para o áudio (cadeias pré-alocadas). Montada fora
/// da thread de áudio e entregue ao motor com [`Engine::install`].
pub struct Prepared {
    library: Arc<Library>,
    chains: Vec<Chain>,
    fallback: Chain,
}

impl Prepared {
    pub fn new(library: Arc<Library>, sr: f32) -> Self {
        let chains = library.packs.iter().map(|p| Chain::new(sr, &p.chain)).collect();
        Self { library, chains, fallback: Chain::new(sr, &ChainSettings::default()) }
    }
    pub fn library(&self) -> &Arc<Library> {
        &self.library
    }
}

pub struct Engine {
    sr: f32,
    latency: usize,
    prepared: Prepared,
    delay: Vec<[f32; 2]>,
    wpos: usize,
    now: u64,
    detector: Detector,
    pending: VecDeque<Hit>,
    voices: Vec<Voice>,
    rr: [usize; 4],
    pub params: Params,
    current_style: usize,
    meters: Arc<Meters>,
    in_env: Envelope,
    out_env: Envelope,
    /// batidas já analisadas (modo offline); quando presente, o detector em tempo real é ignorado
    preset_hits: bool,
}

impl Engine {
    pub fn new(sr: f32, library: Arc<Library>) -> Self {
        let detector = Detector::new(sr);
        let latency = detector.decision_delay() as usize + (0.004 * sr) as usize;
        Self {
            sr,
            latency,
            prepared: Prepared::new(library, sr),
            delay: vec![[0.0; 2]; latency + 1],
            wpos: 0,
            now: 0,
            detector,
            pending: VecDeque::with_capacity(64),
            voices: Vec::with_capacity(64),
            rr: [0; 4],
            params: Params::default(),
            current_style: usize::MAX,
            meters: Arc::new(Meters::default()),
            in_env: Envelope::new(sr, 30.0, 300.0),
            out_env: Envelope::new(sr, 30.0, 300.0),
            preset_hits: false,
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sr
    }
    /// Latência em amostras (o host compensa).
    pub fn latency(&self) -> usize {
        self.latency
    }
    pub fn meters(&self) -> Arc<Meters> {
        self.meters.clone()
    }
    pub fn library(&self) -> &Arc<Library> {
        &self.prepared.library
    }
    pub fn current_pack(&self) -> Option<&Arc<Pack>> {
        self.prepared.library.packs.get(self.params.style)
    }

    /// Troca a biblioteca (sem alocar na thread de áudio). Devolve a antiga
    /// para ser descartada fora da thread de áudio.
    pub fn install(&mut self, prepared: Prepared) -> Prepared {
        self.voices.clear();
        self.current_style = usize::MAX;
        std::mem::replace(&mut self.prepared, prepared)
    }

    /// Usa batidas já analisadas (ex.: [`crate::detect::analyze_song`]) em vez
    /// do detector em tempo real. Tempos em amostras desde o início do áudio.
    pub fn set_hits(&mut self, mut hits: Vec<Hit>) {
        hits.sort_by_key(|h| h.time);
        self.pending = hits.into();
        self.preset_hits = true;
    }

    pub fn reset(&mut self) {
        for d in self.delay.iter_mut() {
            *d = [0.0; 2];
        }
        self.detector.reset();
        self.pending.clear();
        self.voices.clear();
        self.now = 0;
    }

    fn trigger(&mut self, hit: Hit) {
        let Some(pack) = self.prepared.library.packs.get(self.params.style) else { return };
        let kit = pack.piece(hit.piece);
        let n = kit.layers.len();
        if n == 0 {
            return;
        }
        let mut idx = ((hit.velocity * (n as f32 - 1.0)).round() as usize).min(n - 1);
        // alterna com a camada vizinha para não soar como metralhadora
        let rr = &mut self.rr[hit.piece.index()];
        *rr += 1;
        if n > 2 && *rr % 2 == 1 {
            idx = if idx + 1 < n && hit.velocity > 0.5 { idx } else { idx.saturating_sub(1) };
        }
        if self.voices.len() >= 48 {
            self.voices.remove(0);
        }
        let gain = (0.45 + 0.55 * hit.velocity) * db_to_lin(kit.settings.gain_db);
        self.voices.push(Voice { piece: hit.piece, layer: idx, pos: kit.layers[idx].onset, gain });
    }

    /// Processa um bloco estéreo no lugar.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let style = self.params.style.min(self.prepared.library.packs.len().saturating_sub(1));
        if style != self.current_style {
            self.current_style = style;
            self.voices.clear();
        }
        let has_pack = !self.prepared.library.packs.is_empty();
        let pack = self.prepared.library.packs.get(style).cloned();
        let transient = self.params.transient.clamp(0.0, 1.0);
        let mix = self.params.mix.clamp(0.0, 1.0);
        let out_gain = db_to_lin(self.params.output_db);
        let len = self.delay.len();
        let mut in_peak: f32 = 0.0;
        let mut out_peak: f32 = 0.0;

        for i in 0..left.len().min(right.len()) {
            let (l, r) = (left[i], right[i]);
            in_peak = in_peak.max(self.in_env.process(l.abs().max(r.abs())));

            if !self.preset_hits {
                if let Some(hit) = self.detector.process(0.5 * (l + r)) {
                    self.pending.push_back(hit);
                    self.meters.push_hit(&hit);
                }
            }

            // linha de atraso
            self.delay[self.wpos] = [l, r];
            self.wpos = (self.wpos + 1) % len;
            let dry = self.delay[self.wpos];
            let out_time = self.now.saturating_sub(self.latency as u64);
            self.now += 1;

            while let Some(h) = self.pending.front() {
                if h.time <= out_time {
                    let h = *h;
                    self.pending.pop_front();
                    self.trigger(h);
                } else {
                    break;
                }
            }

            let chain = if has_pack { &mut self.prepared.chains[style] } else { &mut self.prepared.fallback };

            // mixagem completa por faixas (todas as peças)
            let (ml, mr, body_send) = chain.bus.process(dry[0], dry[1], transient);
            let o = [ml * chain.orig_gain, mr * chain.orig_gain];

            // reforço com samples do pack (opcional, pelo detector)
            let mut sm = [0.0f32; 2];
            let mut send = 0.0f32;
            if let Some(pack) = pack.as_ref() {
                self.voices.retain_mut(|v| {
                    let kit = pack.piece(v.piece);
                    let Some(layer) = kit.layers.get(v.layer) else { return false };
                    let Some(fr) = layer.frames.get(v.pos) else { return false };
                    sm[0] += fr[0] * v.gain;
                    sm[1] += fr[1] * v.gain;
                    send += 0.5 * (fr[0] + fr[1]) * v.gain * kit.settings.reverb_send;
                    v.pos += 1;
                    true
                });
            }
            let mut bus = [o[0] + sm[0] * chain.samples_gain, o[1] + sm[1] * chain.samples_gain];

            // cola + compressão paralela
            let gg = chain.glue.gain(bus[0], bus[1]);
            let gs = chain.smash.gain(bus[0], bus[1]);
            let p = chain.s.parallel;
            bus = [bus[0] * gg + bus[0] * gs * p, bus[1] * gg + bus[1] * gs * p];

            // reverb (sala da caixa e dos tons + samples)
            let rv_in = send * chain.samples_gain + body_send * chain.orig_gain;
            let (rl, rr) = chain.reverb.process(rv_in);
            bus[0] += rl * chain.s.reverb_return;
            bus[1] += rr * chain.s.reverb_return;

            // saturação e saída
            let (wl, wr) = chain.limiter.process(
                saturate(bus[0] * chain.out_gain, chain.s.drive),
                saturate(bus[1] * chain.out_gain, chain.s.drive),
            );
            let wet = [wl, wr];
            let ol = (dry[0] * (1.0 - mix) + wet[0] * mix) * out_gain;
            let or = (dry[1] * (1.0 - mix) + wet[1] * mix) * out_gain;
            left[i] = ol;
            right[i] = or;
            out_peak = out_peak.max(self.out_env.process(ol.abs().max(or.abs())));
        }
        self.meters.in_rms.store(in_peak.to_bits(), Ordering::Relaxed);
        self.meters.out_rms.store(out_peak.to_bits(), Ordering::Relaxed);
        self.meters.position.store(self.now.saturating_sub(self.latency as u64), Ordering::Relaxed);
    }
}

/// Processa um arquivo inteiro (uso offline: app separado e testes).
/// Compensa a latência para que a saída fique alinhada com a entrada.
pub fn process_offline(engine: &mut Engine, input: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let lat = engine.latency();
    let tail = (engine.sample_rate() * 3.0) as usize;
    let total = input.len() + lat + tail;
    let mut l: Vec<f32> = Vec::with_capacity(total);
    let mut r: Vec<f32> = Vec::with_capacity(total);
    for f in input {
        l.push(f[0]);
        r.push(f[1]);
    }
    l.resize(total, 0.0);
    r.resize(total, 0.0);
    for (cl, cr) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
        engine.process(cl, cr);
    }
    l.iter().zip(r.iter()).skip(lat).map(|(&a, &b)| [a, b]).collect()
}
