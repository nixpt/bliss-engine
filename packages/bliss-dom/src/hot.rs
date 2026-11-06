//! Hot reload of a document's stylesheet (BLISS-AINATIVE-1).
//!
//! [`BaseDocument::hot_reload_style`] replaces the app's `<style>` element's CSS
//! in place and re-resolves, so a running shell can pick up an edited stylesheet
//! without rebuilding the document. Engine-side and dependency-free; the
//! transport (a file watcher, or the `control` channel) is the host's job.
//!
//! Markup (HTML) hot reload needs an `HtmlParserProvider` (from `bliss-html`),
//! which a `bliss-dom`-only test cannot supply — it is a follow-up.
//!
//! Note: verifying a hot-reloaded stylesheet's *computed* effect (a node's new
//! size) needs a fully-parsed document; the `bliss-dom` unit test here checks the
//! mechanism (the `<style>` text is replaced, the sheet is rebuilt, `resolve`
//! runs), not the painted result.

use crate::{BaseDocument, NodeData, local_name};

impl BaseDocument {
    /// Replace the CSS of the app's first `<style>` element and re-resolve.
    ///
    /// Returns `false` (and does nothing) when the document has no `<style>`
    /// element. The stylesheet is rebuilt through `upsert_stylesheet_for_node`,
    /// which marks the whole tree for a restyle, so already-styled elements pick
    /// up the new rules.
    pub fn hot_reload_style(&mut self, css: &str) -> bool {
        let Some(style_node) = self.first_style_node() else {
            return false;
        };

        // A <style> element's CSS lives in its text child; create one if empty.
        let text_child = self.nodes[style_node]
            .children
            .iter()
            .copied()
            .find(|&child| matches!(self.nodes[child].data, NodeData::Text(_)));

        let mut mutator = self.mutate();
        match text_child {
            Some(child) => mutator.set_node_text(child, css),
            None => {
                let child = mutator.create_text_node(css);
                mutator.append_children(style_node, &[child]);
            }
        }
        drop(mutator);

        self.upsert_stylesheet_for_node(style_node);
        let now = self.now_ms() / 1000.0;
        self.resolve(now);
        true
    }

    /// The id of the first `<style>` element, in document order.
    fn first_style_node(&self) -> Option<usize> {
        self.nodes
            .iter()
            .find_map(|(id, node)| node.data.is_element_with_tag_name(&local_name!("style")).then_some(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentConfig, qual_name};

    #[test]
    fn hot_reload_style_reinjects_the_stylesheet() {
        let mut doc = BaseDocument::new(DocumentConfig::default());
        let mut mutator = doc.mutate();
        let html = mutator.create_element(qual_name!("html"), vec![]);
        let head = mutator.create_element(qual_name!("head"), vec![]);
        let style = mutator.create_element(qual_name!("style"), vec![]);
        let css = mutator.create_text_node(".box { width: 50px; }");
        mutator.append_children(style, &[css]);
        mutator.append_children(head, &[style]);
        mutator.append_children(html, &[head]);
        mutator.append_children(0, &[html]);
        drop(mutator);

        let reloaded = ".box { width: 120px; }";
        assert!(doc.hot_reload_style(reloaded));
        // The <style> text was replaced; the sheet is rebuilt from it and the
        // document re-resolved (no panic). Computed-style assertions need an
        // HTML-parsed document — see the module note.
        assert_eq!(doc.nodes[style].text_content(), reloaded);
        // Idempotent: a second reload still finds and updates the element.
        assert!(doc.hot_reload_style(".box { width: 80px; }"));
        assert_eq!(doc.nodes[style].text_content(), ".box { width: 80px; }");
    }

    #[test]
    fn hot_reload_style_is_a_no_op_without_a_style_element() {
        let mut doc = BaseDocument::new(DocumentConfig::default());
        assert!(!doc.hot_reload_style(".x { width: 1px; }"));
    }
}
