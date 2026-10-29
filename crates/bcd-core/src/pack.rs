//! Packs de timbres do BC Drums.
//!
//! Um pack é uma pasta dentro de `Packs/`:
//!
//! ```text
//! Packs/
//!   Worship/
//!     pack.toml          (opcional — nome, ajustes da cadeia do estilo)
//!     bumbo/  01.wav 02.wav ...   (da batida mais fraca para a mais forte)
//!     caixa/  01.wav ...
//!     tom_agudo/ ...
//!     tom_grave/ ...
//! ```
//!
//! Nomes de pasta aceitos: `kick`/`bumbo`, `snare`/`caixa`,
//! `tom_hi`/`tom_agudo`/`rack`, `tom_lo`/`tom_grave`/`floor`.
//! Os arquivos de cada pasta são ordenados pelo nome (ordem natural), do mais
//! fraco para o mais forte. WAV 16/24/32 bits ou float, mono ou estéreo, em
//! qualquer taxa de amostragem (o pack é convertido para a taxa do projeto).

use serde::Deserialize;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Piece {
    Kick,
    Snare,
    TomHi,
    TomLo,
}

impl Piece {
    pub const ALL: [Piece; 4] = [Piece::Kick, Piece::Snare, Piece::TomHi, Piece::TomLo];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn key(self) -> &'static str {
        match self {
            Piece::Kick => "kick",
            Piece::Snare => "snare",
            Piece::TomHi => "tom_hi",
            Piece::TomLo => "tom_lo",
        }
    }
    pub fn label_pt(self) -> &'static str {
        match self {
            Piece::Kick => "Bumbo",
            Piece::Snare => "Caixa",
            Piece::TomHi => "Tom agudo",
            Piece::TomLo => "Tom grave",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            Piece::Kick => "K",
            Piece::Snare => "S",
            Piece::TomHi | Piece::TomLo => "T",
        }
    }
    fn folder_aliases(self) -> &'static [&'static str] {
        match self {
            Piece::Kick => &["kick", "bumbo", "bombo"],
            Piece::Snare => &["snare", "caixa"],
            Piece::TomHi => &["tom_hi", "tom_agudo", "rack", "rack_tom", "tom1"],
            Piece::TomLo => &["tom_lo", "tom_grave", "floor", "floor_tom", "surdo", "tom2"],
        }
    }
}

/// Ajustes da cadeia de processamento de um estilo (todos opcionais no pack.toml).
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ChainSettings {
    pub low_shelf_hz: f32,
    pub low_shelf_db: f32,
    pub mid_hz: f32,
    pub mid_db: f32,
    pub mid_q: f32,
    pub high_shelf_hz: f32,
    pub high_shelf_db: f32,
    pub comp_threshold_db: f32,
    pub comp_ratio: f32,
    pub comp_attack_ms: f32,
    pub comp_release_ms: f32,
    /// 0..1 — quanto da compressão paralela pesada é somada
    pub parallel: f32,
    pub reverb_decay_s: f32,
    pub reverb_predelay_ms: f32,
    pub reverb_damping_hz: f32,
    /// nível do retorno do reverb (0..1)
    pub reverb_return: f32,
    /// quanto do áudio original vai para o reverb (0..1)
    pub reverb_send_original: f32,
    pub drive: f32,
    /// nível do áudio original (já tratado) na saída processada
    pub original_db: f32,
    /// nível dos samples do pack na saída processada
    pub samples_db: f32,
    pub output_db: f32,
}

impl Default for ChainSettings {
    fn default() -> Self {
        Self {
            low_shelf_hz: 70.0,
            low_shelf_db: 2.0,
            mid_hz: 420.0,
            mid_db: -3.0,
            mid_q: 1.0,
            high_shelf_hz: 10_000.0,
            high_shelf_db: 1.0,
            comp_threshold_db: -18.0,
            comp_ratio: 3.0,
            comp_attack_ms: 10.0,
            comp_release_ms: 150.0,
            parallel: 0.35,
            reverb_decay_s: 2.2,
            reverb_predelay_ms: 25.0,
            reverb_damping_hz: 6000.0,
            reverb_return: 0.25,
            reverb_send_original: 0.15,
            drive: 1.3,
            original_db: -4.0,
            samples_db: 0.0,
            output_db: 0.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct PieceSettings {
    pub gain_db: f32,
    /// quanto deste sample vai para o reverb (0..1)
    pub reverb_send: f32,
}

impl Default for PieceSettings {
    fn default() -> Self {
        Self { gain_db: 0.0, reverb_send: 0.3 }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct PieceTable {
    pub kick: Option<PieceSettings>,
    pub snare: Option<PieceSettings>,
    pub tom_hi: Option<PieceSettings>,
    pub tom_lo: Option<PieceSettings>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct PackManifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub chain: ChainSettings,
    pub pieces: PieceTable,
}

/// Uma camada de dinâmica: áudio estéreo intercalado na taxa do motor.
#[derive(Clone, Debug)]
pub struct Layer {
    pub frames: Vec<[f32; 2]>,
    /// índice do ataque dentro do arquivo (silêncio inicial é pulado)
    pub onset: usize,
    pub source: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct PieceKit {
    pub layers: Vec<Layer>,
    pub settings: PieceSettings,
}

#[derive(Clone, Debug)]
pub struct Pack {
    pub name: String,
    pub version: String,
    pub dir: PathBuf,
    pub chain: ChainSettings,
    pub pieces: [PieceKit; 4],
    pub warnings: Vec<String>,
}

impl Pack {
    pub fn piece(&self, p: Piece) -> &PieceKit {
        &self.pieces[p.index()]
    }
    pub fn sample_count(&self) -> usize {
        self.pieces.iter().map(|k| k.layers.len()).sum()
    }
}

#[derive(Debug)]
pub enum PackError {
    Io(PathBuf, std::io::Error),
    Manifest(PathBuf, String),
    Wav(PathBuf, String),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackError::Io(p, e) => write!(f, "{}: {e}", p.display()),
            PackError::Manifest(p, e) => write!(f, "pack.toml inválido em {}: {e}", p.display()),
            PackError::Wav(p, e) => write!(f, "WAV inválido {}: {e}", p.display()),
        }
    }
}

impl std::error::Error for PackError {}

/// Pasta padrão dos packs: `~/Music/BC Drums/Packs` (ou `BCDRUMS_PACKS`).
pub fn default_packs_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("BCDRUMS_PACKS") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join("Music").join("BC Drums").join("Packs")
}

fn natural_key(s: &str) -> (Vec<u64>, String) {
    let mut nums = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            nums.push(cur.parse().unwrap_or(0));
            cur.clear();
        }
    }
    if !cur.is_empty() {
        nums.push(cur.parse().unwrap_or(0));
    }
    (nums, s.to_lowercase())
}

fn is_audio(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("wav")).unwrap_or(false)
        && !p.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("._")).unwrap_or(false)
}

/// Lê um WAV como estéreo f32 + taxa de amostragem.
pub fn read_wav(path: &Path) -> Result<(Vec<[f32; 2]>, u32), PackError> {
    let err = |e: hound::Error| PackError::Wav(path.to_path_buf(), e.to_string());
    let mut r = hound::WavReader::open(path).map_err(err)?;
    let spec = r.spec();
    let ch = spec.channels.max(1) as usize;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().collect::<Result<_, _>>().map_err(err)?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample.saturating_sub(1))) as f32;
            r.samples::<i32>().map(|s| s.map(|v| v as f32 * scale)).collect::<Result<_, _>>().map_err(err)?
        }
    };
    let frames = samples
        .chunks(ch)
        .map(|c| if ch == 1 { [c[0], c[0]] } else { [c[0], c.get(1).copied().unwrap_or(c[0])] })
        .collect();
    Ok((frames, spec.sample_rate))
}

/// Reamostragem offline com sinc janelado (qualidade boa para one-shots).
pub fn resample(input: &[[f32; 2]], from: u32, to: u32) -> Vec<[f32; 2]> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = to as f64 / from as f64;
    let out_len = ((input.len() as f64) * ratio).ceil() as usize;
    let cutoff = ratio.min(1.0) * 0.95;
    const HALF: i64 = 24;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let t = i as f64 / ratio;
        let center = t.floor() as i64;
        let mut acc = [0.0f64; 2];
        let mut wsum = 0.0f64;
        for k in (center - HALF + 1)..=(center + HALF) {
            if k < 0 || k as usize >= input.len() {
                continue;
            }
            let x = t - k as f64;
            let sinc = if x.abs() < 1e-9 { 1.0 } else { (std::f64::consts::PI * x * cutoff).sin() / (std::f64::consts::PI * x * cutoff) };
            let win = 0.5 + 0.5 * (std::f64::consts::PI * x / HALF as f64).cos();
            let w = sinc * win * cutoff;
            acc[0] += input[k as usize][0] as f64 * w;
            acc[1] += input[k as usize][1] as f64 * w;
            wsum += w;
        }
        let n = if wsum.abs() > 1e-9 { cutoff / wsum } else { 1.0 };
        out.push([(acc[0] * n / cutoff) as f32, (acc[1] * n / cutoff) as f32]);
    }
    out
}

fn find_onset(frames: &[[f32; 2]]) -> usize {
    let peak = frames.iter().map(|f| f[0].abs().max(f[1].abs())).fold(0.0, f32::max);
    if peak <= 0.0 {
        return 0;
    }
    let idx = frames.iter().position(|f| f[0].abs().max(f[1].abs()) > peak * 0.08).unwrap_or(0);
    idx.saturating_sub(16)
}

fn load_piece_dir(dir: &Path, sr: u32) -> Result<Vec<Layer>, PackError> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| PackError::Io(dir.to_path_buf(), e))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| is_audio(p))
        .collect();
    files.sort_by_key(|p| natural_key(&p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    let mut layers = Vec::new();
    for f in files {
        let (frames, rate) = read_wav(&f)?;
        let frames = resample(&frames, rate, sr);
        let onset = find_onset(&frames);
        layers.push(Layer { frames, onset, source: f });
    }
    Ok(layers)
}

/// Carrega um pack de uma pasta, convertendo os samples para `sr`.
pub fn load_pack(dir: &Path, sr: u32) -> Result<Pack, PackError> {
    let manifest_path = dir.join("pack.toml");
    let manifest: PackManifest = if manifest_path.exists() {
        let text = std::fs::read_to_string(&manifest_path).map_err(|e| PackError::Io(manifest_path.clone(), e))?;
        toml::from_str(&text).map_err(|e| PackError::Manifest(manifest_path.clone(), e.to_string()))?
    } else {
        PackManifest::default()
    };
    let folder_name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Pack".into());
    let mut pieces: [PieceKit; 4] = Default::default();
    let mut warnings = Vec::new();
    let entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| PackError::Io(dir.to_path_buf(), e))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    for piece in Piece::ALL {
        let settings = match piece {
            Piece::Kick => manifest.pieces.kick.clone(),
            Piece::Snare => manifest.pieces.snare.clone(),
            Piece::TomHi => manifest.pieces.tom_hi.clone(),
            Piece::TomLo => manifest.pieces.tom_lo.clone(),
        }
        .unwrap_or_default();
        let sub = entries.iter().find(|p| {
            let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase().replace([' ', '-'], "_")).unwrap_or_default();
            piece.folder_aliases().iter().any(|a| *a == n)
        });
        let layers = match sub {
            Some(d) => load_piece_dir(d, sr)?,
            None => Vec::new(),
        };
        if layers.is_empty() {
            warnings.push(format!("{}: sem samples (o original segue sem reforço)", piece.label_pt()));
        }
        pieces[piece.index()] = PieceKit { layers, settings };
    }
    Ok(Pack {
        name: manifest.name.unwrap_or(folder_name),
        version: manifest.version.unwrap_or_else(|| "1.0".into()),
        dir: dir.to_path_buf(),
        chain: manifest.chain,
        pieces,
        warnings,
    })
}

/// Resultado de uma varredura da pasta de packs.
#[derive(Clone, Debug, Default)]
pub struct Library {
    pub packs: Vec<Arc<Pack>>,
    pub errors: Vec<String>,
    pub dir: PathBuf,
}

impl Library {
    pub fn names(&self) -> Vec<String> {
        self.packs.iter().map(|p| p.name.clone()).collect()
    }
}

/// Varre `dir` e carrega todos os packs (cada subpasta é um estilo).
/// Erros num pack não impedem os outros de carregar.
pub fn scan_library(dir: &Path, sr: u32) -> Library {
    let mut lib = Library { dir: dir.to_path_buf(), ..Default::default() };
    let mut dirs: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_dir()).collect(),
        Err(e) => {
            lib.errors.push(format!("Pasta de packs não encontrada ({}): {e}", dir.display()));
            return lib;
        }
    };
    dirs.sort_by_key(|p| natural_key(&p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    for d in dirs {
        match load_pack(&d, sr) {
            Ok(p) if p.sample_count() > 0 => lib.packs.push(Arc::new(p)),
            Ok(p) => lib.errors.push(format!("{}: nenhuma pasta de samples encontrada", p.name)),
            Err(e) => lib.errors.push(e.to_string()),
        }
    }
    lib
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order() {
        let mut v = vec!["10.wav", "2.wav", "1.wav"];
        v.sort_by_key(|s| natural_key(s));
        assert_eq!(v, vec!["1.wav", "2.wav", "10.wav"]);
    }

    #[test]
    fn resample_keeps_length_ratio() {
        let x: Vec<[f32; 2]> = (0..4800).map(|i| [(i as f32 * 0.01).sin(); 2]).collect();
        let y = resample(&x, 48_000, 44_100);
        assert!((y.len() as i64 - 4410).abs() <= 1);
        let mid = y[2205][0];
        let expect = ((2205.0 / 44_100.0 * 48_000.0) * 0.01f32).sin();
        assert!((mid - expect).abs() < 0.02, "{mid} vs {expect}");
    }

    #[test]
    fn manifest_defaults() {
        let m: PackManifest = toml::from_str("name = \"Teste\"\n[chain]\nmid_db = -5.0\n").unwrap_or_default();
        assert_eq!(m.name.as_deref(), Some("Teste"));
        assert_eq!(m.chain.mid_db, -5.0);
        assert_eq!(m.chain.comp_ratio, 3.0);
    }
}
