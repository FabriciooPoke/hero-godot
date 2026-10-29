//! BC Drums — motor de áudio.
//!
//! LR da bateria crua entra; LR pronta sai. Veja [`engine::Engine`].

pub mod detect;
pub mod dsp;
pub mod engine;
pub mod mixbus;
pub mod pack;

pub use detect::{analyze_song, Detector, Hit};
pub use engine::{process_offline, Engine, Meters, Params, Prepared};
pub use pack::{default_packs_dir, load_pack, scan_library, Library, Pack, Piece};
