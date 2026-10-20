//! WebKit's `takeSnapshotWithConfiguration:completionHandler:`.

#![allow(unsafe_code)]

use std::ffi::c_void;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2_app_kit::NSImage;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImageAlphaInfo, CGImageByteOrderInfo,
    kCGColorSpaceSRGB,
};
use objc2_foundation::{NSError, NSNumber};
use objc2_web_kit::{WKSnapshotConfiguration, WKWebView};

use super::Done;
use crate::frame::Frame;
use crate::{HEIGHT, WIDTH};

pub fn snapshot(window: &tauri::WebviewWindow, done: Done) {
    // WebKit measures in points; a Retina screen has two pixels per point.
    let scale = window.scale_factor().unwrap_or(1.0);
    let width = f64::from(WIDTH) / scale;
    let height = f64::from(HEIGHT) / scale;

    let mut done = Some(done);
    let result = window.with_webview(move |webview| {
        let Some(done) = done.take() else { return };
        let Some(mtm) = MainThreadMarker::new() else {
            done(Err("not on the main thread".to_owned()));
            return;
        };
        // Tauri hands over the page's WKWebView.
        let web_view = unsafe { &*webview.inner().cast::<WKWebView>() };
        let config = unsafe { WKSnapshotConfiguration::new(mtm) };
        unsafe {
            config.setRect(CGRect::new(
                CGPoint::new(0.0, 0.0),
                CGSize::new(width, height),
            ));
            config.setSnapshotWidth(Some(&NSNumber::new_f64(width)));
            // Include what the page drew a moment ago.
            config.setAfterScreenUpdates(true);
        }
        let done = std::cell::Cell::new(Some(done));
        let handler = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
            let Some(done) = done.take() else { return };
            if image.is_null() {
                let reason = if error.is_null() {
                    "no picture".to_owned()
                } else {
                    unsafe { &*error }.localizedDescription().to_string()
                };
                done(Err(reason));
                return;
            }
            done(to_frame(unsafe { &*image }));
        });
        unsafe {
            web_view.takeSnapshotWithConfiguration_completionHandler(Some(&config), &handler)
        };
    });
    if let Err(error) = result {
        log::warn!("NDI: could not reach the page: {error}");
    }
}

/// Draws the picture into a buffer laid out as a [`Frame`]: blue, green,
/// red, alpha, premultiplied, in sRGB.
fn to_frame(image: &NSImage) -> Result<Frame, String> {
    let image =
        unsafe { image.CGImageForProposedRect_context_hints(std::ptr::null_mut(), None, None) }
            .ok_or("the picture has no pixels")?;
    let mut frame = Frame::transparent(WIDTH, HEIGHT);
    let space = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }));
    // Little-endian 32-bit with alpha first: B, G, R, A in memory.
    let info = CGImageAlphaInfo::PremultipliedFirst.0 | CGImageByteOrderInfo::Order32Little.0;
    let context = unsafe {
        CGBitmapContextCreate(
            frame.pixels.as_mut_ptr().cast::<c_void>(),
            WIDTH as usize,
            HEIGHT as usize,
            8,
            WIDTH as usize * 4,
            space.as_deref(),
            info,
        )
    }
    .ok_or("could not make a drawing buffer")?;
    let bounds = CGRect::new(
        CGPoint::new(0.0, 0.0),
        CGSize::new(f64::from(WIDTH), f64::from(HEIGHT)),
    );
    CGContext::draw_image(Some(&context), bounds, Some(&image));
    // The context drew straight into `frame.pixels`.
    drop(context);
    Ok(frame)
}
