//! Declarative DOM element builder and window mount.
//!
//! Provides [`Element`] (a tree-builder that stores a tag name, optional text,
//! style, attributes and children) and [`Window`] (a thin wrapper that borrows
//! a bliss-shell `View` to materialise an `Element` tree via [`Window::mount`]).

use anyrender_vello::VelloWindowRenderer;

use bliss_dom::{Attribute, DocumentMutator};
use bliss_dom::LocalName;
use bliss_dom::Namespace;
use bliss_dom::QualName;
use bliss_shell::View;
use bliss_traits::events::BlissKeyEvent;

use crate::style::Style;

// ---------------------------------------------------------------------------
// Element
// ---------------------------------------------------------------------------

/// A declarative DOM element builder.
///
/// `Element` stores a tree of nodes in an off-line structure.  It does *not*
/// touch the actual bliss‑dom slab until [`Window::mount`] materialises it.
///
/// - `Element::new("div")` creates an empty element.
/// - Builder methods (`set_text`, `set_style`, `append_child`, …) mutate the
///   local tree.
/// - `Element: Clone` is a **deep clone** of the declarative tree, which is
///   safe because the builder owns its children.
#[derive(Debug, Clone)]
pub struct Element {
    pub(crate) tag: String,
    pub(crate) text: Option<String>,
    pub(crate) style: Style,
    pub(crate) attributes: Vec<(String, String)>,
    pub(crate) children: Vec<Element>,
}

/// The HTML namespace URI.
const HTML_NS: &str = "http://www.w3.org/1999/xhtml";

impl Element {
    /// Create a new element with the given tag name.
    pub fn new(tag: &str) -> Self {
        Element {
            tag: tag.to_string(),
            text: None,
            style: Style::default(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Set the text content of this element.
    pub fn set_text(&mut self, text: &str) {
        self.text = Some(text.to_string());
    }

    /// Overwrite the style declaration.
    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    /// Append a child element.
    pub fn append_child(&mut self, child: Element) {
        self.children.push(child);
    }

    /// Set a named attribute.
    pub fn set_attribute(&mut self, name: &str, value: &str) {
        self.attributes.retain(|(k, _)| k != name);
        self.attributes.push((name.to_string(), value.to_string()));
    }

    /// Replace all children with a single text node containing `html`.
    ///
    /// This is a convenience used by the terminal renderer.  In the current
    /// implementation it sets the text content (plain text, not parsed HTML).
    pub fn set_inner_html(&mut self, html: &str) {
        self.text = Some(html.to_string());
        self.children.clear();
    }

    /// Build a `QualName` in the HTML namespace from a dynamic tag name.
    fn make_qname(tag: &str) -> QualName {
        QualName {
            prefix: None,
            ns: Namespace::from(HTML_NS),
            local: LocalName::from(tag),
        }
    }
}

// Helper — materialise an Element tree into a real bliss-dom document.
fn materialise_element(
    mutator: &mut DocumentMutator<'_>,
    el: &Element,
    parent_id: usize,
) {
    let style_inline = el.style.to_inline_css();

    let mut attrs: Vec<Attribute> = Vec::new();
    for (name, value) in &el.attributes {
        attrs.push(Attribute {
            name: Element::make_qname(name),
            value: value.clone().into(),
        });
    }
    if !style_inline.is_empty() {
        attrs.push(Attribute {
            name: Element::make_qname("style"),
            value: style_inline.into(),
        });
    }

    let node_id = mutator.create_element(Element::make_qname(&el.tag), attrs);
    mutator.append_children(parent_id, &[node_id]);

    if let Some(ref text) = el.text {
        let text_id = mutator.create_text_node(text);
        mutator.append_children(node_id, &[text_id]);
    }

    for child in &el.children {
        materialise_element(mutator, child, node_id);
    }
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

/// A window that can display an [`Element`] tree.
///
/// Thin wrapper that **borrows** a bliss-shell [`View<VelloWindowRenderer>`]
/// and adds a [`mount`](Self::mount) method to materialise an Element tree
/// into the window's document.
pub struct Window<'a> {
    view: &'a mut View<VelloWindowRenderer>,
}

impl<'a> Window<'a> {
    /// Borrow a `View` as a `Window`.
    pub fn from_view(view: &'a mut View<VelloWindowRenderer>) -> Self {
        Window { view }
    }

    /// Mount (or remount) an [`Element`] tree into this window's document.
    ///
    /// Finds the document body (node id 2), removes existing children, then
    /// materialises the Element tree.
    pub fn mount(&mut self, root: Element) {
        let mut inner = self.view.doc.inner_mut();
        let mut mutator = DocumentMutator::new(&mut *inner);

        // Find the root node to attach to.
        let mount_id = if mutator.element_name(2).is_some() { 2 } else { 0 };

        // Remove existing children.
        let existing = mutator.child_ids(mount_id);
        for child_id in existing {
            mutator.remove_node(child_id);
        }

        materialise_element(&mut mutator, &root, mount_id);
    }

    /// Register a callback for keydown events.
    ///
    /// Placeholder — keyboard routing is handled at the app level.
    pub fn on_keydown(&mut self, _cb: impl FnMut(&BlissKeyEvent)) {
        // Placeholder
    }

    /// Register a callback for resize events.
    ///
    /// Placeholder — resize is propagated through
    /// `ApplicationHandler::resized` → `CeceCodeApp::SurfaceResized`.
    pub fn on_resize(&mut self, _cb: impl FnMut(u32, u32) + 'static) {
        // Placeholder
    }

    /// Request a redraw of this window.
    ///
    /// Delegates to the underlying [`winit::window::Window`].
    pub fn request_redraw(&self) {
        self.view.window.request_redraw();
    }
}
