use surface::*;

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

fn row(i: usize) -> Node {
    Node::new(Role::Group, format!("row{i}")).styled(Style::default().height(Length::Px(20.)))
}

/// A 100 x 200 scroller named "log" with `n` rows of 20 px.
fn fixture(n: usize, padding: f32, offset: f32) -> (MicroDom, NodeId) {
    let mut dom = MicroDom::new(Node::new(Role::Window, "App").styled(Style::column()));
    let mut style = Style::column()
        .width(Length::Px(100.))
        .height(Length::Px(200.))
        .padding(padding)
        .clip(true);
    style.scroll_y = offset;
    let log = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "log").id("log").styled(style),
        )
        .unwrap();
    for i in 0..n {
        dom.append(log, row(i)).unwrap();
    }
    (dom, log)
}

fn extent(dom: &MicroDom, id: NodeId) -> Option<(f32, f32)> {
    let computed = LayoutEngine::default()
        .compute(dom, 300., 400., &mut Metrics)
        .unwrap();
    scroll_extent(dom, &computed, id)
}

#[test]
fn extent_follows_the_row_count() {
    for (n, height) in [(5, 200.), (10, 200.), (30, 600.)] {
        let (dom, log) = fixture(n, 0., 0.);
        assert_eq!(extent(&dom, log), Some((100., height)), "n = {n}");
    }
}

#[test]
fn extent_ignores_the_offset() {
    let (a, log) = fixture(30, 0., 0.);
    let (b, _) = fixture(30, 0., 300.);
    assert_eq!(extent(&a, log), extent(&b, log));
    assert_eq!(extent(&b, log), Some((100., 600.)));
}

#[test]
fn extent_adds_padding_at_the_end() {
    let (dom, log) = fixture(30, 8., 0.);
    assert_eq!(extent(&dom, log), Some((100., 616.)));
}

#[test]
fn extent_skips_fixed_children() {
    let (mut dom, log) = fixture(30, 0., 0.);
    dom.append(
        log,
        Node::new(Role::Group, "layer").styled(Style::default().height(Length::Px(10.)).fixed(
            Insets {
                top: Length::Px(5000.),
                ..Insets::all(0.)
            },
        )),
    )
    .unwrap();
    assert_eq!(extent(&dom, log), Some((100., 600.)));
}

#[test]
fn extent_counts_a_clip_child_without_an_id() {
    let (mut dom, log) = fixture(0, 0., 0.);
    let inner = dom
        .append(
            log,
            Node::new(Role::Group, "inner")
                .styled(Style::column().height(Length::Px(500.)).clip(true)),
        )
        .unwrap();
    dom.append(inner, row(0)).unwrap();
    assert_eq!(extent(&dom, log), Some((100., 500.)));
}

#[test]
fn extent_of_an_empty_node_is_its_own_size() {
    let (dom, log) = fixture(0, 0., 0.);
    assert_eq!(extent(&dom, log), Some((100., 200.)));
}
