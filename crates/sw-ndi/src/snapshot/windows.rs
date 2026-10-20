//! WebView2's `CapturePreview`, which gives a PNG.

#![allow(unsafe_code)]

use webview2_com::CapturePreviewCompletedHandler;
use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG;
use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
use windows::Win32::System::Com::{IStream, STREAM_SEEK_SET};

use super::Done;
use crate::frame::Frame;

pub fn snapshot(window: &tauri::WebviewWindow, done: Done) {
    let mut done = Some(done);
    let result = window.with_webview(move |webview| {
        let Some(done) = done.take() else { return };
        let done = std::sync::Mutex::new(Some(done));
        let started = (|| unsafe {
            let core = webview
                .controller()
                .CoreWebView2()
                .map_err(|e| e.to_string())?;
            let stream: IStream =
                CreateStreamOnHGlobal(Default::default(), true).map_err(|e| e.to_string())?;
            let reader = stream.clone();
            let handler = CapturePreviewCompletedHandler::create(Box::new(move |result| {
                let png = result
                    .map_err(|e| e.to_string())
                    .and_then(|()| read_all(&reader));
                if let Some(done) = done.lock().ok().and_then(|mut done| done.take()) {
                    // Decoding takes a few dozen milliseconds: not on the UI thread.
                    std::thread::spawn(move || done(png.and_then(|png| decode(&png))));
                }
                Ok(())
            }));
            core.CapturePreview(
                COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG,
                &stream,
                &handler,
            )
            .map_err(|e| e.to_string())
        })();
        if let Err(error) = started {
            log::warn!("NDI: could not take a picture of the page: {error}");
        }
    });
    if let Err(error) = result {
        log::warn!("NDI: could not reach the page: {error}");
    }
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

fn decode(png_bytes: &[u8]) -> Result<Frame, String> {
    let mut decoder = png::Decoder::new(png_bytes);
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).map_err(|e| e.to_string())?;
    let pixels = &pixels[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels
            .chunks_exact(3)
            .flat_map(|px| [px[0], px[1], px[2], 255])
            .collect(),
        other => return Err(format!("unexpected picture format {other:?}")),
    };
    Ok(Frame::from_rgba(info.width, info.height, &rgba))
}
