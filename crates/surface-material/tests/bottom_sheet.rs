use surface::{MicroDom, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};
use surface_material::{shape, ButtonKind, Theme};

#[derive(Clone)]
enum Msg {
    Open,
    Close,
    Click,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Standard,
    Modal,
}

struct Sheet {
    dark: bool,
    mode: Mode,
    open: bool,
    clicks: usize,
}

impl Sheet {
    fn new(dark: bool, mode: Mode, open: bool) -> Self {
        Self {
            dark,
            mode,
            open,
            clicks: 0,
        }
    }
}

impl Application for Sheet {
    type Message = Msg;
    fn update(&mut self, m: Msg) {
        match m {
            Msg::Open => self.open = true,
            Msg::Close => self.open = false,
            Msg::Click => self.clicks += 1,
        }
    }
    fn view(&self) -> View<Msg> {
        let t = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let mut children = vec![
            t.button_with("outside", "外部", ButtonKind::Text, false, Msg::Click),
            t.button_with("open", "開啟", ButtonKind::Filled, false, Msg::Open),
        ];
        match self.mode {
            Mode::Standard => {
                children.push(t.bottom_sheet("sheet", "Sheet", [t.text("body", "Body")]))
            }
            Mode::Modal => {
                if self.open {
                    children.push(t.modal_bottom_sheet(
                        "sheet",
                        "Sheet",
                        [t.button_with("close", "關閉", ButtonKind::Text, false, Msg::Close)],
                        Msg::Close,
                    ));
                }
            }
        }
        t.page(children)
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

fn render(dark: bool, mode: Mode, open: bool, w: f32, h: f32) -> Runtime<Sheet> {
    let mut r = Runtime::new(Sheet::new(dark, mode, open)).unwrap();
    r.render(w, h, &mut Metrics).unwrap();
    r
}

fn sheet_bounds(r: &Runtime<Sheet>, id: &str) -> surface::Rect {
    let dom: &MicroDom = r.dom();
    let node = dom.by_id(id).expect("sheet node");
    r.computed().unwrap().nodes[node.index()].bounds
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn standard_sheet_has_handle_and_top_corners() {
    let r = render(false, Mode::Standard, false, 400., 600.);
    let dom = r.dom();
    let sheet = dom.by_id("sheet").expect("sheet");
    let handle = dom.children(sheet).next().expect("handle child");
    let handle_bounds = r.computed().unwrap().nodes[handle.index()].bounds;
    assert!(approx(handle_bounds.width, 32.), "handle width");
    assert!(approx(handle_bounds.height, 4.), "handle height");
    let handle_style = dom.get(handle).unwrap().style().clone();
    assert_eq!(
        handle_style.background,
        Theme::light().colors.on_surface_variant
    );
    let style = dom.get(sheet).unwrap().style().clone();
    assert_eq!(style.corners, Some(shape::EXTRA_LARGE_TOP));
    assert_eq!(
        style.background,
        Theme::light().colors.surface_container_low
    );
}

#[test]
fn sheet_is_capped_at_640_wide() {
    let wide = render(false, Mode::Standard, false, 900., 600.);
    assert!(approx(sheet_bounds(&wide, "sheet").width, 640.), "capped");
    let narrow = render(false, Mode::Standard, false, 400., 600.);
    assert!(sheet_bounds(&narrow, "sheet").width <= 400., "narrow");
}

#[test]
fn colors_use_tokens_in_both_themes() {
    for dark in [false, true] {
        let r = render(dark, Mode::Standard, false, 400., 600.);
        let theme = if dark { Theme::dark() } else { Theme::light() };
        let style = r
            .dom()
            .get(r.dom().by_id("sheet").unwrap())
            .unwrap()
            .style()
            .clone();
        assert_eq!(
            style.background, theme.colors.surface_container_low,
            "dark={dark}"
        );
    }
}

#[test]
fn modal_sheet_sits_at_the_bottom() {
    let r = render(false, Mode::Modal, true, 400., 600.);
    let b = sheet_bounds(&r, "sheet");
    assert!(
        approx(b.y + b.height, 600.),
        "bottom edge at {}",
        b.y + b.height
    );
}

#[test]
fn modal_sheet_traps_focus_and_dismisses_on_escape() {
    let mut r = render(false, Mode::Modal, false, 400., 600.);
    r.activate("open").unwrap();
    assert_eq!(r.focused(), Some("close"));
    r.dispatch(Event::Cancel).unwrap();
    assert!(!r.app().open);
    assert_eq!(r.focused(), Some("open"));
}

#[test]
fn outside_click_dismisses_the_modal_sheet() {
    let mut r = render(false, Mode::Modal, false, 400., 600.);
    r.activate("open").unwrap();
    r.render(400., 600., &mut Metrics).unwrap();
    // The "outside" button sits above the sheet, under the scrim.
    let b = r.computed().unwrap().nodes[r.dom().by_id("outside").unwrap().index()].bounds;
    let (x, y) = (b.x + b.width / 2., b.y + b.height / 2.);
    r.dispatch(Event::PointerDown { x, y }).unwrap();
    r.dispatch(Event::PointerUp { x, y }).unwrap();
    assert!(!r.app().open, "outside click closes the sheet");
    assert_eq!(
        r.app().clicks,
        0,
        "control under the scrim is not activated"
    );
}

#[test]
fn standard_sheet_does_not_block_the_page() {
    let mut r = render(false, Mode::Standard, false, 400., 600.);
    assert!(r.activate("outside").unwrap());
    assert_eq!(r.app().clicks, 1);
}
