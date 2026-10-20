//! The NDI library, loaded at run time, and sending pictures through it.
//!
//! SimpleWorship does not ship the NDI library: the user installs the free
//! NDI Tools or NDI Runtime from ndi.video, and it is loaded from where
//! those put it. Only the few functions SimpleWorship needs are looked up.
//! The type layouts below follow NDI's own headers (`Processing.NDI.*.h`,
//! MIT-licensed), NDI SDK 5 and 6.

#![allow(unsafe_code)]

use std::ffi::{CString, c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Instant;

use libloading::Library;

use crate::compositor::Compositor;
use crate::frame::Frame;
use crate::{FRAME_RATE, HEIGHT, WIDTH};

/// Why NDI is not available.
#[derive(Debug, thiserror::Error)]
pub enum NdiError {
    /// The NDI library is not installed (or not where NDI puts it).
    #[error("the NDI Runtime is not installed (looked in {looked})")]
    NotInstalled {
        /// Where it looked, for the log.
        looked: String,
    },
    /// The library is there but is not one SimpleWorship can use.
    #[error("the NDI library at {path} cannot be used: {reason}")]
    Unusable {
        /// The file.
        path: String,
        /// What went wrong.
        reason: String,
    },
    /// NDI refused to start (an unsupported processor, say).
    #[error("NDI could not start on this computer")]
    NotSupported,
    /// A sender could not be made.
    #[error("could not create the NDI source {0:?}")]
    Sender(String),
}

// ── NDI's types (from its headers) ────────────────────────────────────────

/// `NDIlib_send_create_t`.
#[repr(C)]
struct SendCreate {
    p_ndi_name: *const c_char,
    p_groups: *const c_char,
    clock_video: bool,
    clock_audio: bool,
}

/// `NDIlib_video_frame_v2_t`.
#[repr(C)]
struct VideoFrame {
    xres: c_int,
    yres: c_int,
    four_cc: u32,
    frame_rate_n: c_int,
    frame_rate_d: c_int,
    picture_aspect_ratio: f32,
    frame_format_type: c_int,
    timecode: i64,
    p_data: *const u8,
    line_stride_in_bytes: c_int,
    p_metadata: *const c_char,
    timestamp: i64,
}

/// `NDIlib_FourCC_video_type_BGRA`: blue, green, red, alpha (straight).
const FOURCC_BGRA: u32 = u32::from_le_bytes(*b"BGRA");
/// `NDIlib_frame_format_type_progressive`.
const PROGRESSIVE: c_int = 1;
/// `NDIlib_send_timecode_synthesize`: NDI fills in the timecode.
const TIMECODE_SYNTHESIZE: i64 = i64::MAX;

type Initialize = unsafe extern "C" fn() -> bool;
type SendCreateFn = unsafe extern "C" fn(*const SendCreate) -> *mut c_void;
type SendDestroy = unsafe extern "C" fn(*mut c_void);
type SendVideo = unsafe extern "C" fn(*mut c_void, *const VideoFrame);
type GetConnections = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
type Version = unsafe extern "C" fn() -> *const c_char;

/// The NDI library, loaded.
pub struct Runtime {
    /// Kept loaded while the functions below are in use.
    _library: Library,
    path: PathBuf,
    version: String,
    send_create: SendCreateFn,
    send_destroy: SendDestroy,
    send_video: SendVideo,
    get_connections: GetConnections,
}

// The function pointers are plain C functions that NDI documents as safe
// to call from any thread.
unsafe impl Send for Runtime {}
unsafe impl Sync for Runtime {}

impl Runtime {
    /// Where the NDI library may be, most likely first.
    pub fn candidates() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        let file = library_file();
        for variable in ["NDI_RUNTIME_DIR_V6", "NDI_RUNTIME_DIR_V5"] {
            if let Some(folder) = std::env::var_os(variable) {
                paths.push(PathBuf::from(folder).join(file));
            }
        }
        #[cfg(target_os = "macos")]
        paths.extend(
            [
                "/usr/local/lib/libndi.dylib",
                "/Library/NDI SDK for Apple/lib/macOS/libndi.dylib",
            ]
            .map(PathBuf::from),
        );
        #[cfg(windows)]
        paths.extend(
            [
                r"C:\Program Files\NDI\NDI 6 Runtime\v6",
                r"C:\Program Files\NDI\NDI 5 Runtime\v5",
                r"C:\Program Files\NDI\NDI 6 Tools\Runtime",
            ]
            .map(|folder| PathBuf::from(folder).join(file)),
        );
        #[cfg(target_os = "linux")]
        paths.extend(["libndi.so.6", "libndi.so.5"].map(PathBuf::from));
        paths
    }

    /// Loads the NDI library from the first place it is found.
    pub fn load() -> Result<Arc<Self>, NdiError> {
        let candidates = Self::candidates();
        for path in &candidates {
            // A bare name (Linux) is searched for by the system.
            let exists = path.is_file() || path.parent() == Some(std::path::Path::new(""));
            if exists {
                return Self::load_from(path.clone()).map(Arc::new);
            }
        }
        Err(NdiError::NotInstalled {
            looked: candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
        })
    }

    fn load_from(path: PathBuf) -> Result<Self, NdiError> {
        let unusable = |reason: String| NdiError::Unusable {
            path: path.display().to_string(),
            reason,
        };
        // Loading a library runs its start-up code; NDI's is well behaved.
        let library = unsafe { Library::new(&path) }.map_err(|e| unusable(e.to_string()))?;
        let symbol = |name: &[u8]| -> Result<*const c_void, NdiError> {
            unsafe { library.get::<*const c_void>(name) }
                .map(|symbol| *symbol)
                .map_err(|e| unusable(e.to_string()))
        };
        // Each is cast to the type NDI's header gives it.
        let (initialize, send_create, send_destroy, send_video, get_connections, version) = unsafe {
            (
                std::mem::transmute::<*const c_void, Initialize>(symbol(b"NDIlib_initialize")?),
                std::mem::transmute::<*const c_void, SendCreateFn>(symbol(b"NDIlib_send_create")?),
                std::mem::transmute::<*const c_void, SendDestroy>(symbol(b"NDIlib_send_destroy")?),
                std::mem::transmute::<*const c_void, SendVideo>(symbol(
                    b"NDIlib_send_send_video_v2",
                )?),
                std::mem::transmute::<*const c_void, GetConnections>(symbol(
                    b"NDIlib_send_get_no_connections",
                )?),
                std::mem::transmute::<*const c_void, Version>(symbol(b"NDIlib_version")?),
            )
        };
        if !unsafe { initialize() } {
            return Err(NdiError::NotSupported);
        }
        let version = unsafe {
            let text = version();
            if text.is_null() {
                String::new()
            } else {
                std::ffi::CStr::from_ptr(text)
                    .to_string_lossy()
                    .into_owned()
            }
        };
        log::info!("NDI: loaded {} ({version})", path.display());
        Ok(Self {
            _library: library,
            path,
            version,
            send_create,
            send_destroy,
            send_video,
            get_connections,
        })
    }

    /// Where the library was loaded from.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// The library's version, e.g. "NDI SDK APPLE 6.1.1".
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// The library's file name on this system.
fn library_file() -> &'static str {
    if cfg!(windows) {
        if cfg!(target_arch = "aarch64") {
            "Processing.NDI.Lib.arm64.dll"
        } else {
            "Processing.NDI.Lib.x64.dll"
        }
    } else if cfg!(target_os = "macos") {
        "libndi.dylib"
    } else {
        "libndi.so.6"
    }
}

// ── Sending ───────────────────────────────────────────────────────────────

/// An NDI source on the network, sending a [`Compositor`]'s picture 30
/// times a second from a thread of its own until dropped.
pub struct Sender {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// NDI's sender handle, moved to the sending thread.
struct Instance {
    runtime: Arc<Runtime>,
    handle: *mut c_void,
}

// NDI allows a sender to be used from another thread than the one that
// made it (one thread at a time, which is all this does).
unsafe impl Send for Instance {}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { (self.runtime.send_destroy)(self.handle) };
    }
}

impl Sender {
    /// Creates the NDI source `name` (receivers show it as
    /// "COMPUTER (name)") and starts sending `compositor`'s picture.
    /// `on_receivers` is called with the number of receivers whenever it
    /// changes.
    pub fn start(
        runtime: Arc<Runtime>,
        name: &str,
        compositor: Arc<Compositor>,
        on_receivers: impl Fn(u32) + Send + 'static,
    ) -> Result<Self, NdiError> {
        let c_name = CString::new(name).map_err(|_| NdiError::Sender(name.to_owned()))?;
        let settings = SendCreate {
            p_ndi_name: c_name.as_ptr(),
            p_groups: std::ptr::null(),
            // NDI paces the sending at the frame rate.
            clock_video: true,
            clock_audio: false,
        };
        let handle = unsafe { (runtime.send_create)(&settings) };
        if handle.is_null() {
            return Err(NdiError::Sender(name.to_owned()));
        }
        let instance = Instance { runtime, handle };
        let stop = Arc::new(AtomicBool::new(false));
        let thread = std::thread::Builder::new()
            .name(format!("ndi {name}"))
            .spawn({
                let stop = stop.clone();
                move || send_loop(&instance, &compositor, &stop, on_receivers)
            })
            .map_err(|_| NdiError::Sender(name.to_owned()))?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for Sender {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Sends the compositor's picture until told to stop. The picture is only
/// converted to NDI's straight alpha when it changed.
fn send_loop(
    instance: &Instance,
    compositor: &Compositor,
    stop: &AtomicBool,
    on_receivers: impl Fn(u32),
) {
    let mut premultiplied = Vec::new();
    let mut straight = Vec::new();
    let mut sent_version = u64::MAX;
    let mut receivers = None;
    let mut frames: u64 = 0;

    while !stop.load(Ordering::SeqCst) {
        let version = compositor.render(Instant::now(), sent_version, &mut premultiplied);
        if version != sent_version || straight.is_empty() {
            Frame::write_straight(&premultiplied, &mut straight);
            sent_version = version;
        }
        let frame = VideoFrame {
            xres: WIDTH as c_int,
            yres: HEIGHT as c_int,
            four_cc: FOURCC_BGRA,
            frame_rate_n: FRAME_RATE as c_int,
            frame_rate_d: 1,
            picture_aspect_ratio: WIDTH as f32 / HEIGHT as f32,
            frame_format_type: PROGRESSIVE,
            timecode: TIMECODE_SYNTHESIZE,
            p_data: straight.as_ptr(),
            line_stride_in_bytes: (WIDTH * 4) as c_int,
            p_metadata: std::ptr::null(),
            timestamp: 0,
        };
        // Blocks until it is time for the next frame (`clock_video`).
        unsafe { (instance.runtime.send_video)(instance.handle, &frame) };

        // About every two seconds: who is watching?
        if frames.is_multiple_of(u64::from(FRAME_RATE * 2)) {
            let count = unsafe { (instance.runtime.get_connections)(instance.handle, 0) };
            let count = u32::try_from(count).unwrap_or(0);
            if receivers != Some(count) {
                receivers = Some(count);
                on_receivers(count);
            }
        }
        frames += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_in_the_usual_places() {
        let candidates = Runtime::candidates();
        assert!(!candidates.is_empty());
        #[cfg(target_os = "macos")]
        assert!(candidates.contains(&PathBuf::from("/usr/local/lib/libndi.dylib")));
    }

    #[test]
    fn a_missing_library_is_reported_not_fatal() {
        let error = Runtime::load_from(PathBuf::from("/nowhere/libndi.dylib"))
            .err()
            .expect("there is no library there");
        assert!(matches!(error, NdiError::Unusable { .. }));
    }

    #[test]
    fn the_pixel_format_is_bgra() {
        assert_eq!(FOURCC_BGRA, 0x4152_4742);
    }
}
