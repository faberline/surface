use std::{cell::RefCell, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut size = std::env::args().skip(1);
    let width = size.next().ok_or("missing logical width")?.parse()?;
    let height = size.next().ok_or("missing logical height")?.parse()?;
    if size.next().is_some() {
        return Err("expected logical width and height".into());
    }
    let runtime = surface_app::Runtime::new(surface_material_gallery::Gallery::default())?;
    let options = surface_platform::Options {
        width,
        height,
        ..Default::default()
    };
    surface_platform::run_runtime(Rc::new(RefCell::new(runtime)), options)?;
    Ok(())
}
