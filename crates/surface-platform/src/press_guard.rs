//! Ignore the host textarea blur that the browser fires inside one canvas press.
#[derive(Default)]
pub(crate) struct PressGuard {
    pressing: bool,
}
impl PressGuard {
    /// Canvas pointerdown: a press is in progress.
    pub(crate) fn press(&mut self) {
        self.pressing = true;
    }
    /// Pointerup or pointercancel: the press is over.
    pub(crate) fn release(&mut self) {
        self.pressing = false;
    }
    /// A textarea blur is forwarded unless it lands inside a canvas press.
    pub(crate) fn forwards_blur(&self) -> bool {
        !self.pressing
    }
}

#[cfg(test)]
mod tests {
    use super::PressGuard;
    use surface::{Length, Node, Role, Style, TextMeasurer};
    use surface_app::{Application, Event, ImeEvent, Runtime, View};

    #[derive(Default)]
    struct App {
        text: String,
        count: u32,
    }
    #[derive(Clone)]
    enum Message {
        Text(String),
        Add,
    }
    impl Application for App {
        type Message = Message;
        fn update(&mut self, message: Message) {
            match message {
                Message::Text(text) => self.text = text,
                Message::Add => self.count += 1,
            }
        }
        fn view(&self) -> View<Message> {
            View::new(
                Node::new(Role::Group, "Form")
                    .id("form")
                    .styled(Style::column()),
            )
            .children([
                View::new(
                    Node::new(Role::TextInput, "Your name")
                        .id("name")
                        .text(&self.text)
                        .styled(Style::default().padding(12.)),
                )
                .on_input(Message::Text),
                View::new(
                    Node::new(Role::Button, "Add one")
                        .id("increment")
                        .text("Add one")
                        .styled(Style::default().height(Length::Px(32.))),
                )
                .on_press(Message::Add),
            ])
        }
    }

    struct Metrics;
    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
            (text.chars().count() as f32 * size, size * 1.5)
        }
    }

    /// Focus the name field, open a Zhuyin composition and commit ㄅ.
    fn committed() -> Runtime<App> {
        let mut r = Runtime::new(App::default()).unwrap();
        r.render(640., 480., &mut Metrics).unwrap();
        r.activate("name").unwrap();
        r.dispatch(Event::ImeFor {
            id: "name".into(),
            event: ImeEvent::Begin,
        })
        .unwrap();
        r.dispatch(Event::ImeFor {
            id: "name".into(),
            event: ImeEvent::Preedit {
                text: "ㄅ".into(),
                selection: Some((3, 3)),
            },
        })
        .unwrap();
        r.dispatch(Event::ImeFor {
            id: "name".into(),
            event: ImeEvent::Commit("ㄅ".into()),
        })
        .unwrap();
        r.render(640., 480., &mut Metrics).unwrap();
        r
    }

    /// The centre of the `increment` button, from the last laid-out frame.
    fn click_point(r: &Runtime<App>) -> (f32, f32) {
        let id = r.dom().by_id("increment").unwrap();
        let b = r.computed().unwrap().nodes[id.index()].bounds;
        (b.x + b.width / 2., b.y + b.height / 2.)
    }

    /// Safari order: the commit lands on mouse down, then canvas pointerdown,
    /// then textarea blur, then pointerup. With `guard_on`, the guard drops
    /// that blur while the press is in progress.
    fn replay(guard_on: bool) -> Runtime<App> {
        let mut r = committed();
        let (x, y) = click_point(&r);
        let mut guard = PressGuard::default();
        guard.press();
        r.dispatch(Event::PointerDown { x, y }).unwrap();
        if !guard_on || guard.forwards_blur() {
            r.dispatch(Event::Blur).unwrap();
        }
        guard.release();
        r.dispatch(Event::PointerUp { x, y }).unwrap();
        r
    }

    #[test]
    fn safari_order_activates_the_button_and_keeps_the_committed_value() {
        let r = replay(true);
        assert_eq!(r.app().count, 1);
        assert_eq!(r.focused(), Some("increment"));
        assert_eq!(r.app().text, "ㄅ");
    }

    #[test]
    fn safari_order_without_the_guard_loses_the_click() {
        let r = replay(false);
        assert_eq!(r.app().count, 0);
    }

    #[test]
    fn chrome_order_still_activates_and_keeps_the_value() {
        let mut r = committed();
        let (x, y) = click_point(&r);
        r.dispatch(Event::PointerDown { x, y }).unwrap();
        r.dispatch(Event::PointerUp { x, y }).unwrap();
        assert_eq!(r.app().count, 1);
        assert_eq!(r.focused(), Some("increment"));
        assert_eq!(r.app().text, "ㄅ");
    }

    #[test]
    fn blur_is_forwarded_outside_a_press() {
        let mut guard = PressGuard::default();
        assert!(guard.forwards_blur());
        guard.press();
        assert!(!guard.forwards_blur());
        guard.release();
        assert!(guard.forwards_blur());
    }

    #[test]
    fn pointercancel_release_reopens_the_blur_path() {
        let mut guard = PressGuard::default();
        guard.press();
        guard.release();
        assert!(guard.forwards_blur());
    }
}
