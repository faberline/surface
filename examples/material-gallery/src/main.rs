fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = surface_app::Runtime::new(surface_material_gallery::Gallery::default())?;
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.iter().any(|arg| arg == "--headless") {
        #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
        {
            let options = headless_options(&arguments)?;
            let mut runtime = runtime;
            if let Some(contrast) = options.contrast {
                runtime.set_display_contrast(contrast)?;
            }
            return surface_platform::run_headless_runtime(runtime, options.width, options.height);
        }
        #[cfg(not(all(feature = "devtools", not(target_arch = "wasm32"))))]
        return Err("build the native Gallery with --features devtools for --headless".into());
    }
    if arguments.iter().any(|arg| arg == "--forced-colors") {
        return Err("--forced-colors requires --headless".into());
    }
    let mut options = surface_platform::Options::default();
    if arguments.iter().any(|arg| arg == "--smoke") {
        options.close_after_frames = Some(2);
    }
    surface_platform::run_runtime(std::rc::Rc::new(std::cell::RefCell::new(runtime)), options)?;
    Ok(())
}

#[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
struct HeadlessOptions {
    width: u32,
    height: u32,
    contrast: Option<surface_app::DisplayContrast>,
}

#[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
fn headless_options(arguments: &[String]) -> Result<HeadlessOptions, Box<dyn std::error::Error>> {
    let mut options = HeadlessOptions {
        width: 960,
        height: 800,
        contrast: None,
    };
    let mut args = arguments.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => {}
            "--width" => {
                options.width = args.next().ok_or("missing value for --width")?.parse()?;
            }
            "--height" => {
                options.height = args.next().ok_or("missing value for --height")?.parse()?;
            }
            "--forced-colors" => {
                let contrast = match args.next().map(String::as_str) {
                    Some("light") => preview_contrast(false),
                    Some("dark") => preview_contrast(true),
                    Some("field-pair") => field_pair_preview(),
                    _ => return Err("--forced-colors expects light, dark, or field-pair".into()),
                };
                options.contrast = Some(contrast);
            }
            flag => return Err(format!("unexpected headless argument {flag}").into()),
        }
    }
    Ok(options)
}

#[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
fn preview_contrast(dark: bool) -> surface_app::DisplayContrast {
    use surface::Color;
    use surface_app::{DisplayContrast, SystemColorPair, SystemContrastColors};

    // CSS Color Adjustment Level 1, Forced Colors Mode Color Palettes.
    let black = Color::rgb(0, 0, 0);
    let white = Color::rgb(255, 255, 255);
    let (background, foreground, disabled, highlight) = if dark {
        (
            black,
            white,
            Color::rgb(63, 242, 63),
            Color::rgb(26, 235, 255),
        )
    } else {
        (white, black, Color::rgb(96, 0, 0), Color::rgb(55, 0, 110))
    };
    let canvas = SystemColorPair {
        background,
        foreground,
    };
    let selection = SystemColorPair {
        background: highlight,
        foreground: background,
    };
    DisplayContrast::Forced(
        SystemContrastColors::new(canvas, canvas, selection)
            .with_disabled_text(disabled)
            .with_field(canvas),
    )
}

#[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
fn field_pair_preview() -> surface_app::DisplayContrast {
    use surface::Color;
    use surface_app::{DisplayContrast, SystemColorPair, SystemContrastColors};

    // Synthetic palette for identifying each pair in a headless screenshot.
    let pair = |[br, bg, bb]: [u8; 3], [fr, fg, fb]: [u8; 3]| SystemColorPair {
        background: Color::rgb(br, bg, bb),
        foreground: Color::rgb(fr, fg, fb),
    };
    DisplayContrast::Forced(
        SystemContrastColors::new(
            pair([248, 247, 250], [28, 23, 38]),
            pair([92, 35, 45], [255, 232, 216]),
            pair([63, 24, 97], [237, 239, 254]),
        )
        .with_disabled_text(Color::rgb(119, 56, 71))
        .with_field(pair([19, 45, 47], [249, 220, 126])),
    )
}
