//! A component invocation: a named render function with type-erased props.

use std::rc::Rc;

use crate::element::Element;

/// Component = render function + typed props erased behind `Any`.
#[derive(Clone)]
pub struct Component {
    name: &'static str,
    render: ComponentFn,
    props: Rc<dyn std::any::Any>,
}

pub type ComponentFn = fn(&Rc<dyn std::any::Any>) -> Element;

impl Component {
    /// Pair a render function with its props. `render` receives `props`
    /// each time the component renders.
    pub const fn new(
        name: &'static str,
        render: ComponentFn,
        props: Rc<dyn std::any::Any>,
    ) -> Self {
        Self {
            name,
            render,
            props,
        }
    }

    /// The component's name. A snapshot records it on the component's node.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Render the component: call its render function with its props.
    pub fn render(&self) -> Element {
        (self.render)(&self.props)
    }
}
