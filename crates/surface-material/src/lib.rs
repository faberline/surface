//! A small Material 3-inspired component set. Values remain renderer-independent.
use surface::{Align, Color, Length, Node, Role, Style};
use surface_app::View;

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub primary: Color,
    pub on_primary: Color,
    pub background: Color,
    pub on_background: Color,
    pub container: Color,
    pub on_container: Color,
    pub outline: Color,
    pub disabled: Color,
    pub tonal: Color,
    pub on_tonal: Color,
    pub input_container: Color,
}
impl Theme {
    pub fn light() -> Self {
        Self {
            primary: Color::rgb(103, 80, 164),
            on_primary: Color::rgb(255, 255, 255),
            background: Color::rgb(254, 247, 255),
            on_background: Color::rgb(29, 27, 32),
            container: Color::rgb(243, 237, 247),
            on_container: Color::rgb(29, 27, 32),
            outline: Color::rgb(121, 116, 126),
            disabled: Color::rgb(223, 218, 227),
            tonal: Color::rgb(234, 221, 255),
            on_tonal: Color::rgb(33, 0, 93),
            input_container: Color::rgb(231, 224, 236),
        }
    }
    pub fn dark() -> Self {
        Self {
            primary: Color::rgb(208, 188, 255),
            on_primary: Color::rgb(56, 30, 114),
            background: Color::rgb(20, 18, 24),
            on_background: Color::rgb(230, 224, 233),
            container: Color::rgb(33, 31, 38),
            on_container: Color::rgb(230, 224, 233),
            outline: Color::rgb(147, 143, 153),
            disabled: Color::rgb(66, 62, 71),
            tonal: Color::rgb(79, 55, 139),
            on_tonal: Color::rgb(234, 221, 255),
            input_container: Color::rgb(49, 45, 56),
        }
    }
    pub fn page<M>(&self, children: impl IntoIterator<Item = View<M>>) -> View<M> {
        View::new(
            Node::new(Role::Window, "Application")
                .id("surface-page")
                .styled(
                    Style::column()
                        .padding(28.)
                        .gap(24.)
                        .clip(true)
                        .background(self.background)
                        .foreground(self.on_background),
                ),
        )
        .children(children)
    }
    pub fn heading<M>(&self, id: &str, text: impl Into<String>) -> View<M> {
        let text = text.into();
        View::new(
            Node::new(Role::Heading, text.clone())
                .id(id)
                .text(text)
                .styled(
                    Style::default()
                        .foreground(self.on_background)
                        .font_size(32.),
                ),
        )
    }
    pub fn text<M>(&self, id: &str, text: impl Into<String>) -> View<M> {
        let text = text.into();
        View::new(
            Node::new(Role::Text, text.clone())
                .id(id)
                .text(text)
                .styled(Style::default().foreground(self.on_background)),
        )
    }
    pub fn card<M>(&self, id: &str, children: impl IntoIterator<Item = View<M>>) -> View<M> {
        View::new(
            Node::new(Role::Group, "Card").id(id).styled(
                Style::column()
                    .padding(24.)
                    .gap(16.)
                    .background(self.container)
                    .foreground(self.on_container)
                    .radius(16.),
            ),
        )
        .children(children)
    }
    pub fn row<M>(&self, children: impl IntoIterator<Item = View<M>>) -> View<M> {
        View::new(
            Node::new(Role::Group, "Row")
                .styled(Style::row().gap(12.).align(Align::Center).wrap(true)),
        )
        .children(children)
    }
    pub fn button<M>(&self, id: &str, label: &str, message: M) -> View<M> {
        self.button_variant(id, label, message, false, false)
    }
    pub fn tonal_button<M>(&self, id: &str, label: &str, message: M) -> View<M> {
        self.button_variant(id, label, message, true, false)
    }
    pub fn disabled_button<M>(&self, id: &str, label: &str, message: M) -> View<M> {
        self.button_variant(id, label, message, false, true)
    }
    fn button_variant<M>(
        &self,
        id: &str,
        label: &str,
        message: M,
        tonal: bool,
        disabled: bool,
    ) -> View<M> {
        let background = if disabled {
            self.disabled
        } else if tonal {
            self.tonal
        } else {
            self.primary
        };
        let foreground = if disabled {
            self.outline
        } else if tonal {
            self.on_tonal
        } else {
            self.on_primary
        };
        let mut style = Style::default()
            .padding(12.)
            .height(Length::Px(48.))
            .radius(24.)
            .background(background)
            .foreground(foreground)
            .font_size(16.);
        style.min_width = 96.;
        View::new(
            Node::new(Role::Button, label)
                .id(id)
                .text(label)
                .disabled(disabled)
                .styled(style),
        )
        .on_press(message)
    }
    pub fn checkbox<M>(&self, id: &str, label: &str, checked: bool, message: M) -> View<M> {
        View::new(
            Node::new(Role::Checkbox, label)
                .id(id)
                .checked(checked)
                .styled(
                    Style::row()
                        .padding(12.)
                        .gap(12.)
                        .align(Align::Center)
                        .radius(12.)
                        .foreground(self.on_container),
                ),
        )
        .children([
            View::new(
                Node::new(Role::Text, "")
                    .text(if checked { "x" } else { "" })
                    .styled(
                        Style::default()
                            .width(Length::Px(24.))
                            .height(Length::Px(24.))
                            .radius(4.)
                            .padding(2.)
                            .background(if checked {
                                self.primary
                            } else {
                                self.input_container
                            })
                            .foreground(self.on_primary)
                            .font_size(14.),
                    ),
            ),
            View::new(
                Node::new(Role::Text, label)
                    .text(label)
                    .styled(Style::default().foreground(self.on_container)),
            ),
        ])
        .on_press(message)
    }
    /// Controlled text field with runtime caret, selection and IME support.
    pub fn text_field<M>(
        &self,
        id: &str,
        label: &str,
        value: &str,
        on_input: impl Fn(String) -> M + 'static,
    ) -> View<M> {
        View::new(
            Node::new(Role::TextInput, label).id(id).text(value).styled(
                Style::default()
                    .padding(16.)
                    .height(Length::Px(56.))
                    .radius(4.)
                    .background(self.input_container)
                    .foreground(self.on_container),
            ),
        )
        .on_input(on_input)
    }
}
impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}
