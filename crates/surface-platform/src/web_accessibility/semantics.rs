//! Project a rendered Surface node without adding browser-native actions.

use surface::{GridRole, Node, Role};
use surface_app::AccessibilityNode;
use wasm_bindgen::JsValue;
use web_sys::{Document, Element};

pub(super) fn element(
    document: &Document,
    source: &AccessibilityNode<'_>,
    modal: bool,
    leaf: bool,
) -> Result<Element, JsValue> {
    let node = source.node;
    let role = aria_role(node);
    let element = document.create_element(if node.role() == Role::Text {
        "span"
    } else {
        "div"
    })?;
    if let Some(role) = role {
        element.set_attribute("role", role)?;
    }
    if modal && role == Some("dialog") {
        element.set_attribute("aria-modal", "true")?;
    }
    if node.role() == Role::Heading {
        // MicroDOM does not distinguish heading levels.
        element.set_attribute("aria-level", "2")?;
    }

    let label = node.name();
    let reads_content = matches!(
        node.role(),
        Role::Text | Role::Heading | Role::Status | Role::Tooltip
    );
    if reads_content {
        let text = node.text_value().or_else(|| leaf.then_some(label));
        element.set_text_content(text);
    } else if role.is_some() && !label.trim().is_empty() {
        element.set_attribute("aria-label", label)?;
    }
    if node.is_disabled() {
        element.set_attribute("aria-disabled", "true")?;
    }
    if let Some(expanded) = node.expanded_state() {
        set_bool(&element, "aria-expanded", expanded)?;
    }
    if node.has_menu_popup() {
        element.set_attribute("aria-haspopup", "menu")?;
    }
    let current_page = node.is_current_page();
    if node.is_current_date() {
        element.set_attribute("aria-current", "date")?;
    } else if role != Some("gridcell") && current_page == Some(true) {
        element.set_attribute("aria-current", "page")?;
    }

    match role {
        Some("checkbox" | "menuitemcheckbox") => {
            let checked = if node.is_mixed() {
                "mixed"
            } else {
                bool_value(node.is_checked().unwrap_or(false))
            };
            element.set_attribute("aria-checked", checked)?;
        }
        Some("radio" | "switch") => {
            let checked = node.is_checked().or(node.is_selected()).unwrap_or(false);
            set_bool(&element, "aria-checked", checked)?;
        }
        Some("option") => {
            set_bool(
                &element,
                "aria-selected",
                node.is_checked().unwrap_or(false),
            )?;
        }
        Some("button") => {
            let pressed = match current_page {
                Some(_) => node.is_checked(),
                None => node.is_checked().or(node.is_selected()),
            };
            if let Some(pressed) = pressed {
                set_bool(&element, "aria-pressed", pressed)?;
            }
        }
        Some("tab") => {
            set_bool(
                &element,
                "aria-selected",
                node.is_selected().unwrap_or(false),
            )?;
        }
        Some("grid") if node.is_grid_multiselectable() => {
            element.set_attribute("aria-multiselectable", "true")?;
        }
        Some("gridcell") => {
            if let Some(selected) = node.is_selected() {
                set_bool(&element, "aria-selected", selected)?;
            }
        }
        Some("spinbutton") => {
            if let Some(range) = node.accessibility_adjustable() {
                element.set_attribute("aria-valuemin", &range.min.to_string())?;
                element.set_attribute("aria-valuemax", &range.max.to_string())?;
                if let Some(value) = range.value {
                    element.set_attribute("aria-valuenow", &value.to_string())?;
                }
                if let Some(text) = range
                    .value_text
                    .as_deref()
                    .filter(|text| !text.trim().is_empty())
                {
                    element.set_attribute("aria-valuetext", text)?;
                    if range.value.is_none() {
                        // Chrome exposes a default numeric zero and can drop
                        // value text when the authored current value is absent.
                        element.set_attribute("aria-description", text)?;
                    }
                }
            }
        }
        Some("slider" | "progressbar") => {
            if let Some(range) = node.range_value().filter(|range| range.is_valid()) {
                element.set_attribute("aria-valuemin", &range.min.to_string())?;
                element.set_attribute("aria-valuemax", &range.max.to_string())?;
                element.set_attribute("aria-valuenow", &range.value.to_string())?;
            }
        }
        Some("textbox") => {
            if let Some(options) = source.text_options {
                set_bool(&element, "aria-multiline", options.multiline)?;
                set_bool(&element, "aria-readonly", options.read_only)?;
            }
        }
        _ => {}
    }
    Ok(element)
}

fn aria_role(node: &Node) -> Option<&'static str> {
    if let Some(role) = node.accessibility_grid_role() {
        return Some(match role {
            GridRole::Grid => "grid",
            GridRole::Row => "row",
            GridRole::ColumnHeader => "columnheader",
            GridRole::GridCell => "gridcell",
        });
    }
    if node.accessibility_adjustable().is_some() {
        return Some("spinbutton");
    }
    Some(match node.role() {
        Role::Group if node.name().trim().is_empty() && !node.is_focusable() => return None,
        Role::Group | Role::Window => "group",
        Role::Heading => "heading",
        Role::Text => return None,
        Role::Button => "button",
        Role::Checkbox => "checkbox",
        Role::TextInput => "textbox",
        Role::Navigation => "navigation",
        Role::Radio if node.uses_web_listbox_semantics() => "option",
        Role::Radio => "radio",
        Role::RadioGroup if node.uses_web_listbox_semantics() => "listbox",
        Role::RadioGroup => "radiogroup",
        Role::Switch => "switch",
        Role::Slider => "slider",
        Role::Tab => "tab",
        Role::TabList => "tablist",
        Role::Menu => "menu",
        Role::MenuItem if node.is_checked().is_some() => "menuitemcheckbox",
        Role::MenuItem => "menuitem",
        Role::Dialog => "dialog",
        Role::Progress => "progressbar",
        Role::Status => "status",
        Role::Separator => "separator",
        Role::Tooltip => "tooltip",
        Role::List => "list",
        Role::ListItem => "listitem",
    })
}

fn bool_value(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn set_bool(element: &Element, attribute: &str, value: bool) -> Result<(), JsValue> {
    element.set_attribute(attribute, bool_value(value))
}
