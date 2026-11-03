//! The host props of an intrinsic element.
//!
//! The fields are private: build props with `Props::default()` and the
//! `with_<field>` builders, and read them with the getter of the same name.
//! Every field starts unset (`None`, or `false` for `disabled`).

use crate::callback::Callback;

/// Host props shared by framework runtimes and renderers.
#[derive(Clone, Default, Debug)]
pub struct Props {
    class_name: Option<String>,
    style: Option<String>,
    on_click: Option<Callback<()>>,
    on_change: Option<Callback<String>>,
    on_checked_change: Option<Callback<bool>>,
    id: Option<String>,
    value: Option<String>,
    input_type: Option<String>,
    placeholder: Option<String>,
    checked: Option<bool>,
    aria_label: Option<String>,
    html_for: Option<String>,
    disabled: bool,
}

impl Props {
    /// Set the CSS class name.
    pub fn with_class_name(mut self, class_name: impl Into<String>) -> Self {
        self.class_name = Some(class_name.into());
        self
    }

    /// Set the inline style.
    pub fn with_style(mut self, style: impl Into<String>) -> Self {
        self.style = Some(style.into());
        self
    }

    /// Set the click handler.
    pub fn with_on_click(mut self, on_click: Callback<()>) -> Self {
        self.on_click = Some(on_click);
        self
    }

    /// Set the change handler; it receives the new value.
    pub fn with_on_change(mut self, on_change: Callback<String>) -> Self {
        self.on_change = Some(on_change);
        self
    }

    /// Set the checked-change handler; it receives the new checked state.
    pub fn with_on_checked_change(mut self, on_checked_change: Callback<bool>) -> Self {
        self.on_checked_change = Some(on_checked_change);
        self
    }

    /// Set the id. A snapshot uses it as the node's semantic id.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the value of an input or textarea.
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set the input type, such as `checkbox` or `text`.
    pub fn with_input_type(mut self, input_type: impl Into<String>) -> Self {
        self.input_type = Some(input_type.into());
        self
    }

    /// Set the placeholder text.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set the checked state.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Set the ARIA label.
    pub fn with_aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// Set the id of the element a label is for.
    pub fn with_html_for(mut self, html_for: impl Into<String>) -> Self {
        self.html_for = Some(html_for.into());
        self
    }

    /// Set whether the element is disabled.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn class_name(&self) -> Option<&str> {
        self.class_name.as_deref()
    }

    pub fn style(&self) -> Option<&str> {
        self.style.as_deref()
    }

    pub fn on_click(&self) -> Option<&Callback<()>> {
        self.on_click.as_ref()
    }

    pub fn on_change(&self) -> Option<&Callback<String>> {
        self.on_change.as_ref()
    }

    pub fn on_checked_change(&self) -> Option<&Callback<bool>> {
        self.on_checked_change.as_ref()
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    pub fn input_type(&self) -> Option<&str> {
        self.input_type.as_deref()
    }

    pub fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }

    pub fn checked(&self) -> Option<bool> {
        self.checked
    }

    pub fn aria_label(&self) -> Option<&str> {
        self.aria_label.as_deref()
    }

    pub fn html_for(&self) -> Option<&str> {
        self.html_for.as_deref()
    }

    pub fn disabled(&self) -> bool {
        self.disabled
    }
}
