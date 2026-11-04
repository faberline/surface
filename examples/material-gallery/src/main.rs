fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = surface_app::Runtime::new(surface_material_gallery::Gallery::default())?;
    let mut options = surface_platform::Options::default();
    if std::env::args().any(|s| s == "--smoke") {
        options.close_after_frames = Some(2);
    }
    surface_platform::run_runtime(std::rc::Rc::new(std::cell::RefCell::new(runtime)), options)?;
    Ok(())
}
