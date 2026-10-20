//! NDI experiment (debug builds on macOS and Windows only; not part of the
//! app).
//!
//! Answers: can SimpleWorship take frames of its own stream page fast enough,
//! with transparency, while the window is not on screen? Run with
//! `SW_NDI_SPIKE=/some/folder pnpm dev` (on Windows, in PowerShell:
//! `$env:SW_NDI_SPIKE="C:\ndi-test"; pnpm dev`). Results go to the log,
//! pictures to the folder.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sw_core::Command;
use sw_core::stream::{StreamLook, StreamSettings};
use sw_core::themes::starter_ids;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commands::engine::dispatch;
use crate::state::AppState;

#[cfg(target_os = "macos")]
use mac::{on_screen, snapshot, window_number};
#[cfg(windows)]
use win::{on_screen, snapshot};

const LABEL: &str = "ndi-spike";

/// One snapshot, measured.
#[derive(Debug)]
struct Shot {
    /// From asking until WebKit handed over the picture.
    snap_ms: f64,
    /// Turning it into pixels.
    convert_ms: f64,
    width: isize,
    height: isize,
    /// Lowest and highest alpha seen (0–1), on a grid over the picture.
    min_alpha: f64,
    max_alpha: f64,
    /// Changes when the picture changes.
    fingerprint: u64,
}

pub fn start_if_asked(app: &AppHandle) {
    let Some(folder) = std::env::var_os("SW_NDI_SPIKE").map(PathBuf::from) else {
        return;
    };
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(5));
        let _ = std::fs::create_dir_all(&folder);
        run(&app, &folder);
        log::info!("NDI SPIKE: done");
    });
}

fn run(app: &AppHandle, folder: &Path) {
    let state = app.state::<AppState>();
    let lower_third = StreamLook::Theme {
        theme_id: starter_ids::LOWER_THIRD.parse().expect("valid id"),
    };
    let settings = StreamSettings {
        other: lower_third,
        ..StreamSettings::default()
    };
    let _ = dispatch(app, &state, Command::SetStreamSettings { settings });
    show_text(
        app,
        "Amazing grace, how sweet the sound\nThat saved a wretch like me",
    );

    for case in ["visible", "hidden", "offscreen", "behind"] {
        if let Err(error) = run_case(app, folder, case) {
            log::warn!("NDI SPIKE [{case}]: failed: {error}");
        }
        if let Some(window) = app.get_webview_window(LABEL) {
            let _ = window.destroy();
        }
        std::thread::sleep(Duration::from_millis(800));
    }
}

fn show_text(app: &AppHandle, text: &str) {
    let state = app.state::<AppState>();
    let _ = dispatch(
        app,
        &state,
        Command::PreviewText {
            title: "NDI test".into(),
            text: text.into(),
        },
    );
    let _ = dispatch(app, &state, Command::GoLive);
}

fn run_case(app: &AppHandle, folder: &Path, case: &str) -> Result<(), String> {
    let url = WebviewUrl::App("index.html?view=stream&transparent".into());
    let mut builder = WebviewWindowBuilder::new(app, LABEL, url)
        .title("NDI spike")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .focused(false)
        .inner_size(960.0, 540.0);
    builder = match case {
        "hidden" => builder.visible(false),
        "offscreen" => builder.position(-6000.0, -6000.0),
        _ => builder.position(80.0, 80.0),
    };
    let window = builder.build().map_err(|e| e.to_string())?;
    if case == "offscreen" {
        // macOS may pull a new window back on screen; push it out again.
        let _ = window.set_position(LogicalPosition::new(-6000.0, -6000.0));
    }
    if case == "behind"
        && let Some(main) = app.get_webview_window("main")
    {
        let _ = main.set_focus();
    }
    let _ = window.set_size(LogicalSize::new(960.0, 540.0));
    std::thread::sleep(Duration::from_secs(4));

    log::info!(
        "NDI SPIKE [{case}]: window at {:?}, visible={:?}, on screen={}",
        window.outer_position().ok(),
        window.is_visible().ok(),
        on_screen(&window)
    );

    // 1. One picture, saved, with its transparency checked.
    let first = snapshot(&window, Some(&folder.join(format!("{case}.png"))))?;
    log::info!("NDI SPIKE [{case}]: first {first:?}");

    // 2. Back-to-back pictures: how many a second?
    let started = Instant::now();
    let mut times = Vec::new();
    let mut converts = Vec::new();
    for _ in 0..30 {
        let shot = snapshot(&window, None)?;
        times.push(shot.snap_ms);
        converts.push(shot.convert_ms);
    }
    converts.sort_by(f64::total_cmp);
    log::info!(
        "NDI SPIKE [{case}]: converting to pixels, median {:.1}ms",
        converts[15]
    );
    let total = started.elapsed().as_secs_f64();
    times.sort_by(f64::total_cmp);
    log::info!(
        "NDI SPIKE [{case}]: 30 shots in {:.2}s = {:.1}/s, median {:.1}ms, worst {:.1}ms",
        total,
        30.0 / total,
        times[15],
        times[29]
    );

    // 3. A new slide: how long until a picture shows it?
    let before = snapshot(&window, None)?.fingerprint;
    show_text(app, &format!("New slide for the {case} test\nSecond line"));
    let changed = Instant::now();
    let mut seen = None;
    while changed.elapsed() < Duration::from_secs(4) {
        if snapshot(&window, None)?.fingerprint != before {
            seen = Some(changed.elapsed());
            break;
        }
    }
    match seen {
        Some(after) => log::info!(
            "NDI SPIKE [{case}]: new slide seen after {}ms",
            after.as_millis()
        ),
        None => log::warn!("NDI SPIKE [{case}]: new slide NOT seen within 4s"),
    }
    snapshot(&window, Some(&folder.join(format!("{case}-changed.png"))))?;

    #[cfg(target_os = "macos")]
    capture_outside(&window, folder, case);
    Ok(())
}

/// The same window captured from outside, as OBS would.
#[cfg(target_os = "macos")]
fn capture_outside(window: &tauri::WebviewWindow, folder: &Path, case: &str) {
    let number = window_number(window);
    let outside = folder.join(format!("{case}-outside.png"));
    let started = Instant::now();
    let status = std::process::Command::new("screencapture")
        .args(["-x", "-o", &format!("-l{number}")])
        .arg(&outside)
        .status();
    log::info!(
        "NDI SPIKE [{case}]: outside capture of window {number}: {:?} in {}ms, file {}",
        status.map(|s| s.success()),
        started.elapsed().as_millis(),
        if outside.is_file() {
            "written"
        } else {
            "missing"
        }
    );
    if let Ok(bytes) = std::fs::read(&outside) {
        let data = objc2_foundation::NSData::with_bytes(&bytes);
        if let Some(rep) = objc2_app_kit::NSBitmapImageRep::imageRepWithData(&data) {
            let (min, max) = mac::alpha_range(&rep);
            log::info!(
                "NDI SPIKE [{case}]: outside capture {}x{}, alpha {min:.2}–{max:.2}",
                rep.pixelsWide(),
                rep.pixelsHigh()
            );
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    //! WebKit's own snapshot of the page.

    use std::hash::{DefaultHasher, Hash, Hasher};
    use std::path::Path;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use block2::RcBlock;
    use objc2::AnyThread;
    use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage, NSWindow};
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_foundation::{NSDictionary, NSError, NSNumber};
    use objc2_web_kit::{WKSnapshotConfiguration, WKWebView};

    use super::Shot;

    /// Whether macOS considers the window visible on a screen.
    pub fn on_screen(window: &tauri::WebviewWindow) -> bool {
        let Ok(pointer) = window.ns_window() else {
            return false;
        };
        let ns_window = unsafe { &*(pointer as *const NSWindow) };
        ns_window
            .occlusionState()
            .contains(objc2_app_kit::NSWindowOcclusionState::Visible)
    }

    pub fn window_number(window: &tauri::WebviewWindow) -> isize {
        let Ok(pointer) = window.ns_window() else {
            return 0;
        };
        let ns_window = unsafe { &*(pointer as *const NSWindow) };
        ns_window.windowNumber()
    }

    /// Takes a picture of the window's page with WebKit, waiting for it.
    pub fn snapshot(window: &tauri::WebviewWindow, save: Option<&Path>) -> Result<Shot, String> {
        let (sender, receiver) = mpsc::channel::<Result<Shot, String>>();
        let save = save.map(Path::to_path_buf);
        window
            .with_webview(move |webview| {
                let web_view = unsafe { &*(webview.inner() as *const WKWebView) };
                let mtm =
                    objc2::MainThreadMarker::new().expect("with_webview runs on the main thread");
                let config = unsafe { WKSnapshotConfiguration::new(mtm) };
                unsafe {
                    config.setRect(CGRect::new(
                        CGPoint::new(0.0, 0.0),
                        CGSize::new(960.0, 540.0),
                    ));
                    // In points: 960 points are 1920 pixels on a Retina screen.
                    config.setSnapshotWidth(Some(&NSNumber::new_f64(960.0)));
                    config.setAfterScreenUpdates(true);
                }
                let started = Instant::now();
                let block = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
                    let snap_ms = started.elapsed().as_secs_f64() * 1000.0;
                    let result = if image.is_null() {
                        let message = if error.is_null() {
                            "no image".to_owned()
                        } else {
                            unsafe { &*error }.localizedDescription().to_string()
                        };
                        Err(message)
                    } else {
                        Ok(measure(unsafe { &*image }, snap_ms, save.as_deref()))
                    };
                    let _ = sender.send(result);
                });
                unsafe {
                    web_view.takeSnapshotWithConfiguration_completionHandler(Some(&config), &block)
                };
            })
            .map_err(|e| e.to_string())?;
        receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "no snapshot within 5s".to_owned())?
    }

    fn measure(image: &NSImage, snap_ms: f64, save: Option<&Path>) -> Shot {
        let started = Instant::now();
        // Straight to a bitmap, without going through a file format.
        let cg_image =
            unsafe { image.CGImageForProposedRect_context_hints(std::ptr::null_mut(), None, None) };
        let rep =
            cg_image.map(|cg| NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg));
        let Some(rep) = rep else {
            return Shot {
                snap_ms,
                convert_ms: 0.0,
                width: 0,
                height: 0,
                min_alpha: -1.0,
                max_alpha: -1.0,
                fingerprint: 0,
            };
        };
        let width = rep.pixelsWide();
        let height = rep.pixelsHigh();
        let bytes = rep.bytesPerRow() * height;
        let pixels = unsafe { std::slice::from_raw_parts(rep.bitmapData(), bytes.max(0) as usize) };
        let mut hasher = DefaultHasher::new();
        pixels.hash(&mut hasher);
        let convert_ms = started.elapsed().as_secs_f64() * 1000.0;
        let (min_alpha, max_alpha) = alpha_range(&rep);
        if let Some(path) = save {
            let png = unsafe {
                rep.representationUsingType_properties(
                    NSBitmapImageFileType::PNG,
                    &NSDictionary::new(),
                )
            };
            if let Some(png) = png {
                let _ = std::fs::write(path, png.to_vec());
            }
        }
        Shot {
            snap_ms,
            convert_ms,
            width,
            height,
            min_alpha,
            max_alpha,
            fingerprint: hasher.finish(),
        }
    }

    /// The lowest and highest alpha on a grid over the picture.
    pub fn alpha_range(rep: &NSBitmapImageRep) -> (f64, f64) {
        let (width, height) = (rep.pixelsWide(), rep.pixelsHigh());
        let (mut min, mut max) = (f64::MAX, f64::MIN);
        for y in (0..height).step_by(12) {
            for x in (0..width).step_by(12) {
                if let Some(color) = rep.colorAtX_y(x, y) {
                    let alpha = color.alphaComponent();
                    min = min.min(alpha);
                    max = max.max(alpha);
                }
            }
        }
        (min, max)
    }
}

#[cfg(windows)]
mod win {
    //! WebView2's own snapshot of the page (`CapturePreview`, as PNG).

    use std::hash::{DefaultHasher, Hash, Hasher};
    use std::path::Path;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use webview2_com::CapturePreviewCompletedHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG;
    use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
    use windows::Win32::System::Com::{IStream, STREAM_SEEK_SET};

    use super::Shot;

    /// Windows does not say; the log shows `is_visible` instead.
    pub fn on_screen(_window: &tauri::WebviewWindow) -> bool {
        false
    }

    pub fn snapshot(window: &tauri::WebviewWindow, save: Option<&Path>) -> Result<Shot, String> {
        let (sender, receiver) = mpsc::channel::<Result<(Vec<u8>, f64), String>>();
        window
            .with_webview(move |webview| unsafe {
                let started = Instant::now();
                let result = (|| {
                    let core = webview
                        .controller()
                        .CoreWebView2()
                        .map_err(|e| e.to_string())?;
                    let stream: IStream = CreateStreamOnHGlobal(Default::default(), true)
                        .map_err(|e| e.to_string())?;
                    let reader = stream.clone();
                    let sender = sender.clone();
                    let handler = CapturePreviewCompletedHandler::create(Box::new(move |result| {
                        let snap_ms = started.elapsed().as_secs_f64() * 1000.0;
                        let bytes = result
                            .map_err(|e| e.to_string())
                            .and_then(|()| read_all(&reader))
                            .map(|bytes| (bytes, snap_ms));
                        let _ = sender.send(bytes);
                        Ok(())
                    }));
                    core.CapturePreview(
                        COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG,
                        &stream,
                        &handler,
                    )
                    .map_err(|e| e.to_string())
                })();
                if let Err(error) = result {
                    let _ = sender.send(Err(error));
                }
            })
            .map_err(|e| e.to_string())?;
        let (png_bytes, snap_ms) = receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "no snapshot within 5s".to_owned())??;

        // Decoding the PNG is part of the cost on Windows.
        let started = Instant::now();
        let decoder = png::Decoder::new(png_bytes.as_slice());
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let mut pixels = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut pixels).map_err(|e| e.to_string())?;
        let convert_ms = started.elapsed().as_secs_f64() * 1000.0;
        let pixels = &pixels[..info.buffer_size()];
        let (min_alpha, max_alpha) = if info.color_type == png::ColorType::Rgba {
            let (lo, hi) = pixels
                .chunks(4)
                .map(|px| px[3])
                .fold((255u8, 0u8), |(lo, hi), a| (lo.min(a), hi.max(a)));
            (f64::from(lo) / 255.0, f64::from(hi) / 255.0)
        } else {
            (1.0, 1.0)
        };
        let mut hasher = DefaultHasher::new();
        pixels.hash(&mut hasher);
        if let Some(path) = save {
            let _ = std::fs::write(path, &png_bytes);
        }
        Ok(Shot {
            snap_ms,
            convert_ms,
            width: info.width as isize,
            height: info.height as isize,
            min_alpha,
            max_alpha,
            fingerprint: hasher.finish(),
        })
    }

    fn read_all(stream: &IStream) -> Result<Vec<u8>, String> {
        unsafe {
            stream
                .Seek(0, STREAM_SEEK_SET, None)
                .map_err(|e| e.to_string())?;
            let mut bytes = Vec::new();
            let mut chunk = vec![0u8; 1 << 20];
            loop {
                let mut read = 0u32;
                let _ = stream.Read(
                    chunk.as_mut_ptr().cast(),
                    chunk.len() as u32,
                    Some(&mut read),
                );
                if read == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..read as usize]);
            }
            Ok(bytes)
        }
    }
}
