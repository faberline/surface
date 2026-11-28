//! Decode the Canvas-only image produced by Chrome's real compositor.

use std::{fs, io::Cursor};

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::json;

use crate::browser::Tab;
use crate::{check, Env};

pub(super) type Rect = [f64; 4];
pub(super) type Rgb = [u8; 3];

pub(super) struct CanvasPng {
    bytes: Vec<u8>,
    info: png::OutputInfo,
    channels: usize,
    scale: f64,
}

impl CanvasPng {
    pub(super) async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Self> {
        Self::capture_at_scale(env, tab, name, 2.).await
    }

    pub(super) async fn capture_at_scale(
        env: &Env,
        tab: &Tab,
        name: &str,
        scale: f64,
    ) -> Result<Self> {
        Self::capture_file(
            env,
            tab,
            &format!("material-web-contrast-{name}.png"),
            scale,
        )
        .await
    }

    pub(super) async fn capture_file(
        env: &Env,
        tab: &Tab,
        filename: &str,
        scale: f64,
    ) -> Result<Self> {
        tab.settle().await?;
        let [x, y, width, height] = tab.canvas_box().await?;
        let dpr = tab.eval("devicePixelRatio").await?;
        check!(
            dpr.as_f64() == Some(scale),
            "the Canvas uses the requested DPR: {dpr}"
        );
        let response = tab
            .cdp(
                "Page.captureScreenshot",
                json!({
                    "format":"png","fromSurface":true,"captureBeyondViewport":false,
                    "clip":{"x":x,"y":y,"width":width,"height":height,"scale":1}
                }),
            )
            .await?;
        let png = STANDARD.decode(response["data"].as_str().context("Chrome PNG bytes")?)?;
        fs::write(env.artifact(filename), &png)?;
        let mut decoder = png::Decoder::new(Cursor::new(png));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let mut bytes = vec![0; reader.output_buffer_size().context("PNG buffer size")?];
        let info = reader.next_frame(&mut bytes)?;
        let channels = match info.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => anyhow::bail!("unexpected Chrome PNG color type {other:?}"),
        };
        check!(
            f64::from(info.width) == (scale * width).round()
                && f64::from(info.height) == (scale * height).round(),
            "the actual Canvas-only PNG matches the viewport at DPR {scale}"
        );
        Ok(Self {
            bytes,
            info,
            channels,
            scale,
        })
    }

    pub(super) fn dimensions(&self) -> [u32; 2] {
        [self.info.width, self.info.height]
    }

    pub(super) fn sample(&self, x: f64, y: f64) -> Result<Rgb> {
        let px = (self.scale * x).floor();
        let py = (self.scale * y).floor();
        if !(0. ..f64::from(self.info.width)).contains(&px)
            || !(0. ..f64::from(self.info.height)).contains(&py)
        {
            anyhow::bail!("Canvas point ({x}, {y}) is outside its PNG");
        }
        self.pixel(px as usize, py as usize)
    }

    pub(super) fn pixel(&self, x: usize, y: usize) -> Result<Rgb> {
        if x >= self.info.width as usize || y >= self.info.height as usize {
            anyhow::bail!("Canvas pixel ({x}, {y}) is outside its PNG");
        }
        let offset = y * self.info.line_size + x * self.channels;
        Ok(self.bytes[offset..offset + 3].try_into()?)
    }

    pub(super) fn count_color(&self, [x, y, w, h]: Rect, color: Rgb) -> Result<usize> {
        let edge = |v: f64| (self.scale * v).round().max(0.) as usize;
        let (x1, x2, y1, y2) = (edge(x), edge(x + w), edge(y), edge(y + h));
        check!(
            x1 < x2 && y1 < y2 && x2 <= self.info.width as usize && y2 <= self.info.height as usize,
            "the sampled ink region intersects the actual Canvas PNG"
        );
        let mut count = 0;
        for row in y1..y2 {
            let base = row * self.info.line_size;
            count += self.bytes[base + x1 * self.channels..base + x2 * self.channels]
                .chunks_exact(self.channels)
                .filter(|pixel| pixel[..3] == color)
                .count();
        }
        Ok(count)
    }
}

pub(super) fn intersection(a: Rect, b: Rect) -> Rect {
    let x = a[0].max(b[0]);
    let y = a[1].max(b[1]);
    [
        x,
        y,
        (a[0] + a[2]).min(b[0] + b[2]) - x,
        (a[1] + a[3]).min(b[1] + b[3]) - y,
    ]
}
