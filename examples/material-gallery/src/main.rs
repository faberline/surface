fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = surface_app::Runtime::new(surface_material_gallery::Gallery::default())?;
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.iter().any(|arg| arg == "--headless") {
        #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
        {
            let (width, height) = headless_viewport(&arguments)?;
            return surface_platform::run_headless_runtime(runtime, width, height);
        }
        #[cfg(not(all(feature = "devtools", not(target_arch = "wasm32"))))]
        return Err("build the native Gallery with --features devtools for --headless".into());
    }
    let mut options = surface_platform::Options::default();
    if arguments.iter().any(|arg| arg == "--smoke") {
        options.close_after_frames = Some(2);
    }
    surface_platform::run_runtime(std::rc::Rc::new(std::cell::RefCell::new(runtime)), options)?;
    Ok(())
}

#[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
fn headless_viewport(arguments: &[String]) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let (mut width, mut height) = (960, 800);
    let mut args = arguments.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => {}
            "--width" => width = args.next().ok_or("missing value for --width")?.parse()?,
            "--height" => height = args.next().ok_or("missing value for --height")?.parse()?,
            flag => return Err(format!("unexpected headless argument {flag}").into()),
        }
    }
    Ok((width, height))
}
