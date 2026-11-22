//! Capture a native Surface frame for the authenticated DevTools session.
use std::io::{self, Write};

use base64::prelude::{Engine, BASE64_STANDARD};
use serde_json::{json, Value};
use surface::ExtendedDisplayList;
use surface_devtools_protocol::{
    ErrorCode, Reply, ScreenshotParams, ScreenshotResult, RESPONSE_CAP_BYTES,
    SCREENSHOT_DEFAULT_MAX_SIDE, SCREENSHOT_MAX_SIDE,
};
use surface_wgpu::{Capture, Renderer, MAX_CAPTURE_SIDE};

pub(crate) fn ready<W, G>(window: Option<W>, gpu: Option<G>, id: u64) -> Result<(W, G), Reply> {
    window.zip(gpu).ok_or_else(|| {
        Reply::error(
            id,
            ErrorCode::NotReady,
            "the application renderer is not ready",
        )
    })
}

pub(crate) fn supports_format(format: wgpu::TextureFormat) -> bool {
    use wgpu::TextureFormat::*;

    matches!(
        format,
        Rgba8Unorm | Rgba8UnormSrgb | Bgra8Unorm | Bgra8UnormSrgb
    )
}

pub(crate) fn params(value: &Value) -> Result<ScreenshotParams, String> {
    if !value.is_object() {
        return Err("screenshot params must be an object".into());
    }
    let params: ScreenshotParams =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    if params
        .scale
        .is_some_and(|scale| !scale.is_finite() || scale <= 0.)
    {
        return Err("scale must be finite and greater than zero".into());
    }
    if params
        .max_side
        .is_some_and(|side| !(1..=SCREENSHOT_MAX_SIDE).contains(&side))
    {
        return Err(format!(
            "max_side must be between 1 and {SCREENSHOT_MAX_SIDE}"
        ));
    }
    Ok(params)
}

#[derive(Debug, PartialEq)]
struct Resolution {
    width: u32,
    height: u32,
    scale: f32,
}

impl Resolution {
    fn new(
        (width, height): (u32, u32),
        native_scale: f32,
        params: ScreenshotParams,
        device_max_side: u32,
    ) -> Option<Self> {
        let max_side = params
            .max_side
            .unwrap_or(SCREENSHOT_DEFAULT_MAX_SIDE)
            .min(MAX_CAPTURE_SIDE)
            .min(device_max_side);
        if width == 0
            || height == 0
            || max_side == 0
            || !native_scale.is_finite()
            || native_scale <= 0.
        {
            return None;
        }

        let longest = f64::from(width.max(height));
        let native_scale = f64::from(native_scale);
        let requested = params.scale.map_or(native_scale, f64::from);
        // Scale the same logical viewport in both directions. Keep at least
        // one pixel even if the requested scale is smaller than useful.
        let ratio = (requested / native_scale).clamp(1. / longest, f64::from(max_side) / longest);
        let side = |pixels| (f64::from(pixels) * ratio).round().max(1.) as u32;
        let scale = (native_scale * ratio) as f32;
        (scale.is_finite() && scale > 0.).then(|| Self {
            width: side(width),
            height: side(height),
            scale,
        })
    }
}

pub(crate) fn capture(
    renderer: &mut Renderer,
    frame: &ExtendedDisplayList,
    size: (u32, u32),
    native_scale: f32,
    id: u64,
    params: ScreenshotParams,
) -> Reply {
    let Some(resolution) = Resolution::new(
        size,
        native_scale,
        params,
        renderer.device.limits().max_texture_dimension_2d,
    ) else {
        return Reply::error(id, ErrorCode::NotReady, "the window has no drawable size");
    };
    match renderer.capture_extended(frame, resolution.width, resolution.height, resolution.scale) {
        Ok(capture) => png_reply(id, &capture, resolution.scale, RESPONSE_CAP_BYTES as usize),
        Err(error) => Reply::error(id, ErrorCode::RenderFailed, error.to_string()),
    }
}

fn png_reply(id: u64, capture: &Capture, scale: f32, response_cap: usize) -> Reply {
    let mut result = ScreenshotResult {
        format: "png".into(),
        width: capture.width,
        height: capture.height,
        scale,
        data_base64: String::new(),
    };
    // Standard base64 uses no JSON escapes. Measure the envelope with empty
    // data so the PNG limit includes metadata, base64 padding and newline.
    let empty = Reply::ok(id, json!(&result));
    let overhead = match serde_json::to_vec(&empty) {
        Ok(bytes) => bytes.len() + 1,
        Err(error) => return Reply::error(id, ErrorCode::RenderFailed, error.to_string()),
    };
    let mut png = CappedPng {
        bytes: Vec::new(),
        max_bytes: response_cap.saturating_sub(overhead) / 4 * 3,
        too_large: false,
    };
    if let Err(error) = encode_png(capture, &mut png) {
        if png.too_large {
            // A 1024 px square of RGBA pixels fits comfortably even when its
            // colors compress poorly. Also make sure the hint reduces size.
            let hint = capture
                .width
                .max(capture.height)
                .saturating_sub(1)
                .clamp(1, 1024);
            return Reply::error(
                id,
                ErrorCode::TooLarge,
                format!("the PNG exceeds the response limit; retry with max_side={hint} or less"),
            );
        }
        return Reply::error(id, ErrorCode::RenderFailed, error.to_string());
    }
    result.data_base64 = BASE64_STANDARD.encode(png.bytes);
    Reply::ok(id, json!(result))
}

fn encode_png(capture: &Capture, out: impl Write) -> Result<(), png::EncodingError> {
    let mut encoder = png::Encoder::new(out, capture.width, capture.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    let mut stream = writer.stream_writer()?;
    stream.write_all(&capture.rgba)?;
    stream.finish()?;
    writer.finish()
}

struct CappedPng {
    bytes: Vec<u8>,
    max_bytes: usize,
    too_large: bool,
}

impl Write for CappedPng {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.max_bytes - self.bytes.len() {
            self.too_large = true;
            return Err(io::Error::other("PNG exceeds the response limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_requires_both_window_and_gpu_before_capture() {
        for (window, gpu) in [(None, None), (Some(()), None), (None, Some(()))] {
            let reply = ready(window, gpu, 15).unwrap_err();
            assert_eq!(reply.id, 15);
            assert!(!reply.ok);
            assert_eq!(reply.error.unwrap().code, ErrorCode::NotReady);
        }
        assert!(ready(Some(()), Some(()), 15).is_ok());
    }

    #[test]
    fn screenshot_accepts_only_the_renderers_rgba8_and_bgra8_formats() {
        use wgpu::TextureFormat::*;

        assert!(supports_format(Rgba8UnormSrgb));
        assert!(supports_format(Bgra8Unorm));
        assert!(!supports_format(Rgba16Float));
        assert!(!supports_format(Rgb10a2Unorm));
        assert!(!supports_format(Rgba8Snorm));
    }

    #[test]
    fn screenshot_parameters_validate_bounds() {
        assert_eq!(params(&json!({})), Ok(ScreenshotParams::default()));
        assert_eq!(
            params(&json!({"scale": 1.5, "max_side": 8192})),
            Ok(ScreenshotParams {
                scale: Some(1.5),
                max_side: Some(8192)
            })
        );
        for value in [
            json!([]),
            json!({"scale": 0}),
            json!({"scale": -1}),
            json!({"scale": 1e100_f64}),
            json!({"scale": "bad"}),
            json!({"max_side": 0}),
            json!({"max_side": 8193}),
            json!({"max_side": 1.2}),
        ] {
            assert!(params(&value).is_err(), "{value}");
        }
    }

    #[test]
    fn resolution_preserves_window_aspect_and_reports_scale() {
        let native = (1800, 1200);
        let options = ScreenshotParams::default();
        assert_eq!(
            Resolution::new(native, 2., options, 8192),
            Some(Resolution {
                width: 1800,
                height: 1200,
                scale: 2.
            })
        );
        let capped = ScreenshotParams {
            scale: Some(3.),
            max_side: Some(900),
        };
        assert_eq!(
            Resolution::new(native, 2., capped, 8192),
            Some(Resolution {
                width: 900,
                height: 600,
                scale: 1.
            })
        );
        assert_eq!(
            Resolution::new(native, 2., options, 450),
            Some(Resolution {
                width: 450,
                height: 300,
                scale: 0.5
            })
        );
        let tiny = ScreenshotParams {
            scale: Some(f32::MIN_POSITIVE),
            max_side: None,
        };
        let tiny = Resolution::new(native, 2., tiny, 8192).unwrap();
        assert_eq!((tiny.width, tiny.height), (1, 1));
        assert!(tiny.scale > 0.);
        assert!(Resolution::new((0, 1200), 2., options, 8192).is_none());
        assert!(Resolution::new(native, 0., options, 8192).is_none());
    }

    #[test]
    fn png_reply_round_trips_rgba_without_a_gpu() {
        let capture = Capture {
            width: 2,
            height: 2,
            rgba: vec![1, 2, 3, 255, 4, 5, 6, 255, 10, 20, 30, 255, 2, 4, 8, 123],
        };
        let reply = png_reply(7, &capture, 2., 1024);
        assert_eq!(reply.id, 7);
        assert!(reply.ok);
        let png: ScreenshotResult = serde_json::from_value(reply.result.unwrap()).unwrap();
        assert_eq!(
            (png.format.as_str(), png.width, png.height, png.scale),
            ("png", 2, 2, 2.)
        );
        let bytes = BASE64_STANDARD.decode(png.data_base64).unwrap();
        let mut reader = png::Decoder::new(io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut decoded = vec![0; reader.output_buffer_size().expect("test image fits")];
        let frame = reader.next_frame(&mut decoded).unwrap();
        assert_eq!(frame.color_type, png::ColorType::Rgba);
        assert_eq!(&decoded[..frame.buffer_size()], capture.rgba);
    }

    #[test]
    fn over_cap_is_rejected_with_a_smaller_max_side_hint() {
        let capture = Capture {
            width: 3,
            height: 2,
            rgba: vec![127; 24],
        };
        let success = png_reply(5, &capture, 1., 1024);
        assert!(success.ok);
        let exact_cap = serde_json::to_vec(&success).unwrap().len() + 1;
        assert_eq!(png_reply(5, &capture, 1., exact_cap), success);
        let rejected = png_reply(5, &capture, 1., exact_cap - 1);
        assert!(!rejected.ok);
        let error = rejected.error.unwrap();
        assert_eq!(error.code, ErrorCode::TooLarge);
        assert!(error.message.contains("max_side=2"));
    }

    #[test]
    fn capture_at_max_side_obeys_the_wire_response_budget() {
        let width = SCREENSHOT_MAX_SIDE;
        let height = 512;
        // A deterministic noisy image exceeds the wire cap even after PNG
        // compression. A wide strip limits memory use in the test.
        let mut state = 0xa5a5_5a5au32;
        let rgba = std::iter::repeat_with(|| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_le_bytes()
        })
        .take((width * height) as usize)
        .flatten()
        .collect();
        let capture = Capture {
            width,
            height,
            rgba,
        };
        let reply = png_reply(8, &capture, 1., RESPONSE_CAP_BYTES as usize);
        assert_eq!(reply.id, 8);
        assert!(!reply.ok);
        let error = reply.error.unwrap();
        assert_eq!(error.code, ErrorCode::TooLarge);
        assert!(error.message.contains("max_side=1024"));
    }
}
