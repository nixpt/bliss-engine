//! Serializable DOM mutations — the write counterpart to `document_tree_json`.
//!
//! An agent (or a test) emits a list of [`Mutation`]s and the document applies
//! them. Plain data with no dependencies, so a caller can map it from JSON (the
//! same choice the tree dump made). Read the result back with
//! [`BaseDocument::document_tree_json`], drive it with
//! [`EventDriver::synthesize_*`](crate::EventDriver), and step time with
//! [`BaseDocument::set_clock_ms`] — together, the headless read / write / drive /
//! time loop. See the `BLISS-AINATIVE-1` ticket.
//!
//! The document is not restyled or relaid out here; call
//! [`resolve`](BaseDocument::resolve) afterwards.

use crate::{BaseDocument, LocalName, QualName, ns};

/// One mutation to apply to a DOM node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mutation {
    /// Set an attribute on an element.
    SetAttribute { node: usize, name: String, value: String },
    /// Remove an attribute from an element.
    RemoveAttribute { node: usize, name: String },
    /// Set an inline CSS property (e.g. `width`, `opacity`).
    SetStyle { node: usize, property: String, value: String },
    /// Replace the text content of a text node.
    SetText { node: usize, text: String },
    /// Remove and drop a node (with its subtree).
    Remove { node: usize },
}

impl BaseDocument {
    /// Apply `mutations` in order, through a single `DocumentMutator`.
    ///
    /// Does not restyle or relayout; call [`resolve`](BaseDocument::resolve)
    /// afterwards. A mutation naming a missing node is ignored.
    pub fn apply_mutations(&mut self, mutations: &[Mutation]) {
        let mut mutator = self.mutate();
        for mutation in mutations {
            match mutation {
                Mutation::SetAttribute { node, name, value } => {
                    mutator.set_attribute(*node, html_attr(name), value);
                }
                Mutation::RemoveAttribute { node, name } => {
                    mutator.clear_attribute(*node, html_attr(name));
                }
                Mutation::SetStyle { node, property, value } => {
                    mutator.set_style_property(*node, property, value);
                }
                Mutation::SetText { node, text } => {
                    mutator.set_node_text(*node, text);
                }
                Mutation::Remove { node } => {
                    mutator.remove_and_drop_node(*node);
                }
            }
        }
    }
}

/// Build an HTML-namespaced attribute name from a runtime string (the caller's
/// `Mutation`s carry plain `String`s, so names are interned dynamically).
fn html_attr(name: &str) -> QualName {
    QualName::new(None, ns!(html), LocalName::from(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentConfig, qual_name};

    fn doc_with_text() -> (BaseDocument, usize, usize) {
        let mut doc = BaseDocument::new(DocumentConfig::default());
        let mut mutator = doc.mutate();
        let div = mutator.create_element(qual_name!("div"), vec![]);
        let text = mutator.create_text_node("hello");
        mutator.append_children(div, &[text]);
        mutator.append_children(0, &[div]);
        drop(mutator);
        (doc, div, text)
    }

    #[test]
    fn updates_attributes_and_text() {
        let (mut doc, div, text) = doc_with_text();
        doc.apply_mutations(&[
            Mutation::SetAttribute { node: div, name: "title".into(), value: "hi".into() },
            Mutation::SetStyle { node: div, property: "width".into(), value: "50px".into() },
            Mutation::SetText { node: text, text: "bye".into() },
        ]);
        let json = doc.document_tree_json();
        assert!(json.contains("\"title\":\"hi\""), "{json}");
        assert!(json.contains("\"text\":\"bye\""), "{json}");
    }

    #[test]
    fn remove_drops_the_node() {
        let (mut doc, div, _text) = doc_with_text();
        doc.apply_mutations(&[Mutation::Remove { node: div }]);
        let json = doc.document_tree_json();
        assert!(!json.contains("\"tag\":\"div\""), "{json}");
    }

    #[test]
    fn remove_attribute_round_trips() {
        let (mut doc, div, _text) = doc_with_text();
        doc.apply_mutations(&[
            Mutation::SetAttribute { node: div, name: "title".into(), value: "x".into() },
            Mutation::RemoveAttribute { node: div, name: "title".into() },
        ]);
        assert!(!doc.document_tree_json().contains("\"title\""));
    }
}
