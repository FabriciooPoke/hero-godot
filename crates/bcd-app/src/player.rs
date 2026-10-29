//! Player simples para ouvir o resultado antes de baixar (A/B com o original).

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct Shared {
    pub original: Mutex<Arc<Vec<[f32; 2]>>>,
    pub processed: Mutex<Arc<Vec<[f32; 2]>>>,
    /// posição em amostras do arquivo
    pub pos: AtomicU64,
    pub playing: AtomicBool,
    /// true = BC Drums, false = original
    pub use_processed: AtomicBool,
    pub file_sr: AtomicU64,
}

pub struct Player {
    pub shared: Arc<Shared>,
    _stream: Option<cpal::Stream>,
    pub error: Option<String>,
}

impl Player {
    pub fn new() -> Self {
        let shared = Arc::new(Shared {
            original: Mutex::new(Arc::new(Vec::new())),
            processed: Mutex::new(Arc::new(Vec::new())),
            pos: AtomicU64::new(0),
            playing: AtomicBool::new(false),
            use_processed: AtomicBool::new(true),
            file_sr: AtomicU64::new(44_100),
        });
        match Self::open(shared.clone()) {
            Ok(stream) => Self { shared, _stream: Some(stream), error: None },
            Err(e) => Self { shared, _stream: None, error: Some(e) },
        }
    }

    fn open(shared: Arc<Shared>) -> Result<cpal::Stream, String> {
        let host = cpal::default_host();
        let dev = host.default_output_device().ok_or("sem saída de áudio")?;
        let cfg = dev.default_output_config().map_err(|e| e.to_string())?;
        let channels = cfg.channels() as usize;
        let out_sr = cfg.sample_rate().0 as f64;
        let config: cpal::StreamConfig = cfg.clone().into();
        let mut frac = 0.0f64;
        let stream = dev
            .build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    let playing = shared.playing.load(Ordering::Relaxed);
                    let buf = if shared.use_processed.load(Ordering::Relaxed) {
                        shared.processed.try_lock().ok().map(|b| b.clone())
                    } else {
                        shared.original.try_lock().ok().map(|b| b.clone())
                    };
                    let step = shared.file_sr.load(Ordering::Relaxed) as f64 / out_sr;
                    let mut pos = shared.pos.load(Ordering::Relaxed) as f64 + frac;
                    for frame in data.chunks_mut(channels) {
                        let s = match (&buf, playing) {
                            (Some(b), true) if (pos as usize) + 1 < b.len() => {
                                let i = pos as usize;
                                let t = (pos - i as f64) as f32;
                                [b[i][0] + (b[i + 1][0] - b[i][0]) * t, b[i][1] + (b[i + 1][1] - b[i][1]) * t]
                            }
                            _ => [0.0, 0.0],
                        };
                        for (c, out) in frame.iter_mut().enumerate() {
                            *out = if c % 2 == 0 { s[0] } else { s[1] };
                        }
                        if playing {
                            pos += step;
                        }
                    }
                    if let Some(b) = &buf {
                        if playing && pos as usize + 1 >= b.len() {
                            shared.playing.store(false, Ordering::Relaxed);
                            pos = 0.0;
                        }
                    }
                    frac = pos.fract();
                    shared.pos.store(pos as u64, Ordering::Relaxed);
                },
                |e| eprintln!("áudio: {e}"),
                None,
            )
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(stream)
    }

    pub fn load(&self, original: Vec<[f32; 2]>, processed: Vec<[f32; 2]>, sr: u32) {
        self.shared.playing.store(false, Ordering::Relaxed);
        if let Ok(mut o) = self.shared.original.lock() {
            *o = Arc::new(original);
        }
        if let Ok(mut p) = self.shared.processed.lock() {
            *p = Arc::new(processed);
        }
        self.shared.file_sr.store(sr as u64, Ordering::Relaxed);
        self.shared.pos.store(0, Ordering::Relaxed);
    }

    pub fn toggle(&self) {
        let p = self.shared.playing.load(Ordering::Relaxed);
        self.shared.playing.store(!p, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        self.shared.playing.store(false, Ordering::Relaxed);
        self.shared.pos.store(0, Ordering::Relaxed);
    }

    pub fn seek_seconds(&self, s: f32) {
        let sr = self.shared.file_sr.load(Ordering::Relaxed) as f32;
        self.shared.pos.store((s.max(0.0) * sr) as u64, Ordering::Relaxed);
    }

    pub fn position_seconds(&self) -> f32 {
        self.shared.pos.load(Ordering::Relaxed) as f32 / self.shared.file_sr.load(Ordering::Relaxed).max(1) as f32
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    pub fn processed_on(&self) -> bool {
        self.shared.use_processed.load(Ordering::Relaxed)
    }

    pub fn set_processed(&self, on: bool) {
        self.shared.use_processed.store(on, Ordering::Relaxed);
    }
}
