//! A small Material 3-inspired component set. Values remain renderer-independent.
use surface::{Align, Color, Length, Node, Role, Style};
use surface_app::View;
mod components;
pub mod motion;
pub mod shape;
mod tokens;
pub mod typography;
pub use components::{
    ButtonKind, CardKind, CheckValue, ChipKind, ChoiceState, FabColor, FabSize, IconButtonKind,
    LoadingKind, SegmentPosition,
};
pub use tokens::ColorScheme;

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub colors: ColorScheme,
    pub is_dark: bool,
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
        static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
        *THEME.get_or_init(|| Self::from_seed(Color::rgb(103, 80, 164), false))
    }
    pub fn dark() -> Self {
        static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
        *THEME.get_or_init(|| Self::from_seed(Color::rgb(103, 80, 164), true))
    }
    pub fn from_seed(seed: Color, dark: bool) -> Self {
        let colors = ColorScheme::from_seed(seed, dark);
        Self {
            colors,
            is_dark: dark,
            primary: colors.primary,
            on_primary: colors.on_primary,
            background: colors.surface,
            on_background: colors.on_surface,
            container: colors.surface_container,
            on_container: colors.on_surface,
            outline: colors.outline,
            disabled: tokens::blend(colors.surface, colors.on_surface, 0.12),
            tonal: colors.secondary_container,
            on_tonal: colors.on_secondary_container,
            input_container: colors.surface_container_highest,
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
                        .with_text_style(typography::HEADLINE_LARGE)
                        .foreground(self.on_background),
                ),
        )
    }
    pub fn text<M>(&self, id: &str, text: impl Into<String>) -> View<M> {
        let text = text.into();
        View::new(
            Node::new(Role::Text, text.clone())
                .id(id)
                .text(text)
                .styled(
                    Style::default()
                        .with_text_style(typography::BODY_LARGE)
                        .foreground(self.on_background),
                ),
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
                    .corners(shape::MEDIUM),
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
        self.button_with(id, label, ButtonKind::Filled, false, message)
    }
    pub fn tonal_button<M>(&self, id: &str, label: &str, message: M) -> View<M> {
        self.button_with(id, label, ButtonKind::Tonal, false, message)
    }
    pub fn disabled_button<M>(&self, id: &str, label: &str, message: M) -> View<M> {
        self.button_with(id, label, ButtonKind::Filled, true, message)
    }
    pub fn checkbox<M>(&self, id: &str, label: &str, checked: bool, message: M) -> View<M> {
        self.checkbox_with(
            id,
            label,
            if checked {
                CheckValue::Checked
            } else {
                CheckValue::Unchecked
            },
            false,
            message,
        )
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
                    .with_text_style(typography::BODY_LARGE)
                    .padding(16.)
                    .height(Length::Px(56.))
                    .corners(shape::EXTRA_SMALL_TOP)
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
