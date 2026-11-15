//! Material cursor defaults (issue #121). Every button kind gets `Pointer`
//! when enabled, text inputs get `Text`, nothing else gets a cursor.
use surface::TextMeasurer;
use surface_app::{Application, CursorStyle, Runtime, View};
use surface_material::{ButtonKind, ChoiceState, FabColor, FabSize, IconButtonKind, Theme};

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.5, size * 1.5)
    }
}

struct Gallery;
impl Application for Gallery {
    type Message = ();
    fn update(&mut self, _: ()) {}
    fn view(&self) -> View<()> {
        let t = Theme::light();
        let kinds = [
            ("filled", ButtonKind::Filled),
            ("tonal", ButtonKind::Tonal),
            ("outlined", ButtonKind::Outlined),
            ("text", ButtonKind::Text),
        ];
        let mut items = Vec::new();
        for (name, kind) in kinds {
            items.push(t.button_with(&format!("{name}-on"), name, kind, false, ()));
            items.push(t.button_with(&format!("{name}-off"), name, kind, true, ()));
        }
        items.push(t.button("plain", "Plain", ()));
        items.push(t.disabled_button("plain-off", "Plain", ()));
        for off in [false, true] {
            let s = if off { "off" } else { "on" };
            items.push(t.fab(
                &format!("fab-{s}"),
                "Add",
                "+",
                FabSize::Regular,
                FabColor::Primary,
                off,
                (),
            ));
            items.push(t.icon_button(
                &format!("icon-{s}"),
                "Star",
                "*",
                IconButtonKind::Standard,
                None,
                off,
                (),
            ));
        }
        items.push(t.text_field("field", "Name", "", |_| ()));
        items.push(t.search_bar("search", "Search", "", "S", |_| (), ()));
        items.push(t.checkbox("check", "Check", false, ()));
        items.push(t.radio("radio", "Radio", ChoiceState::default(), ()));
        items.push(t.switch("switch", "Switch", ChoiceState::default(), ()));
        t.page(items)
    }
}

fn cursor(rt: &Runtime<Gallery>, id: &str) -> CursorStyle {
    let node = rt.dom().by_id(id).unwrap_or_else(|| panic!("no node {id}"));
    let b = rt.computed().unwrap().nodes[node.index()].bounds;
    rt.cursor_at(b.x + b.width / 2., b.y + b.height / 2.)
}

fn runtime() -> Runtime<Gallery> {
    let mut rt = Runtime::new(Gallery).unwrap();
    rt.render(800., 1600., &mut Metrics).unwrap();
    rt
}

#[test]
fn every_enabled_button_kind_is_a_pointer_and_disabled_is_not() {
    let rt = runtime();
    for name in ["filled", "tonal", "outlined", "text"] {
        assert_eq!(
            cursor(&rt, &format!("{name}-on")),
            CursorStyle::Pointer,
            "{name}"
        );
        assert_eq!(
            cursor(&rt, &format!("{name}-off")),
            CursorStyle::Default,
            "{name}"
        );
    }
    assert_eq!(cursor(&rt, "plain"), CursorStyle::Pointer);
    assert_eq!(cursor(&rt, "plain-off"), CursorStyle::Default);
}

#[test]
fn fab_and_icon_button_follow_the_same_rule() {
    let rt = runtime();
    assert_eq!(cursor(&rt, "fab-on"), CursorStyle::Pointer);
    assert_eq!(cursor(&rt, "fab-off"), CursorStyle::Default);
    assert_eq!(cursor(&rt, "icon-on"), CursorStyle::Pointer);
    assert_eq!(cursor(&rt, "icon-off"), CursorStyle::Default);
}

#[test]
fn text_field_and_search_input_are_text() {
    let rt = runtime();
    assert_eq!(cursor(&rt, "field"), CursorStyle::Text);
    assert_eq!(cursor(&rt, "search-input"), CursorStyle::Text);
}

#[test]
fn other_controls_have_no_cursor() {
    let rt = runtime();
    for id in ["check", "radio", "switch"] {
        assert_eq!(cursor(&rt, id), CursorStyle::Default, "{id}");
    }
}
