//! SimpleWorship desktop entry point. All the setup lives in the library
//! crate (`lib.rs`) so it can be shared with Tauri's mobile targets.
//!
//! The same executable also runs Listen's speech to text helper, when
//! started with `--listen-helper <model>` (see `sw_listen::worker`): a
//! crash in Whisper then ends only the helper, never the projector.

// Hide the extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let mut args = std::env::args_os().skip(1);
    if args
        .next()
        .is_some_and(|flag| flag == sw_listen::worker::HELPER_FLAG)
    {
        let Some(model) = args.next() else {
            eprintln!("usage: {} <model file>", sw_listen::worker::HELPER_FLAG);
            std::process::exit(2);
        };
        std::process::exit(sw_listen::worker::serve(std::path::Path::new(&model)));
    }
    simpleworship_desktop_lib::run();
}
