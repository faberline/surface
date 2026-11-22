use surface::{Align, Insets, Length, Node, Role, Style, TextScaling};
use surface_app::{CursorStyle, View};

use crate::{shape::LayoutDirection, typography, IconButtonKind, Theme};

/// Names and current input for a Search View opened from an app bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppBarSearchOptions<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub input_label: &'a str,
    pub close_label: &'a str,
    pub clear_label: &'a str,
    pub query: &'a str,
    pub direction: LayoutDirection,
}

/// Caller-controlled results and dismissal messages.
pub struct AppBarSearchContent<M> {
    pub results: View<M>,
    pub on_close: M,
    pub on_clear: M,
}

impl Theme {
    /// Open a full-viewport modal Search View. The caller owns open state,
    /// query and results. Render it as a sibling above the app bar or page.
    ///
    /// Initial focus is in the editable search field. Escape respects IME
    /// composition and then closes the view. Dismissal restores prior focus.
    /// The view reserves `{id}-view`, `{id}-layer`, `{id}-header`, `{id}-input`,
    /// `{id}-close`, `{id}-clear` and `{id}-results`.
    pub fn app_bar_search_view<M: Clone>(
        &self,
        options: AppBarSearchOptions<'_>,
        content: AppBarSearchContent<M>,
        on_query: impl Fn(String) -> M + 'static,
    ) -> View<M> {
        let AppBarSearchContent {
            results,
            on_close,
            on_clear,
        } = content;
        let id = options.id;
        let rtl = options.direction == LayoutDirection::RightToLeft;
        let input = View::new(
            Node::new(Role::TextInput, options.input_label)
                .id(format!("{id}-input"))
                .text(options.query)
                .styled(
                    Style::column()
                        .width(Length::Px(0.))
                        .grow(1.)
                        .with_text_style(typography::BODY_LARGE)
                        .foreground(self.colors.on_surface),
                ),
        )
        .on_input(on_query)
        .cursor(CursorStyle::Text);
        let mut editor_children = vec![input];
        if !options.query.is_empty() {
            let clear = self
                .icon_button_with_text_scaling(
                    &format!("{id}-clear"),
                    options.clear_label,
                    ("×", TextScaling::Fixed),
                    IconButtonKind::Standard,
                    None,
                    false,
                    on_clear,
                )
                .accessible_activation()
                .protect_composition_on_press()
                .restore_focus_on_removal(format!("{id}-input"));
            editor_children.push(clear);
        }
        let mut editor = Node::new(Role::Group, "").styled(
            Style::row()
                .width(Length::Px(0.))
                .grow(1.)
                .align(Align::Center),
        );
        if rtl {
            editor = editor.reverse_row();
        }
        let close = self
            .icon_button_with_text_scaling(
                &format!("{id}-close"),
                options.close_label,
                (if rtl { "→" } else { "←" }, TextScaling::Fixed),
                IconButtonKind::Standard,
                None,
                false,
                on_close.clone(),
            )
            .accessible_activation()
            .protect_composition_on_press();
        let mut header_style = Style::row()
            .align(Align::Center)
            .gap(8.)
            .padding(4.)
            .background(self.colors.surface_container_high);
        header_style.min_height = 64.;
        let mut header = Node::new(Role::Group, "")
            .id(format!("{id}-header"))
            .styled(header_style);
        // The field is the first Tab stop. Put the Close button on the
        // physical leading side without changing the semantic order.
        if !rtl {
            header = header.reverse_row();
        }
        let header =
            View::new(header).children([View::new(editor).children(editor_children), close]);
        let results = View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-results"))
                .styled(Style::column().height(Length::Px(0.)).grow(1.).clip(true)),
        )
        .children([results])
        .accessibility_scroll_pages();
        let mut backdrop = Style::column()
            .fixed(Insets::all(0.))
            .background(self.colors.surface);
        backdrop.z_index = 1000;
        View::new(
            Node::new(Role::Dialog, options.title)
                .id(format!("{id}-view"))
                .tab_index(-1)
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.))
                        .background(self.colors.surface),
                ),
        )
        .children([header, self.divider(&format!("{id}-divider")), results])
        .modal_with_backdrop(
            Node::new(Role::Group, "")
                .id(format!("{id}-layer"))
                .styled(backdrop),
            on_close,
        )
    }
}
