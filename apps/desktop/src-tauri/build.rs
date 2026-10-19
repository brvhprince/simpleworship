//! Build script: generates Tauri context (config, icons, permissions).

use std::fs;
use std::path::Path;

/// The phone remote's built web app, bundled into the installers (see
/// `bundle.resources` in `tauri.conf.json`). It is built by `pnpm dev` and
/// `pnpm build`.
const REMOTE_DIST: &str = "../../remote/dist";

fn main() {
    ensure_remote_dist();
    tauri_build::build();
}

/// Tauri refuses to build when a bundled folder is missing. So that plain
/// `cargo` commands (tests, clippy, the editor) work before the remote has
/// been built, puts a page there that says how to build it.
fn ensure_remote_dist() {
    let dist = Path::new(REMOTE_DIST);
    if dist.join("index.html").exists() {
        return;
    }
    fs::create_dir_all(dist).expect("create the remote's dist folder");
    fs::write(
        dist.join("index.html"),
        "<!doctype html><meta charset=\"utf-8\"><title>SimpleWorship</title>\
         <p>The phone remote was not built. Run \
         <code>pnpm --filter @simpleworship/remote build</code>.</p>\n",
    )
    .expect("write the remote's placeholder page");
}
