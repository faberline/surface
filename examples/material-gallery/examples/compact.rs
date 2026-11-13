//! A small native viewport for the same gallery and bridge acceptance journey.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = surface_app::Runtime::new(surface_material_gallery::Gallery::default())?;
    surface_platform::run_runtime(
        std::rc::Rc::new(std::cell::RefCell::new(runtime)),
        surface_platform::Options {
            width: 420,
            height: 360,
            close_after_frames: std::env::args().any(|arg| arg == "--smoke").then_some(2),
            ..Default::default()
        },
    )?;
    Ok(())
}
