//! Detector de transientes da LR da bateria.
//!
//! Funciona em tempo real com uma pequena janela de análise depois de cada
//! ataque: detecta o ataque, mede a energia em 4 faixas durante a janela
//! e decide se foi bumbo, caixa, tom agudo ou tom grave (ou prato/chimbal,
//! que é ignorado). O motor atrasa o áudio pelo mesmo tempo (latência
//! informada ao host), então o sample entra exatamente no ataque original.

use crate::dsp::{lin_to_db, Biquad, Envelope};
use crate::pack::Piece;

/// Uma batida detectada.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// posição absoluta (em amostras) do ataque
    pub time: u64,
    pub piece: Piece,
    /// força 0..1
    pub velocity: f32,
}

/// Medidas de uma batida (útil para calibrar e para a tela de transientes).
#[derive(Clone, Copy, Debug, Default)]
pub struct Features {
    pub low: f32,
    pub low_mid: f32,
    pub mid: f32,
    pub high: f32,
    pub low_tail: f32,
    pub low_mid_tail: f32,
    pub peak_db: f32,
    /// frequência fundamental estimada na cauda (Hz), 0 se não deu
    pub pitch_hz: f32,
}

impl Features {
    pub fn total(&self) -> f32 {
        (self.low + self.low_mid + self.mid + self.high).max(1e-12)
    }
}

#[derive(Clone, Debug)]
pub struct DetectorSettings {
    /// diferença rápido/lento (dB) para considerar ataque
    pub onset_db: f32,
    /// intervalo mínimo entre ataques (ms)
    pub refractory_ms: f32,
    /// abaixo do pico recente menos isso, ignora (dB)
    pub gate_range_db: f32,
}

impl Default for DetectorSettings {
    fn default() -> Self {
        Self { onset_db: 6.0, refractory_ms: 45.0, gate_range_db: 28.0 }
    }
}

struct Pending {
    time: u64,
    f: Features,
    last_sign: bool,
    first_cross: u32,
    last_cross: u32,
    crosses: u32,
}

pub struct Detector {
    sr: f32,
    pub settings: DetectorSettings,
    // filtros
    onset_lp: [Biquad; 2],
    low: [Biquad; 2],
    low_mid: Biquad,
    mid: Biquad,
    high: [Biquad; 2],
    pitch_lp: [Biquad; 2],
    fast: Envelope,
    slow: Envelope,
    peak_hold: f32,
    peak_decay: f32,
    refractory: u32,
    since_onset: u32,
    w1: u32,
    w2: u32,
    w_pitch: u32,
    /// afinações recentes dos tons (para separar agudo/grave em qualquer kit)
    tom_pitches: [f32; 16],
    tom_pitch_n: usize,
    pending: Option<Pending>,
    /// pico recente por peça, para a velocity relativa
    piece_max_db: [f32; 4],
    /// batida mais forte recente (qualquer peça)
    global_max_db: f32,
    max_decay_db: f32,
    now: u64,
    pub last_features: Features,
    /// último ataque analisado (com ou sem classificação), para a análise da música inteira
    pub completed: Option<(u64, Features)>,
}

impl Detector {
    pub fn new(sr: f32) -> Self {
        let q = std::f32::consts::FRAC_1_SQRT_2;
        let settings = DetectorSettings::default();
        Self {
            sr,
            onset_lp: [Biquad::lowpass(sr, 2500.0, q); 2],
            low: [Biquad::lowpass(sr, 95.0, q); 2],
            low_mid: Biquad::bandpass(sr, 165.0, 1.1),
            mid: Biquad::bandpass(sr, 600.0, 0.8),
            high: [Biquad::highpass(sr, 2800.0, q); 2],
            pitch_lp: [Biquad::lowpass(sr, 220.0, q); 2],
            fast: Envelope::new(sr, 0.3, 12.0),
            slow: Envelope::new(sr, 20.0, 90.0),
            peak_hold: 0.0,
            peak_decay: (-1.0 / (2.5 * sr)).exp(),
            refractory: (settings.refractory_ms * 1e-3 * sr) as u32,
            since_onset: u32::MAX,
            w1: (0.018 * sr) as u32,
            w2: (0.038 * sr) as u32,
            w_pitch: (0.004 * sr) as u32,
            tom_pitches: [0.0; 16],
            tom_pitch_n: 0,
            pending: None,
            piece_max_db: [-60.0; 4],
            global_max_db: -60.0,
            max_decay_db: 3.0 / sr, // 3 dB por segundo
            now: 0,
            last_features: Features::default(),
            completed: None,
            settings,
        }
    }

    /// Quantas amostras depois do ataque a decisão sai (o motor precisa de
    /// pelo menos isso de latência).
    pub fn decision_delay(&self) -> u32 {
        self.w2 + 2
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.sr);
    }

    /// Alimenta uma amostra mono. Devolve uma batida quando a janela de
    /// análise de um ataque termina.
    #[inline]
    pub fn process(&mut self, x: f32) -> Option<Hit> {
        let t = self.now;
        self.now += 1;

        let ol = self.onset_lp[0].process(x);
        let ol = self.onset_lp[1].process(ol);
        let lo = self.low[0].process(x);
        let lo = self.low[1].process(lo);
        let lm = self.low_mid.process(x);
        let md = self.mid.process(x);
        let hi = self.high[0].process(x);
        let hi = self.high[1].process(hi);
        let pl = self.pitch_lp[0].process(x);
        let pl = self.pitch_lp[1].process(pl);

        let f = self.fast.process(ol);
        let s = self.slow.process(ol);
        self.peak_hold = (self.peak_hold * self.peak_decay).max(f);
        for m in self.piece_max_db.iter_mut() {
            *m = (*m - self.max_decay_db).max(-60.0);
        }
        self.global_max_db = (self.global_max_db - self.max_decay_db / 3.0).max(-60.0);
        self.since_onset = self.since_onset.saturating_add(1);

        // acumula energia da batida em análise
        let mut out = None;
        if let Some(p) = self.pending.as_mut() {
            let k = (t - p.time) as u32;
            // cruzamentos por zero (subindo) da cauda filtrada → afinação
            let sign = pl > 0.0;
            if k > self.w_pitch && k < self.w2 && sign && !p.last_sign {
                if p.crosses == 0 {
                    p.first_cross = k;
                }
                p.last_cross = k;
                p.crosses += 1;
            }
            p.last_sign = sign;
            if k < self.w1 {
                p.f.low += lo * lo;
                p.f.low_mid += lm * lm;
                p.f.mid += md * md;
                p.f.high += hi * hi;
                p.f.peak_db = p.f.peak_db.max(lin_to_db(f));
            } else if k < self.w2 {
                p.f.low_tail += lo * lo;
                p.f.low_mid_tail += lm * lm;
            } else {
                if p.crosses >= 2 && p.last_cross > p.first_cross {
                    p.f.pitch_hz = (p.crosses - 1) as f32 * self.sr / (p.last_cross - p.first_cross) as f32;
                }
                let done = Pending { time: p.time, f: p.f, last_sign: false, first_cross: 0, last_cross: 0, crosses: 0 };
                self.pending = None;
                self.last_features = done.f;
                self.completed = Some((done.time, done.f));
                let growth = done.f.low_tail / done.f.low.max(1e-12);
                let piece = classify(&done.f).map(|p| self.refine_tom(p, done.f.pitch_hz, growth));
                // muito abaixo das batidas principais (vazamento, prato): ignora
                let audible = done.f.peak_db > self.global_max_db - 24.0;
                self.global_max_db = self.global_max_db.max(done.f.peak_db);
                if let Some(piece) = piece.filter(|_| audible) {
                    let i = piece.index();
                    // a primeira batida de cada peça usa a referência geral
                    if self.piece_max_db[i] <= -59.0 {
                        self.piece_max_db[i] = done.f.peak_db.max(self.global_max_db - 6.0);
                    }
                    self.piece_max_db[i] = self.piece_max_db[i].max(done.f.peak_db);
                    let rel = done.f.peak_db - self.piece_max_db[i];
                    if rel > -24.0 {
                        let velocity = (1.0 + rel / 20.0).clamp(0.05, 1.0);
                        out = Some(Hit { time: done.time, piece, velocity });
                    }
                }
            }
        }

        // novo ataque?
        let fdb = lin_to_db(f);
        let gate = (lin_to_db(self.peak_hold) - self.settings.gate_range_db).max(-62.0);
        if self.pending.is_none()
            && self.since_onset > self.refractory
            && fdb - lin_to_db(s) > self.settings.onset_db
            && fdb > gate
        {
            // o ataque real começa um pouco antes do cruzamento do limiar
            let back = 0u64;
            self.pending = Some(Pending { time: t.saturating_sub(back), f: Features { peak_db: fdb, ..Default::default() }, last_sign: false, first_cross: 0, last_cross: 0, crosses: 0 });
            self.since_onset = 0;
        }
        out
    }
}

impl Detector {
    /// Separa tom agudo/grave pela afinação, comparando com os tons já
    /// ouvidos nesta música (funciona com qualquer afinação de kit).
    fn refine_tom(&mut self, p: Piece, pitch: f32, low_growth: f32) -> Piece {
        if !matches!(p, Piece::TomHi | Piece::TomLo) {
            return p;
        }
        // o surdo/tom grave continua crescendo no grave depois do ataque
        if low_growth > 1.5 {
            return Piece::TomLo;
        }
        if !(40.0..400.0).contains(&pitch) {
            return p;
        }
        let i = self.tom_pitch_n % self.tom_pitches.len();
        self.tom_pitches[i] = pitch;
        self.tom_pitch_n += 1;
        let n = self.tom_pitch_n.min(self.tom_pitches.len());
        let seen = &self.tom_pitches[..n];
        let lo = seen.iter().copied().fold(f32::MAX, f32::min);
        let hi = seen.iter().copied().fold(0.0, f32::max);
        let split = if hi / lo > 1.15 { (lo * hi).sqrt() } else { 95.0 };
        if pitch >= split { Piece::TomHi } else { Piece::TomLo }
    }
}

/// Regras de classificação a partir das energias por faixa.
///
/// Calibradas com um kit multipista real (bumbo, caixa grave, tons e pratos):
/// - bumbo: o grave continua crescendo depois do ataque (sub/ressonância);
/// - caixa: pouco grave e esteira presente (agudos ~10x maiores que nos tons);
/// - tons: grave + grave-médio; a proporção entre eles separa agudo e grave;
/// - pratos/chimbal: quase só agudos → ignorados.
pub fn classify(f: &Features) -> Option<Piece> {
    let tot = f.total();
    let low = f.low / tot;
    let lm = f.low_mid / tot;
    let mid = f.mid / tot;
    let high = f.high / tot;
    let low_growth = f.low_tail / f.low.max(1e-12);
    let body = low + lm;

    if high > 0.45 && low < 0.08 && lm < 0.2 {
        return None; // prato / chimbal
    }
    if low_growth > 3.2 && body > 0.3 {
        return Some(Piece::Kick);
    }
    if low > 0.7 {
        return Some(Piece::Kick);
    }
    if high >= 0.022 && low < 0.10 && (lm + mid) > 0.5 {
        return Some(Piece::Snare);
    }
    if body > 0.55 {
        let ratio = low / body.max(1e-6);
        return Some(if ratio > 0.28 { Piece::TomLo } else { Piece::TomHi });
    }
    if mid > 0.5 && high < 0.02 {
        return Some(Piece::TomHi);
    }
    None
}

/// Análise da música inteira (modo offline do app).
///
/// 1. acha todos os ataques e mede cada um;
/// 2. classifica pelas regras gerais (semente);
/// 3. aprende o "perfil" de cada peça *desta* gravação (média e dispersão
///    das medidas) e reclassifica cada ataque pelo perfil mais próximo;
/// 4. ataques que não se parecem com nenhum perfil ficam sem reforço.
///
/// O resultado adapta-se ao kit, à afinação e à captação de cada música.
pub fn analyze_song(mono: &[f32], sr: f32) -> Vec<Hit> {
    let mut d = Detector::new(sr);
    let mut onsets: Vec<(u64, Features)> = Vec::new();
    for &x in mono.iter().chain(std::iter::repeat_n(&0.0, (0.1 * sr) as usize)) {
        d.process(x);
        if let Some(o) = d.completed.take() {
            onsets.push(o);
        }
    }
    if onsets.is_empty() {
        return Vec::new();
    }
    // nível: ignora vazamentos muito abaixo das batidas principais da música
    let mut peaks: Vec<f32> = onsets.iter().map(|o| o.1.peak_db).collect();
    peaks.sort_by(|a, b| a.total_cmp(b));
    let loud = peaks[(peaks.len() as f32 * 0.9) as usize % peaks.len()];

    let vec_of = |f: &Features| -> [f32; 6] {
        let t = f.total();
        let l = |v: f32| (v.max(1e-9)).ln();
        [
            l(f.low / t),
            l(f.low_mid / t),
            l(f.mid / t),
            l(f.high / t),
            l(f.low_tail / f.low.max(1e-12)).clamp(-4.0, 4.0),
            if f.pitch_hz > 0.0 { (f.pitch_hz.max(30.0)).ln() } else { 4.6 },
        ]
    };
    let feats: Vec<[f32; 6]> = onsets.iter().map(|o| vec_of(&o.1)).collect();
    // semente: regras gerais (0..3 peças, 4 = nada)
    let mut label: Vec<usize> = onsets
        .iter()
        .map(|o| {
            let growth = o.1.low_tail / o.1.low.max(1e-12);
            match classify(&o.1) {
                Some(Piece::TomHi | Piece::TomLo) if growth > 1.5 => Piece::TomLo.index(),
                Some(p) => p.index(),
                None => 4,
            }
        })
        .collect();

    for _ in 0..6 {
        // perfis (média e desvio por dimensão) das classes com exemplos suficientes
        let mut prof: Vec<Option<([f32; 6], [f32; 6])>> = vec![None; 5];
        for (c, pc) in prof.iter_mut().enumerate() {
            let idx: Vec<usize> = (0..label.len()).filter(|&i| label[i] == c).collect();
            if idx.len() < 3 {
                continue;
            }
            let mut m = [0.0f32; 6];
            for &i in &idx {
                for k in 0..6 {
                    m[k] += feats[i][k] / idx.len() as f32;
                }
            }
            let mut sd = [0.0f32; 6];
            for &i in &idx {
                for k in 0..6 {
                    sd[k] += (feats[i][k] - m[k]).powi(2) / idx.len() as f32;
                }
            }
            for v in sd.iter_mut() {
                *v = v.sqrt().max(0.25);
            }
            *pc = Some((m, sd));
        }
        let mut changed = false;
        for i in 0..label.len() {
            let mut best = (f32::MAX, label[i]);
            for (c, pc) in prof.iter().enumerate() {
                if let Some((m, sd)) = pc {
                    let dist: f32 = (0..6).map(|k| ((feats[i][k] - m[k]) / sd[k]).powi(2)).sum::<f32>() / 6.0;
                    if dist < best.0 {
                        best = (dist, c);
                    }
                }
            }
            // longe de todos os perfis: sem reforço (evita erro)
            let new = if best.0 > 5.0 { 4 } else { best.1 };
            if new != label[i] {
                label[i] = new;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // separa tom agudo/grave pela afinação dentro dos tons da música
    let tom_pitches: Vec<f32> = (0..label.len())
        .filter(|&i| (label[i] == Piece::TomHi.index() || label[i] == Piece::TomLo.index()) && onsets[i].1.pitch_hz > 0.0)
        .map(|i| onsets[i].1.pitch_hz)
        .collect();
    if tom_pitches.len() >= 4 {
        let lo = tom_pitches.iter().copied().fold(f32::MAX, f32::min);
        let hi = tom_pitches.iter().copied().fold(0.0, f32::max);
        if hi / lo > 1.15 {
            let split = (lo * hi).sqrt();
            for i in 0..label.len() {
                if label[i] == Piece::TomHi.index() || label[i] == Piece::TomLo.index() {
                    let growth = onsets[i].1.low_tail / onsets[i].1.low.max(1e-12);
                    let p = onsets[i].1.pitch_hz;
                    label[i] = if growth > 1.5 || (p > 0.0 && p < split) { Piece::TomLo.index() } else { Piece::TomHi.index() };
                }
            }
        }
    }

    // força relativa ao pico típico de cada peça na música
    let mut ref_db = [-60.0f32; 4];
    for (c, r) in ref_db.iter_mut().enumerate() {
        let mut v: Vec<f32> = (0..label.len()).filter(|&i| label[i] == c).map(|i| onsets[i].1.peak_db).collect();
        if !v.is_empty() {
            v.sort_by(|a, b| a.total_cmp(b));
            *r = v[((v.len() as f32 * 0.9) as usize).min(v.len() - 1)];
        }
    }
    (0..label.len())
        .filter(|&i| label[i] < 4 && onsets[i].1.peak_db > loud - 26.0)
        .filter_map(|i| {
            let piece = Piece::ALL.get(label[i]).copied()?;
            let rel = onsets[i].1.peak_db - ref_db[label[i]];
            Some(Hit { time: onsets[i].0, piece, velocity: (1.0 + rel / 20.0).clamp(0.05, 1.0) })
        })
        .collect()
}
