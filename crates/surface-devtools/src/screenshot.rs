//! Decode and publish screenshots returned by the connected host.
use crate::connection::ClientError;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;
use surface_devtools_protocol::ScreenshotResult;

const PNG_HEADER: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR";

pub(crate) fn parse(result: Value) -> Result<(ScreenshotResult, Vec<u8>), ClientError> {
    let result: ScreenshotResult = serde_json::from_value(result)
        .map_err(|error| ClientError::Protocol(format!("invalid screenshot result: {error}")))?;
    if result.format != "png"
        || result.width == 0
        || result.height == 0
        || !result.scale.is_finite()
        || result.scale <= 0.
    {
        return Err(ClientError::Protocol("invalid screenshot metadata".into()));
    }
    let bytes = STANDARD
        .decode(&result.data_base64)
        .map_err(|error| ClientError::Protocol(format!("invalid screenshot base64: {error}")))?;
    if !bytes.starts_with(PNG_HEADER)
        || bytes.get(16..20) != Some(&result.width.to_be_bytes())
        || bytes.get(20..24) != Some(&result.height.to_be_bytes())
        || !bytes.ends_with(b"\x00\x00\x00\x00IEND\xaeB\x60\x82")
    {
        return Err(ClientError::Protocol(
            "screenshot PNG does not match its metadata or is incomplete".into(),
        ));
    }
    Ok((result, bytes))
}

pub(crate) fn metadata(result: &ScreenshotResult) -> Value {
    json!({
        "format": result.format,
        "width": result.width,
        "height": result.height,
        "scale": result.scale,
    })
}

pub(crate) fn save_png(result: Value, path: &Path) -> Result<Value, ClientError> {
    let (result, bytes) = parse(result)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let write_error = |error: std::io::Error| {
        ClientError::Io(format!("write screenshot {}: {error}", path.display()))
    };
    // A private temporary file in the same directory lets us publish a
    // complete PNG without replacing an existing file or following a symlink.
    let mut file = tempfile::Builder::new()
        .prefix(".surface-screenshot-")
        .tempfile_in(parent)
        .map_err(write_error)?;
    file.write_all(&bytes).map_err(write_error)?;
    file.persist_noclobber(path)
        .map_err(|error| write_error(error.error))?;
    let mut result = metadata(&result);
    result["path"] = json!(path);
    Ok(result)
}
