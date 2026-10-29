//! Mixagem completa da LR da bateria, por faixas de frequência.
//!
//! Não depende de detectar as batidas: cada peça da bateria vive numa faixa
//! e recebe tratamento próprio.
//!
//! | faixa    | onde              | peças principais                  |
//! |----------|-------------------|-----------------------------------|
//! | grave    | < 120 Hz          | bumbo, fundamental do surdo       |
//! | corpo    | 120 – 800 Hz      | corpo da caixa e dos tons         |
//! | ataque   | 800 Hz – 6 kHz    | estalo da caixa, batida do bumbo, ataque dos tons, chimbal |
//! | ar       | > 6 kHz           | pratos, chimbal, brilho da sala   |
//!
//! Em cada faixa: Transient Design (peso por faixa), compressor e ganho do
//! estilo. Os pratos ganham abertura estéreo. As faixas somam plano
//! (crossovers Linkwitz-Riley com compensação de fase).

use crate::dsp::*;
use crate::pack::ChainSettings;

const XO: [f32; 3] = [120.0, 800.0, 6000.0];
/// peso do Transient Design em cada faixa (pratos quase não mexem)
const TD_WEIGHT: [f32; 4] = [1.0, 0.9, 0.8, 0.15];

/// Allpass de 4ª ordem = soma das saídas de um crossover LR4.
#[derive(Clone)]
struct Allpass {
    x: Crossover,
}

impl Allpass {
    fn new(sr: f32, f: f32) -> Self {
        Self { x: Crossover::new(sr, f) }
    }
    #[inline]
    fn process(&mut self, v: f32) -> f32 {
        let (a, b) = self.x.split(v);
        a + b
    }
}

/// Divide um canal em 4 faixas que somam plano.
#[derive(Clone)]
struct Splitter {
    x1: Crossover,
    x2: Crossover,
    x3: Crossover,
    ap_low: [Allpass; 2],
    ap_body: Allpass,
}

impl Splitter {
    fn new(sr: f32) -> Self {
        Self {
            x1: Crossover::new(sr, XO[0]),
            x2: Crossover::new(sr, XO[1]),
            x3: Crossover::new(sr, XO[2]),
            ap_low: [Allpass::new(sr, XO[1]), Allpass::new(sr, XO[2])],
            ap_body: Allpass::new(sr, XO[2]),
        }
    }
    #[inline]
    fn split(&mut self, v: f32) -> [f32; 4] {
        let (low, rest) = self.x1.split(v);
        let (body, rest2) = self.x2.split(rest);
        let (attack, air) = self.x3.split(rest2);
        let low = self.ap_low[0].process(low);
        let low = self.ap_low[1].process(low);
        let body = self.ap_body.process(body);
        [low, body, attack, air]
    }
}

pub struct MixBus {
    split: [Splitter; 2],
    shapers: [TransientShaper; 4],
    comps: [Compressor; 4],
    band_gain: [f32; 4],
    width: f32,
    eq: [[Biquad; 3]; 2],
    // ganho automático (deixa qualquer gravação no mesmo nível de trabalho)
    auto_gain: bool,
    target_db: f32,
    rms: Envelope,
    gain_db: f32,
    gain_k: f32,
    /// quanto do corpo + ataque vai para o reverb (sala da caixa/tons)
    pub reverb_send: f32,
}

impl MixBus {
    pub fn new(sr: f32, s: &ChainSettings) -> Self {
        let amt = s.band_comp.clamp(0.0, 1.0);
        let ratio = |base: f32| 1.0 + (base - 1.0) * (0.3 + 1.4 * amt);
        let comps = [
            Compressor::new(sr, -22.0, ratio(3.0), 18.0, 140.0, 0.0),
            Compressor::new(sr, -24.0, ratio(3.0), 8.0, 150.0, 0.0),
            Compressor::new(sr, -26.0, ratio(2.5), 4.0, 90.0, 0.0),
            Compressor::new(sr, -30.0, ratio(2.2), 1.0, 70.0, 0.0),
        ];
        let eq = |_: usize| {
            [
                Biquad::low_shelf(sr, s.low_shelf_hz, s.low_shelf_db),
                Biquad::peaking(sr, s.mid_hz, s.mid_db, s.mid_q),
                Biquad::high_shelf(sr, s.high_shelf_hz, s.high_shelf_db),
            ]
        };
        Self {
            split: [Splitter::new(sr), Splitter::new(sr)],
            shapers: std::array::from_fn(|_| TransientShaper::new(sr)),
            comps,
            band_gain: [db_to_lin(s.kick_db), db_to_lin(s.body_db), db_to_lin(s.attack_db), db_to_lin(s.air_db)],
            width: s.width.clamp(0.0, 2.0),
            eq: [eq(0), eq(1)],
            auto_gain: s.auto_gain,
            target_db: s.target_db,
            rms: Envelope::new(sr, 300.0, 1500.0),
            gain_db: 0.0,
            gain_k: 1.0 - (-1.0 / (2.0 * sr)).exp(),
            reverb_send: s.reverb_send_original,
        }
    }

    /// Ganho automático atual (linear).
    pub fn gain(&self) -> f32 {
        if self.auto_gain { db_to_lin(self.gain_db) } else { 1.0 }
    }

    /// Processa uma amostra estéreo. Devolve (esquerda, direita, envio de reverb).
    #[inline]
    pub fn process(&mut self, l: f32, r: f32, transient: f32) -> (f32, f32, f32) {
        // ganho automático lento, só com sinal presente
        let (l, r) = if self.auto_gain {
            let lvl = lin_to_db(self.rms.process(0.5 * (l.abs() + r.abs())));
            if lvl > -55.0 {
                let want = (self.target_db - lvl).clamp(-12.0, 18.0);
                self.gain_db += (want - self.gain_db) * self.gain_k;
            }
            let g = db_to_lin(self.gain_db);
            (l * g, r * g)
        } else {
            (l, r)
        };

        let bl = self.split[0].split(l);
        let br = self.split[1].split(r);
        let mut out = [0.0f32; 2];
        let mut send = 0.0;
        for b in 0..4 {
            let (mut x, mut y) = (bl[b], br[b]);
            self.shapers[b].amount = transient * TD_WEIGHT[b];
            let g = self.shapers[b].gain(x, y) * self.comps[b].gain(x, y) * self.band_gain[b];
            x *= g;
            y *= g;
            if b == 3 {
                // abertura estéreo dos pratos (M/S)
                let m = 0.5 * (x + y);
                let s = 0.5 * (x - y) * self.width;
                x = m + s;
                y = m - s;
            }
            if b == 1 || b == 2 {
                send += 0.5 * (x + y);
            }
            out[0] += x;
            out[1] += y;
        }
        for (o, eq) in out.iter_mut().zip(self.eq.iter_mut()) {
            for f in eq.iter_mut() {
                *o = f.process(*o);
            }
        }
        (out[0], out[1], send * self.reverb_send)
    }
}

/// Limitador de saída: compressão muito rápida + clip suave no teto.
pub struct Limiter {
    env: Envelope,
    ceiling: f32,
}

impl Limiter {
    pub fn new(sr: f32, ceiling_db: f32) -> Self {
        Self { env: Envelope::new(sr, 0.2, 80.0), ceiling: db_to_lin(ceiling_db) }
    }
    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let pk = self.env.process(l.abs().max(r.abs()));
        let g = if pk > self.ceiling { self.ceiling / pk } else { 1.0 };
        let c = self.ceiling;
        let clip = |v: f32| c * (v / c).tanh();
        (clip(l * g), clip(r * g))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_sum_flat() {
        let sr = 48_000.0;
        let mut sp = Splitter::new(sr);
        for f in [60.0f32, 300.0, 2000.0, 10_000.0] {
            let mut e_in = 0.0;
            let mut e_out = 0.0;
            for n in 0..48_000 {
                let s = (2.0 * std::f32::consts::PI * f * n as f32 / sr).sin();
                let b = sp.split(s);
                let y: f32 = b.iter().sum();
                if n > 9600 {
                    e_in += s * s;
                    e_out += y * y;
                }
            }
            let db = 10.0 * (e_out / e_in).log10();
            assert!(db.abs() < 0.5, "{f} Hz: {db} dB");
        }
    }
}
