//! `bcdrums` — utilitário de linha de comando do BC Drums.
//!
//! bcdrums analisar entrada.wav            lista as batidas detectadas
//! bcdrums processar entrada.wav saida.wav [--packs PASTA] [--estilo NOME]
//!          [--transient 0-100] [--mix 0-100]
//! bcdrums packs [--packs PASTA]           lista os packs encontrados

use bcd_core::*;
use std::path::PathBuf;
use std::sync::Arc;

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn write_wav(path: &str, data: &[[f32; 2]], sr: u32) -> Result<(), String> {
    let spec = hound::WavSpec { channels: 2, sample_rate: sr, bits_per_sample: 24, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for f in data {
        for c in f {
            let v = (c.clamp(-1.0, 1.0) * 8_388_607.0) as i32;
            w.write_sample(v).map_err(|e| e.to_string())?;
        }
    }
    w.finalize().map_err(|e| e.to_string())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("");
    let packs_dir = arg(&args, "--packs").map(PathBuf::from).unwrap_or_else(default_packs_dir);
    match cmd {
        "analisar" | "analyze" => {
            let input = args.get(2).ok_or("informe o arquivo")?;
            let (frames, sr) = pack::read_wav(input.as_ref()).map_err(|e| e.to_string())?;
            let mut d = Detector::new(sr as f32);
            println!("tempo_s,peca,velocity,low,low_mid,mid,high,low_sustain,lm_sustain,peak_db,pitch");
            let mut onsets = 0;
            for f in &frames {
                let before = d.last_features.peak_db;
                let hit = d.process(0.5 * (f[0] + f[1]));
                let ft = d.last_features;
                if let Some(h) = hit {
                    let t = ft.total();
                    println!(
                        "{:.4},{},{:.2},{:.3},{:.3},{:.3},{:.3},{:.2},{:.2},{:.1},{:.0}",
                        h.time as f32 / sr as f32, h.piece.key(), h.velocity,
                        ft.low / t, ft.low_mid / t, ft.mid / t, ft.high / t,
                        ft.low_tail / ft.low.max(1e-12), ft.low_mid_tail / ft.low_mid.max(1e-12), ft.peak_db, ft.pitch_hz
                    );
                } else if ft.peak_db != before {
                    onsets += 1;
                    let t = ft.total();
                    println!(",ignorado,,{:.3},{:.3},{:.3},{:.3},{:.2},{:.2},{:.1}",
                        ft.low / t, ft.low_mid / t, ft.mid / t, ft.high / t,
                        ft.low_tail / ft.low.max(1e-12), ft.low_mid_tail / ft.low_mid.max(1e-12), ft.peak_db);
                }
            }
            eprintln!("({onsets} ataques ignorados)");
            Ok(())
        }
        "analisar-musica" => {
            let input = args.get(2).ok_or("informe o arquivo")?;
            let (frames, sr) = pack::read_wav(input.as_ref()).map_err(|e| e.to_string())?;
            let mono: Vec<f32> = frames.iter().map(|f| 0.5 * (f[0] + f[1])).collect();
            println!("tempo_s,peca,velocity");
            for h in analyze_song(&mono, sr as f32) {
                println!("{:.4},{},{:.2}", h.time as f32 / sr as f32, h.piece.key(), h.velocity);
            }
            Ok(())
        }
        "packs" => {
            let lib = scan_library(&packs_dir, 48_000);
            println!("Pasta: {}", packs_dir.display());
            for p in &lib.packs {
                println!("- {} (v{}) — {} samples", p.name, p.version, p.sample_count());
                for piece in Piece::ALL {
                    println!("    {:10} {} camadas", piece.label_pt(), p.piece(piece).layers.len());
                }
                for w in &p.warnings {
                    println!("    aviso: {w}");
                }
            }
            for e in &lib.errors {
                println!("erro: {e}");
            }
            Ok(())
        }
        "processar" | "process" => {
            let input = args.get(2).ok_or("informe a entrada")?;
            let output = args.get(3).ok_or("informe a saída")?;
            let (frames, sr) = pack::read_wav(input.as_ref()).map_err(|e| e.to_string())?;
            let lib = Arc::new(scan_library(&packs_dir, sr));
            for e in &lib.errors {
                eprintln!("aviso: {e}");
            }
            let mut eng = Engine::new(sr as f32, lib.clone());
            if let Some(name) = arg(&args, "--estilo") {
                eng.params.style = lib.packs.iter().position(|p| p.name.eq_ignore_ascii_case(&name)).ok_or(format!("estilo '{name}' não encontrado"))?;
            }
            if let Some(v) = arg(&args, "--transient") {
                eng.params.transient = v.parse::<f32>().map_err(|e| e.to_string())? / 100.0;
            }
            if let Some(v) = arg(&args, "--mix") {
                eng.params.mix = v.parse::<f32>().map_err(|e| e.to_string())? / 100.0;
            }
            let out = process_offline(&mut eng, &frames);
            let peak = out.iter().map(|f| f[0].abs().max(f[1].abs())).fold(0.0, f32::max);
            let norm = if peak > 0.98 { 0.98 / peak } else { 1.0 };
            let out: Vec<[f32; 2]> = out.iter().map(|f| [f[0] * norm, f[1] * norm]).collect();
            write_wav(output, &out, sr)?;
            let hits = eng.meters().recent_hits().len();
            eprintln!("ok: {} → {} (estilo {}, {} batidas recentes, latência {} amostras)", input, output,
                eng.current_pack().map(|p| p.name.as_str()).unwrap_or("sem pack"), hits, eng.latency());
            Ok(())
        }
        _ => Err("uso: bcdrums analisar|processar|packs ... (veja o código)".into()),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("erro: {e}");
        std::process::exit(1);
    }
}
