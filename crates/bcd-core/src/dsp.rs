//! Blocos de DSP usados pelo motor: filtros biquad, envelopes, compressor,
//! transient shaper, crossover Linkwitz-Riley, saturação e reverb FDN.

use std::f32::consts::PI;

#[inline]
pub fn db_to_lin(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

#[inline]
pub fn lin_to_db(x: f32) -> f32 {
    20.0 * (x.max(1e-9)).log10()
}

/// Biquad em forma direta transposta II.
#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Default for Biquad {
    fn default() -> Self {
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, z1: 0.0, z2: 0.0 }
    }
}

impl Biquad {
    fn from(b: [f32; 3], a: [f32; 3]) -> Self {
        let n = 1.0 / a[0];
        Self { b0: b[0] * n, b1: b[1] * n, b2: b[2] * n, a1: a[1] * n, a2: a[2] * n, z1: 0.0, z2: 0.0 }
    }
    fn w(sr: f32, f: f32) -> (f32, f32) {
        let w = 2.0 * PI * (f.clamp(10.0, sr * 0.45)) / sr;
        (w.cos(), w.sin())
    }
    pub fn lowpass(sr: f32, f: f32, q: f32) -> Self {
        let (c, s) = Self::w(sr, f);
        let al = s / (2.0 * q);
        Self::from([(1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0], [1.0 + al, -2.0 * c, 1.0 - al])
    }
    pub fn highpass(sr: f32, f: f32, q: f32) -> Self {
        let (c, s) = Self::w(sr, f);
        let al = s / (2.0 * q);
        Self::from([(1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0], [1.0 + al, -2.0 * c, 1.0 - al])
    }
    pub fn bandpass(sr: f32, f: f32, q: f32) -> Self {
        let (c, s) = Self::w(sr, f);
        let al = s / (2.0 * q);
        Self::from([al, 0.0, -al], [1.0 + al, -2.0 * c, 1.0 - al])
    }
    pub fn peaking(sr: f32, f: f32, gain_db: f32, q: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let (c, s) = Self::w(sr, f);
        let al = s / (2.0 * q);
        Self::from([1.0 + al * a, -2.0 * c, 1.0 - al * a], [1.0 + al / a, -2.0 * c, 1.0 - al / a])
    }
    pub fn low_shelf(sr: f32, f: f32, gain_db: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let (c, s) = Self::w(sr, f);
        let sa = 2.0 * a.sqrt() * (s / 2.0 * std::f32::consts::SQRT_2);
        Self::from(
            [
                a * ((a + 1.0) - (a - 1.0) * c + sa),
                2.0 * a * ((a - 1.0) - (a + 1.0) * c),
                a * ((a + 1.0) - (a - 1.0) * c - sa),
            ],
            [(a + 1.0) + (a - 1.0) * c + sa, -2.0 * ((a - 1.0) + (a + 1.0) * c), (a + 1.0) + (a - 1.0) * c - sa],
        )
    }
    pub fn high_shelf(sr: f32, f: f32, gain_db: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let (c, s) = Self::w(sr, f);
        let sa = 2.0 * a.sqrt() * (s / 2.0 * std::f32::consts::SQRT_2);
        Self::from(
            [
                a * ((a + 1.0) + (a - 1.0) * c + sa),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
                a * ((a + 1.0) + (a - 1.0) * c - sa),
            ],
            [(a + 1.0) - (a - 1.0) * c + sa, 2.0 * ((a - 1.0) - (a + 1.0) * c), (a + 1.0) - (a - 1.0) * c - sa],
        )
    }
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// Seguidor de envelope de pico com ataque/release separados.
#[derive(Clone, Copy, Debug, Default)]
pub struct Envelope {
    att: f32,
    rel: f32,
    v: f32,
}

impl Envelope {
    pub fn new(sr: f32, attack_ms: f32, release_ms: f32) -> Self {
        let k = |ms: f32| (-1.0 / (ms.max(0.01) * 1e-3 * sr)).exp();
        Self { att: k(attack_ms), rel: k(release_ms), v: 0.0 }
    }
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let x = x.abs();
        let k = if x > self.v { self.att } else { self.rel };
        self.v = k * self.v + (1.0 - k) * x;
        self.v
    }
    pub fn value(&self) -> f32 {
        self.v
    }
}

/// Compressor estéreo linkado com joelho suave.
#[derive(Clone, Debug)]
pub struct Compressor {
    env: Envelope,
    pub threshold_db: f32,
    pub ratio: f32,
    pub knee_db: f32,
    pub makeup_db: f32,
    pub last_gr_db: f32,
}

impl Compressor {
    pub fn new(sr: f32, threshold_db: f32, ratio: f32, attack_ms: f32, release_ms: f32, makeup_db: f32) -> Self {
        Self { env: Envelope::new(sr, attack_ms, release_ms), threshold_db, ratio, knee_db: 6.0, makeup_db, last_gr_db: 0.0 }
    }
    #[inline]
    pub fn gain(&mut self, l: f32, r: f32) -> f32 {
        let e = lin_to_db(self.env.process(l.abs().max(r.abs())));
        let over = e - self.threshold_db;
        let slope = 1.0 - 1.0 / self.ratio.max(1.0);
        let k = self.knee_db;
        let gr = if over <= -k / 2.0 {
            0.0
        } else if over >= k / 2.0 {
            over * slope
        } else {
            slope * (over + k / 2.0).powi(2) / (2.0 * k)
        };
        self.last_gr_db = gr;
        db_to_lin(self.makeup_db - gr)
    }
}

/// Transient shaper: compara um envelope rápido com um lento e realça o ataque.
#[derive(Clone, Debug)]
pub struct TransientShaper {
    fast: Envelope,
    slow: Envelope,
    /// 0..1 — quanto realçar (até +9 dB no ataque) e quanto encurtar a cauda.
    pub amount: f32,
}

impl TransientShaper {
    pub fn new(sr: f32) -> Self {
        Self { fast: Envelope::new(sr, 0.5, 25.0), slow: Envelope::new(sr, 25.0, 200.0), amount: 0.5 }
    }
    #[inline]
    pub fn gain(&mut self, l: f32, r: f32) -> f32 {
        let x = l.abs().max(r.abs());
        let f = self.fast.process(x);
        let s = self.slow.process(x);
        let diff = (lin_to_db(f) - lin_to_db(s)).clamp(-12.0, 12.0);
        let attack = diff.max(0.0) / 12.0; // 0..1 durante o ataque
        let sustain = (-diff).max(0.0) / 12.0; // 0..1 na cauda
        db_to_lin(self.amount * (9.0 * attack - 3.0 * sustain))
    }
}

/// Crossover Linkwitz-Riley de 4ª ordem (graves + agudos somam plano).
#[derive(Clone, Debug)]
pub struct Crossover {
    lp: [Biquad; 2],
    hp: [Biquad; 2],
}

impl Crossover {
    pub fn new(sr: f32, f: f32) -> Self {
        let q = std::f32::consts::FRAC_1_SQRT_2;
        Self { lp: [Biquad::lowpass(sr, f, q); 2], hp: [Biquad::highpass(sr, f, q); 2] }
    }
    #[inline]
    pub fn split(&mut self, x: f32) -> (f32, f32) {
        let lo = self.lp[0].process(x);
        let lo = self.lp[1].process(lo);
        let hi = self.hp[0].process(x);
        let hi = self.hp[1].process(hi);
        (lo, hi)
    }
}

/// Saturação suave normalizada.
#[inline]
pub fn saturate(x: f32, drive: f32) -> f32 {
    if drive <= 1.0001 {
        return x;
    }
    (x * drive).tanh() / drive.tanh()
}

/// Reverb FDN de 8 linhas com amortecimento de agudos (plate/hall escuro).
pub struct Reverb {
    lines: Vec<Vec<f32>>,
    pos: Vec<usize>,
    gains: [f32; 8],
    damp: [Biquad; 8],
    pre: Vec<f32>,
    pre_pos: usize,
    in_hp: Biquad,
}

impl Reverb {
    pub fn new(sr: f32, decay_s: f32, predelay_ms: f32, damping_hz: f32) -> Self {
        // comprimentos primos entre si (ms), escalados pela taxa de amostragem
        const MS: [f32; 8] = [29.7, 37.1, 41.1, 43.7, 53.9, 59.3, 67.1, 73.3];
        let lens: Vec<usize> = MS.iter().map(|m| ((m * 1e-3 * sr) as usize).max(8)).collect();
        let mut gains = [0.0; 8];
        for (g, &l) in gains.iter_mut().zip(&lens) {
            *g = 10f32.powf(-3.0 * l as f32 / (decay_s.max(0.1) * sr));
        }
        let pre_len = ((predelay_ms * 1e-3 * sr) as usize).max(1);
        Self {
            lines: lens.iter().map(|&l| vec![0.0; l]).collect(),
            pos: vec![0; 8],
            gains,
            damp: [Biquad::lowpass(sr, damping_hz, 0.6); 8],
            pre: vec![0.0; pre_len],
            pre_pos: 0,
            in_hp: Biquad::highpass(sr, 150.0, 0.7),
        }
    }
    #[inline]
    pub fn process(&mut self, x: f32) -> (f32, f32) {
        let d = self.pre[self.pre_pos];
        self.pre[self.pre_pos] = self.in_hp.process(x);
        self.pre_pos = (self.pre_pos + 1) % self.pre.len();
        let mut o = [0.0f32; 8];
        for (i, oi) in o.iter_mut().enumerate() {
            *oi = self.lines[i][self.pos[i]];
        }
        // matriz de Householder: y = o - 2/N * soma
        let sum: f32 = o.iter().sum::<f32>() * (2.0 / 8.0);
        let mut l = 0.0;
        let mut r = 0.0;
        #[allow(clippy::needless_range_loop)]
        for i in 0..8 {
            let fb = (o[i] - sum) * self.gains[i];
            let fb = self.damp[i].process(fb);
            let len = self.lines[i].len();
            self.lines[i][self.pos[i]] = d * 0.35 + fb;
            self.pos[i] = (self.pos[i] + 1) % len;
            if i % 2 == 0 {
                l += o[i];
            } else {
                r += o[i];
            }
        }
        (l * 0.5, r * 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossover_sums_flat() {
        let sr = 48_000.0;
        let mut x = Crossover::new(sr, 4000.0);
        // energia de um seno de 1 kHz deve passar praticamente intacta
        let mut e_in = 0.0;
        let mut e_out = 0.0;
        for n in 0..48_000 {
            let s = (2.0 * PI * 1000.0 * n as f32 / sr).sin();
            let (lo, hi) = x.split(s);
            if n > 4800 {
                e_in += s * s;
                e_out += (lo + hi) * (lo + hi);
            }
        }
        let ratio_db = 10.0 * (e_out / e_in).log10();
        assert!(ratio_db.abs() < 0.5, "ratio {ratio_db}");
    }

    #[test]
    fn reverb_is_stable() {
        let mut r = Reverb::new(48_000.0, 3.0, 30.0, 5000.0);
        let mut peak: f32 = 0.0;
        for n in 0..48_000 * 8 {
            let x = if n == 0 { 1.0 } else { 0.0 };
            let (l, rr) = r.process(x);
            peak = peak.max(l.abs()).max(rr.abs());
            assert!(l.is_finite() && rr.is_finite());
        }
        assert!(peak < 2.0);
    }
}
