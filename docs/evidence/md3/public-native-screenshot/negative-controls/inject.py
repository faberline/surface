"""Inject local negative controls into a disposable native screenshot host."""
import difflib
import hashlib
from pathlib import Path
import sys

output = Path(__file__).resolve().parent
source = Path(sys.argv[1]).resolve()
assert source == output / 'repo', f"refusing to edit source outside {output}"
relative = Path('crates/surface-platform/src/screenshot.rs')
file = source / relative
normal = file.read_text()
modified = normal


def replace(old, new):
    global modified
    assert modified.count(old) == 1, f"fixture anchor changed: {old}"
    modified = modified.replace(old, new, 1)


replace(
    'pub(crate) fn capture(\n',
    '''// Disposable DN25 negative controls. Never part of a product build.
#[derive(Clone, Copy, PartialEq)]
enum ScreenshotNegativeControl {
    SwappedRedBlue,
    StaleDefaultImage,
    IgnoredMaxSide,
}

fn screenshot_negative_control() -> Option<ScreenshotNegativeControl> {
    match std::env::var("SURFACE_SCREENSHOT_NEGATIVE_CONTROL").as_deref() {
        Ok("swapped-red-blue") => Some(ScreenshotNegativeControl::SwappedRedBlue),
        Ok("stale-default-image") => Some(ScreenshotNegativeControl::StaleDefaultImage),
        Ok("ignored-max-side") => Some(ScreenshotNegativeControl::IgnoredMaxSide),
        _ => None,
    }
}

std::thread_local! {
    static FIRST_DEFAULT_IMAGE: std::cell::RefCell<Option<Value>> = const {
        std::cell::RefCell::new(None)
    };
}

pub(crate) fn capture(
''',
)
replace(
    '''    params: ScreenshotParams,
) -> Reply {
    let Some(resolution) = Resolution::new(
''',
    '''    mut params: ScreenshotParams,
) -> Reply {
    let fault = screenshot_negative_control();
    let cacheable = fault == Some(ScreenshotNegativeControl::StaleDefaultImage)
        && params == ScreenshotParams::default();
    if cacheable {
        if let Some(result) = FIRST_DEFAULT_IMAGE.with(|entry| entry.borrow().clone()) {
            eprintln!("[negative-control] returned the cached first default PNG");
            return Reply::ok(id, result);
        }
    }
    if fault == Some(ScreenshotNegativeControl::IgnoredMaxSide) && params.max_side.is_some() {
        eprintln!("[negative-control] ignored max_side {:?}", params.max_side);
        params.max_side = None;
    }
    let Some(resolution) = Resolution::new(
''',
)
replace(
    '''        Ok(capture) => png_reply(id, &capture, resolution.scale, RESPONSE_CAP_BYTES as usize),
''',
    '''        Ok(mut capture) => {
            if fault == Some(ScreenshotNegativeControl::SwappedRedBlue) {
                eprintln!("[negative-control] swapped red and blue in the rendered PNG");
                for rgba in capture.rgba.chunks_exact_mut(4) {
                    rgba.swap(0, 2);
                }
            }
            let reply = png_reply(id, &capture, resolution.scale, RESPONSE_CAP_BYTES as usize);
            if cacheable {
                FIRST_DEFAULT_IMAGE.with(|entry| *entry.borrow_mut() = reply.result.clone());
            }
            reply
        }
''',
)
file.write_text(modified)
with (output / 'fault.patch').open('w') as diff:
    diff.writelines(
        difflib.unified_diff(
            normal.splitlines(keepends=True),
            modified.splitlines(keepends=True),
            fromfile=f'a/{relative}',
            tofile=f'b/{relative}',
        )
    )
print(f"untouched host screenshot source {hashlib.sha256(normal.encode()).hexdigest()}")
print(f"fault-injected host source {hashlib.sha256(modified.encode()).hexdigest()}")
