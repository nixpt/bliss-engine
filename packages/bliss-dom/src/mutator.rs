use std::collections::HashSet;
use std::mem;
use std::ops::{Deref, DerefMut};

use crate::document::make_device;
use crate::layout::damage::ALL_DAMAGE;
use crate::net::{ImageHandler, ResourceHandler, StylesheetHandler};
use crate::node::{CanvasData, NodeFlags, SpecialElementData};
use crate::util::ImageType;
use crate::{
    Attribute, BaseDocument, Document, ElementData, Node, NodeData, QualName, local_name, qual_name,
};
use bliss_traits::net::Request;
use bliss_traits::shell::Viewport;
use style::Atom;
use style::invalidation::element::restyle_hints::RestyleHint;
use style::stylesheets::OriginSet;

macro_rules! tag_and_attr {
    ($tag:tt, $attr:tt) => {
        (&local_name!($tag), &local_name!($attr))
    };
}

#[derive(Debug, Clone)]
pub enum AppendTextErr {
    /// The node is not a text node
    NotTextNode,
}

/// Operations that happen almost immediately, but are deferred within a
/// function for borrow-checker reasons.
enum SpecialOp {
    LoadImage(usize),
    LoadStylesheet(usize),
    UnloadStylesheet(usize),
    LoadCustomPaintSource(usize),
    ProcessButtonInput(usize),
}

pub struct DocumentMutator<'doc> {
    /// Document is public as an escape hatch, but users of this API should ideally avoid using it
    /// and prefer exposing additional functionality in DocumentMutator.
    pub doc: &'doc mut BaseDocument,

    eager_op_queue: Vec<SpecialOp>,

    // Tracked nodes for deferred processing when mutations have completed
    title_node: Option<usize>,
    style_nodes: HashSet<usize>,
    form_nodes: HashSet<usize>,

    /// Whether an element/attribute that affect animation status has been seen
    recompute_is_animating: bool,

    /// The (latest) node which has been mounted in and had autofocus=true, if any
    #[cfg(feature = "autofocus")]
    node_to_autofocus: Option<usize>,
}

impl Drop for DocumentMutator<'_> {
    fn drop(&mut self) {
        self.flush(); // Defined at bottom of file
    }
}

impl DocumentMutator<'_> {
    pub fn new<'doc>(doc: &'doc mut BaseDocument) -> DocumentMutator<'doc> {
        DocumentMutator {
            doc,
            eager_op_queue: Vec::new(),
            title_node: None,
            style_nodes: HashSet::new(),
            form_nodes: HashSet::new(),
            recompute_is_animating: false,
            #[cfg(feature = "autofocus")]
            node_to_autofocus: None,
        }
    }

    // Query methods

    pub fn node_has_parent(&self, node_id: usize) -> bool {
        self.doc.get_node(node_id).is_some_and(|n| n.parent.is_some())
    }

    pub fn previous_sibling_id(&self, node_id: usize) -> Option<usize> {
        self.doc.get_node(node_id)?.backward(1).map(|node| node.id)
    }

    pub fn next_sibling_id(&self, node_id: usize) -> Option<usize> {
        self.doc.get_node(node_id)?.forward(1).map(|node| node.id)
    }

    pub fn parent_id(&self, node_id: usize) -> Option<usize> {
        self.doc.get_node(node_id)?.parent
    }

    pub fn last_child_id(&self, node_id: usize) -> Option<usize> {
        self.doc.get_node(node_id)?.children.last().copied()
    }

    pub fn child_ids(&self, node_id: usize) -> Vec<usize> {
        self.doc.get_node(node_id).map(|n| n.children.clone()).unwrap_or_default()
    }

    pub fn element_name(&self, node_id: usize) -> Option<&QualName> {
        self.doc.get_node(node_id)?.element_data().map(|el| &el.name)
    }

    pub fn node_at_path(&self, start_node_id: usize, path: &[u8]) -> usize {
        let Some(mut current) = self.doc.get_node(start_node_id) else {
            return start_node_id;
        };
        for i in path {
            let Some(new_id) = current.children.get(*i as usize) else {
                return current.id;
            };
            let Some(next) = self.doc.get_node(*new_id) else {
                return current.id;
            };
            current = next;
        }
        current.id
    }

    // Node creation methods

    pub fn create_comment_node(&mut self) -> usize {
        self.doc.create_node(NodeData::Comment)
    }

    pub fn create_text_node(&mut self, text: &str) -> usize {
        self.doc.create_text_node(text)
    }

    /// Create a shadow root attached to the given host element.
    /// Returns the node ID of the new shadow root, or 0 if `host_id` is stale
    /// (0 is a documented failure sentinel — the document root is always at id 0
    /// only after construction; callers should treat 0 as "no shadow attached").
    pub fn attach_shadow(&mut self, host_id: usize) -> usize {
        if self.doc.get_node(host_id).is_none() {
            debug_assert!(false, "attach_shadow called with stale host_id={host_id}");
            return 0;
        }
        let shadow_root_id = self.doc.create_node(NodeData::ShadowRoot { host: host_id });
        let shadow_root = &mut self.doc.nodes[shadow_root_id];
        debug_assert_eq!(shadow_root.id, shadow_root_id);
        shadow_root.parent = Some(host_id);
        shadow_root.flags.insert(NodeFlags::IS_IN_DOCUMENT);
        self.doc.nodes[host_id].children.push(shadow_root_id);
        shadow_root_id
    }

    pub fn create_element(&mut self, name: QualName, attrs: Vec<Attribute>) -> usize {
        let mut data = ElementData::new(name, attrs);
        data.flush_style_attribute(self.doc.guard(), &self.doc.url.url_extra_data());

        let id = self.doc.create_node(NodeData::Element(data));
        // `id` was just returned by `create_node`, so the slab entry exists by
        // structural invariant. `.expect` documents the violation rather than
        // a bare `.unwrap()` and matches the rest of the file's expectations
        // (`element_data_mut().expect("Not an element")` style was used pre-D-2b).
        let node = self
            .doc
            .get_node(id)
            .expect("create_node returned stale id");

        // Initialise style data
        *node.stylo_element_data.borrow_mut() = Some(style::data::ElementData {
            damage: ALL_DAMAGE,
            ..Default::default()
        });

        id
    }

    /// Forwards `BaseDocument::deep_clone_node` (D-2c).
    /// Returns `None` if `node_id` is stale or any descendant goes stale
    /// during the recursive clone. The slab is left pristine on the stale-root
    /// path because `BaseDocument::deep_clone_node` validates before allocating.
    pub fn deep_clone_node(&mut self, node_id: usize) -> usize {
        self.doc.deep_clone_node(node_id)
    }

    // Node mutation methods

    pub fn set_node_text(&mut self, node_id: usize, value: &str) {
        let Some(node) = self.doc.get_node_mut(node_id) else {
            return;
        };

        let text = match node.data {
            NodeData::Text(ref mut text) => text,
            // TODO: otherwise this is basically element.textContent which is a bit different - need to parse as html
            _ => return,
        };

        let changed = text.content != value;
        if changed {
            text.content.clear();
            text.content.push_str(value);
            node.insert_damage(ALL_DAMAGE);
            // Mark ancestors dirty so the style traversal visits this subtree.
            // Without this, the traversal may skip nodes with pending damage.
            node.mark_ancestors_dirty();
            let parent_id = node.parent;

            // Also insert damage on the parent element, since text content changes
            // affect the parent's layout (text may wrap differently, change size, etc.)
            if let Some(parent_id) = parent_id {
                if let Some(parent) = self.doc.get_node_mut(parent_id) {
                    parent.insert_damage(ALL_DAMAGE);
                } else {
                    debug_assert!(false, "set_node_text parent_id={parent_id} stale after get_node_mut on child");
                }
            }

            self.maybe_record_node(parent_id);
        }
    }

    pub fn append_text_to_node(&mut self, node_id: usize, text: &str) -> Result<(), AppendTextErr> {
        let Some(node) = self.doc.get_node_mut(node_id) else {
            return Err(AppendTextErr::NotTextNode);
        };
        node.insert_damage(ALL_DAMAGE);
        node.mark_ancestors_dirty();
        match node.text_data_mut() {
            Some(data) => {
                data.content += text;
                Ok(())
            }
            None => Err(AppendTextErr::NotTextNode),
        }
    }

    pub fn add_attrs_if_missing(&mut self, node_id: usize, attrs: Vec<Attribute>) {
        let Some(node) = self.doc.get_node_mut(node_id) else {
            return;
        };
        node.insert_damage(ALL_DAMAGE);
        let Some(element_data) = node.element_data_mut() else {
            debug_assert!(false, "add_attrs_if_missing called on non-element node_id={node_id}");
            return;
        };

        let existing_names = element_data
            .attrs
            .iter()
            .map(|e| e.name.clone())
            .collect::<HashSet<_>>();

        for attr in attrs
            .into_iter()
            .filter(|attr| !existing_names.contains(&attr.name))
        {
            self.set_attribute(node_id, attr.name, &attr.value);
        }
    }

    pub fn set_attribute(&mut self, node_id: usize, name: QualName, value: &str) {
        if self.doc.nodes.get(node_id).is_none() {
            return;
        }
        self.doc.snapshot_node(node_id);

        let node = &mut self.doc.nodes[node_id];
        if let Some(data) = &mut *node.stylo_element_data.borrow_mut() {
            // Attribute changes only affect this element's selector matching,
            // not its children's. RESTYLE_SELF is sufficient.
            data.hint |= RestyleHint::RESTYLE_SELF;
            data.damage.insert(ALL_DAMAGE);
        }

        // The parent needs full subtree restyle to ensure the traversal
        // visits this subtree (without ElementSelectorFlags we can't
        // narrow this further — descendant selectors may target this node).
        let parent = node.parent;
        if let Some(parent_id) = parent {
            let parent = &mut self.doc.nodes[parent_id];
            if let Some(data) = &mut *parent.stylo_element_data.borrow_mut() {
                data.hint |= RestyleHint::restyle_subtree();
            }
        }

        // Mark ancestors dirty so the style traversal visits this subtree.
        // Without this, the traversal may skip nodes with pending RestyleHint/damage
        // because it uses dirty_descendants flags to determine which subtrees to visit.
        self.doc.nodes[node_id].mark_ancestors_dirty();

        let node = &mut self.doc.nodes[node_id];

        let NodeData::Element(ref mut element) = node.data else {
            return;
        };

        element.attrs.set(name.clone(), value);

        let tag = &element.name.local;
        let attr = &name.local;

        if *attr == local_name!("id") {
            element.id = Some(Atom::from(value))
        }

        if *attr == local_name!("value") {
            if let Some(input_data) = element.text_input_data_mut() {
                // Update text input value
                input_data.set_text(
                    &mut self.doc.font_ctx.lock().unwrap_or_else(|e| e.into_inner()),
                    &mut self.doc.layout_ctx,
                    value,
                );
            }
            return;
        }

        if *attr == local_name!("style") {
            element.flush_style_attribute(&self.doc.guard, &self.doc.url.url_extra_data());
            return;
        }

        if *attr == local_name!("disabled") && element.can_be_disabled() {
            node.disable();
            return;
        }

        // If node if not in the document, then don't apply any special behaviours
        // and simply set the attribute value
        if !node.flags.is_in_document() {
            return;
        }

        if (tag, attr) == tag_and_attr!("input", "checked") {
            set_input_checked_state(element, value.to_string());
        } else if (tag, attr) == tag_and_attr!("img", "src") {
            self.load_image(node_id);
        } else if (tag, attr) == tag_and_attr!("canvas", "src") {
            self.load_custom_paint_src(node_id);
        } else if (tag, attr) == tag_and_attr!("link", "href") {
            self.load_linked_stylesheet(node_id);
        }
    }

    pub fn clear_attribute(&mut self, node_id: usize, name: QualName) {
        if self.doc.nodes.get(node_id).is_none() {
            return;
        }
        self.doc.snapshot_node(node_id);

        let node = &mut self.doc.nodes[node_id];

        let mut stylo_element_data = node.stylo_element_data.borrow_mut();
        if let Some(data) = &mut *stylo_element_data {
            // Clearing an attribute only affects this element's selector matching.
            data.hint |= RestyleHint::RESTYLE_SELF;
            data.damage.insert(ALL_DAMAGE);
        }
        drop(stylo_element_data);

        // Mark ancestors dirty so the style traversal visits this subtree.
        // Without this, the traversal may skip nodes with pending RestyleHint/damage.
        node.mark_ancestors_dirty();

        let Some(element) = node.element_data_mut() else {
            return;
        };

        let removed_attr = element.attrs.remove(&name);
        let had_attr = removed_attr.is_some();
        if !had_attr {
            return;
        }

        if name.local == local_name!("id") {
            element.id = None;
        }

        // Update text input value
        if name.local == local_name!("value") {
            if let Some(input_data) = element.text_input_data_mut() {
                input_data.set_text(
                    &mut self.doc.font_ctx.lock().unwrap_or_else(|e| e.into_inner()),
                    &mut self.doc.layout_ctx,
                    "",
                );
            }
        }

        let tag = &element.name.local;
        let attr = &name.local;

        if *attr == local_name!("disabled") && element.can_be_disabled() {
            node.enable();
            return;
        }

        if *attr == local_name!("style") {
            element.flush_style_attribute(&self.doc.guard, &self.doc.url.url_extra_data());
        } else if (tag, attr) == tag_and_attr!("canvas", "src") {
            self.recompute_is_animating = true;
        } else if (tag, attr) == tag_and_attr!("link", "href") {
            self.unload_stylesheet(node_id);
        }
    }

    pub fn set_style_property(&mut self, node_id: usize, name: &str, value: &str) {
        self.doc.set_style_property(node_id, name, value)
    }

    pub fn remove_style_property(&mut self, node_id: usize, name: &str) {
        self.doc.remove_style_property(node_id, name)
    }

    pub fn set_sub_document(&mut self, node_id: usize, sub_document: Box<dyn Document>) {
        self.doc.set_sub_document(node_id, sub_document)
    }

    pub fn remove_sub_document(&mut self, node_id: usize) {
        self.doc.remove_sub_document(node_id)
    }

    /// Remove the node from it's parent but don't drop it
    pub fn remove_node(&mut self, node_id: usize) {
        let Some(node) = self.doc.get_node_mut(node_id) else {
            return;
        };

        // Update child_idx values
        if let Some(parent_id) = node.parent.take() {
            if let Some(parent) = self.doc.get_node_mut(parent_id) {
                parent.insert_damage(ALL_DAMAGE);
                // Mark ancestors dirty so the style traversal visits this subtree.
                parent.mark_ancestors_dirty();
                parent.children.retain(|id| *id != node_id);
                self.maybe_record_node(parent_id);
            } else {
                debug_assert!(false, "remove_node: node {node_id} reports stale parent {parent_id}");
            }
        }

        self.process_removed_subtree(node_id);
    }

    pub fn remove_and_drop_node(&mut self, node_id: usize) -> Option<Node> {
        if self.doc.get_node(node_id).is_none() {
            return None;
        }
        self.process_removed_subtree(node_id);

        let node = self.doc.drop_node_ignoring_parent(node_id);

        // Update child_idx values
        if let Some(parent_id) = node.as_ref().and_then(|node| node.parent) {
            let Some(parent) = self.doc.get_node_mut(parent_id) else {
                debug_assert!(false, "remove_and_drop_node: node {node_id} reports stale parent {parent_id}");
                return node;
            };
            parent.insert_damage(ALL_DAMAGE);
            let parent_is_in_doc = parent.flags.is_in_document();

            // When removing a child, the parent's own styles don't change —
            // only its children set does. RESTYLE_SELF is sufficient for the
            // old parent since we just need to trigger a traversal that will
            // pick up the removed child's absence from the layout tree.
            if parent_is_in_doc {
                if let Some(data) = &mut *parent.stylo_element_data.borrow_mut() {
                    data.hint |= RestyleHint::RESTYLE_SELF;
                }
                // Mark ancestors dirty so the style traversal visits this subtree.
                parent.mark_ancestors_dirty();
            }

            parent.children.retain(|id| *id != node_id);
            self.maybe_record_node(parent_id);
        }

        node
    }

    pub fn remove_and_drop_all_children(&mut self, node_id: usize) {
        let Some(parent) = self.doc.get_node_mut(node_id) else {
            return;
        };
        let parent_is_in_doc = parent.flags.is_in_document();

        // TODO: make this fine grained / conditional based on ElementSelectorFlags
        if parent_is_in_doc {
            if let Some(data) = &mut *parent.stylo_element_data.borrow_mut() {
                data.hint |= RestyleHint::restyle_subtree();
            }
            // Mark ancestors dirty so the style traversal visits this subtree.
            parent.mark_ancestors_dirty();
        }

        let children = mem::take(&mut parent.children);
        for child_id in children {
            self.process_removed_subtree(child_id);
            let _ = self.doc.drop_node_ignoring_parent(child_id);
        }
        self.maybe_record_node(node_id);
    }

    // Tree mutation methods
    pub fn remove_node_if_unparented(&mut self, node_id: usize) {
        if let Some(node) = self.doc.get_node(node_id) {
            if node.parent.is_none() {
                self.remove_and_drop_node(node_id);
            }
        }
    }

    /// Remove all of the children from old_parent_id and append them to new_parent_id
    pub fn append_children(&mut self, parent_id: usize, child_ids: &[usize]) {
        self.add_children_to_parent(parent_id, child_ids, &|parent, child_ids| {
            parent.children.extend_from_slice(child_ids);
        });
    }

    pub fn insert_nodes_before(&mut self, anchor_node_id: usize, new_node_ids: &[usize]) {
        let Some(parent_id) = self.doc.get_node(anchor_node_id).and_then(|n| n.parent) else {
            return;
        };
        self.add_children_to_parent(parent_id, new_node_ids, &|parent, child_ids| {
            // If the anchor was removed from the parent between the get_node
            // check above and the closure, fall back to appending rather than
            // unwrapping on an empty index.
            let Some(node_child_idx) = parent.index_of_child(anchor_node_id) else {
                parent.children.extend_from_slice(child_ids);
                return;
            };
            parent
                .children
                .splice(node_child_idx..node_child_idx, child_ids.iter().copied());
        });
    }

    fn add_children_to_parent(
        &mut self,
        parent_id: usize,
        child_ids: &[usize],
        insert_children_fn: &dyn Fn(&mut Node, &[usize]),
    ) {
        // Validate every id we will touch before mutating any of them, so the
        // function is all-or-nothing: if any id is stale we leave the document
        // untouched rather than half-applying parent damage and reparent hints.
        if self.doc.get_node(parent_id).is_none() {
            debug_assert!(
                false,
                "add_children_to_parent called with stale parent_id={parent_id}"
            );
            return;
        }
        if !child_ids
            .iter()
            .all(|&id| self.doc.get_node(id).is_some())
        {
            debug_assert!(
                false,
                "add_children_to_parent: at least one child_id is stale; aborting"
            );
            return;
        }
        let new_parent = &mut self.doc.nodes[parent_id];
        new_parent.insert_damage(ALL_DAMAGE);
        let new_parent_is_in_doc = new_parent.flags.is_in_document();

        // TODO: make this fine grained / conditional based on ElementSelectorFlags
        if new_parent_is_in_doc {
            if let Some(data) = &mut *new_parent.stylo_element_data.borrow_mut() {
                data.hint |= RestyleHint::restyle_subtree();
            }
            // Mark ancestors dirty so the style traversal visits this subtree.
            new_parent.mark_ancestors_dirty();
        }

        insert_children_fn(new_parent, child_ids);

        for child_id in child_ids.iter().copied() {
            let Some(child) = self.doc.get_node_mut(child_id) else {
                debug_assert!(false, "add_children_to_parent: stale child_id={child_id}");
                continue;
            };
            let old_parent_id = child.parent.replace(parent_id);

            let child_was_in_doc = child.flags.is_in_document();
            if new_parent_is_in_doc != child_was_in_doc {
                self.process_added_subtree(child_id);
            }

            if let Some(old_parent_id) = old_parent_id {
                let Some(old_parent) = self.doc.get_node_mut(old_parent_id) else {
                    debug_assert!(false, "add_children_to_parent: stale old_parent_id={old_parent_id}");
                    continue;
                };
                old_parent.insert_damage(ALL_DAMAGE);

                // When reparenting a child, the old parent's own styles don't
                // change — only its children set does.
                if child_was_in_doc {
                    if let Some(data) = &mut *old_parent.stylo_element_data.borrow_mut() {
                        data.hint |= RestyleHint::RESTYLE_SELF;
                    }
                    // Mark ancestors dirty so the style traversal visits this subtree.
                    old_parent.mark_ancestors_dirty();
                }

                old_parent.children.retain(|id| *id != child_id);
                self.maybe_record_node(old_parent_id);
            }
        }

        self.maybe_record_node(parent_id);
    }

    // Tree mutation methods (that defer to other methods)
    pub fn insert_nodes_after(&mut self, anchor_node_id: usize, new_node_ids: &[usize]) {
        match self.next_sibling_id(anchor_node_id) {
            Some(id) => self.insert_nodes_before(id, new_node_ids),
            None => match self.parent_id(anchor_node_id) {
                Some(parent_id) => self.append_children(parent_id, new_node_ids),
                None => {
                    debug_assert!(false, "insert_nodes_after called with orphan anchor_node_id={anchor_node_id}");
                }
            },
        }
    }

    pub fn reparent_children(&mut self, old_parent_id: usize, new_parent_id: usize) {
        let Some(old_parent) = self.doc.get_node_mut(old_parent_id) else {
            return;
        };
        let child_ids = std::mem::take(&mut old_parent.children);
        self.maybe_record_node(old_parent_id);
        self.append_children(new_parent_id, &child_ids);
    }

    pub fn replace_node_with(&mut self, anchor_node_id: usize, new_node_ids: &[usize]) {
        if self.doc.get_node(anchor_node_id).is_none() {
            return;
        }
        self.insert_nodes_before(anchor_node_id, new_node_ids);
        self.remove_node(anchor_node_id);
    }
}

impl<'doc> DocumentMutator<'doc> {
    pub fn flush(&mut self) {
        if self.recompute_is_animating {
            self.doc.has_canvas = self.doc.compute_has_canvas();
        }

        if let Some(id) = self.title_node {
            if self.doc.get_node(id).is_none() {
                debug_assert!(
                    false,
                    "flush: stale title_node={id} (removed before flush); clearing"
                );
                self.title_node = None;
            } else if let Some(node) = self.doc.get_node(id) {
                let title = node.text_content();
                self.doc.shell_provider.set_window_title(title);
            }
        }

        // Add/Update inline stylesheets (<style> elements)
        for id in self.style_nodes.drain() {
            if self.doc.get_node(id).is_none() {
                debug_assert!(
                    false,
                    "flush: stale style_node={id} (removed before flush); skipped"
                );
                continue;
            }
            self.doc.process_style_element(id);
        }

        for id in self.form_nodes.drain() {
            if self.doc.get_node(id).is_none() {
                debug_assert!(
                    false,
                    "flush: stale form_node={id} (removed before flush); skipped"
                );
                continue;
            }
            self.doc.reset_form_owner(id);
        }

        #[cfg(feature = "autofocus")]
        if let Some(node_id) = self.node_to_autofocus.take() {
            if self.doc.get_node(node_id).is_some() {
                self.doc.set_focus_to(node_id);
            }
        }
    }

    pub fn set_inner_html(&mut self, node_id: usize, html: &str) {
        if self.doc.get_node(node_id).is_none() {
            return;
        }
        self.remove_and_drop_all_children(node_id);
        self.doc
            .html_parser_provider
            .clone()
            .parse_inner_html(self, node_id, html);
    }

    fn flush_eager_ops(&mut self) {
        let mut ops = mem::take(&mut self.eager_op_queue);
        for op in ops.drain(0..) {
            match op {
                SpecialOp::LoadImage(node_id) => self.load_image(node_id),
                SpecialOp::LoadStylesheet(node_id) => self.load_linked_stylesheet(node_id),
                SpecialOp::UnloadStylesheet(node_id) => self.unload_stylesheet(node_id),
                SpecialOp::LoadCustomPaintSource(node_id) => self.load_custom_paint_src(node_id),
                SpecialOp::ProcessButtonInput(node_id) => self.process_button_input(node_id),
            }
        }

        // Queue is empty, but put Vec back anyway so allocation can be reused.
        self.eager_op_queue = ops;
    }

    fn process_added_subtree(&mut self, node_id: usize) {
        self.doc.iter_subtree_mut(node_id, |node_id, doc| {
            let node = &mut doc.nodes[node_id];
            node.flags.set(NodeFlags::IS_IN_DOCUMENT, true);
            node.insert_damage(ALL_DAMAGE);

            // If the node has an "id" attribute, store it in the ID map.
            if let Some(id_attr) = node.attr(local_name!("id")) {
                doc.nodes_to_id.insert(id_attr.to_string(), node_id);
            }

            let NodeData::Element(ref mut element) = node.data else {
                return;
            };

            // Custom post-processing by element tag name
            let tag = element.name.local.as_ref();
            match tag {
                "title" => self.title_node = Some(node_id),
                "link" => self.eager_op_queue.push(SpecialOp::LoadStylesheet(node_id)),
                "img" => self.eager_op_queue.push(SpecialOp::LoadImage(node_id)),
                "canvas" => self
                    .eager_op_queue
                    .push(SpecialOp::LoadCustomPaintSource(node_id)),
                "style" => {
                    self.style_nodes.insert(node_id);
                }
                "button" | "fieldset" | "input" | "select" | "textarea" | "object" | "output" => {
                    self.eager_op_queue
                        .push(SpecialOp::ProcessButtonInput(node_id));
                    self.form_nodes.insert(node_id);
                }
                _ => {}
            }

            #[cfg(feature = "autofocus")]
            if node.is_focussable() {
                if let NodeData::Element(ref element) = node.data {
                    if let Some(value) = element.attr(local_name!("autofocus")) {
                        if value == "true" {
                            self.node_to_autofocus = Some(node_id);
                        }
                    }
                }
            }
        });

        self.flush_eager_ops();
    }

    fn process_removed_subtree(&mut self, node_id: usize) {
        self.doc.iter_subtree_mut(node_id, |node_id, doc| {
            let node = &mut doc.nodes[node_id];
            node.flags.set(NodeFlags::IS_IN_DOCUMENT, false);

            // If the node has an "id" attribute remove it from the ID map.
            if let Some(id_attr) = node.attr(local_name!("id")) {
                doc.nodes_to_id.remove(id_attr);
            }

            let NodeData::Element(ref mut element) = node.data else {
                return;
            };

            match &element.special_data {
                SpecialElementData::SubDocument(_) => {}
                SpecialElementData::Stylesheet(_) => self
                    .eager_op_queue
                    .push(SpecialOp::UnloadStylesheet(node_id)),
                SpecialElementData::Image(_) => {}
                SpecialElementData::Canvas(_) => {
                    self.recompute_is_animating = true;
                }
                SpecialElementData::TableRoot(_) => {}
                SpecialElementData::TextInput(_) => {}
                SpecialElementData::CheckboxInput(_) => {}
                #[cfg(feature = "file_input")]
                SpecialElementData::FileInput(_) => {}
                SpecialElementData::None => {}
            }
        });

        self.flush_eager_ops();
    }

    fn maybe_record_node(&mut self, node_id: impl Into<Option<usize>>) {
        let Some(node_id) = node_id.into() else {
            return;
        };

        let Some(node) = self.doc.get_node(node_id) else {
            debug_assert!(false, "maybe_record_node called with stale node_id={node_id}");
            return;
        };
        let Some(tag_name) = node.data.downcast_element().map(|elem| &elem.name.local) else {
            return;
        };

        match tag_name.as_ref() {
            "title" => self.title_node = Some(node_id),
            "style" => {
                self.style_nodes.insert(node_id);
            }
            _ => {}
        }
    }

    fn load_linked_stylesheet(&mut self, target_id: usize) {
        let Some(node) = self.doc.get_node(target_id) else {
            debug_assert!(false, "load_linked_stylesheet: stale target_id={target_id}");
            return;
        };

        let rel_attr = node.attr(local_name!("rel"));
        let href_attr = node.attr(local_name!("href"));

        let (Some(rels), Some(href)) = (rel_attr, href_attr) else {
            return;
        };
        if !rels.split_ascii_whitespace().any(|rel| rel == "stylesheet") {
            return;
        }

        let Some(url) = self.doc.resolve_url(href) else {
            return;
        };
        self.doc.net_provider.fetch(
            self.doc.id(),
            Request::get(url.clone()),
            ResourceHandler::boxed(
                self.doc.tx.clone(),
                self.doc.id(),
                Some(node.id),
                self.doc.shell_provider.clone(),
                StylesheetHandler {
                    source_url: url,
                    guard: self.doc.guard.clone(),
                    net_provider: self.doc.net_provider.clone(),
                },
            ),
        );
    }

    fn unload_stylesheet(&mut self, node_id: usize) {
        let Some(node) = self.doc.get_node_mut(node_id) else {
            debug_assert!(false, "unload_stylesheet: stale node_id={node_id}");
            return;
        };
        let Some(element) = node.element_data_mut() else {
            debug_assert!(false, "unload_stylesheet: node {node_id} is not an element");
            return;
        };
        let SpecialElementData::Stylesheet(stylesheet) = element.special_data.take() else {
            debug_assert!(false, "unload_stylesheet: node {node_id} carries non-Stylesheet special_data");
            return;
        };

        let guard = self.doc.guard.read();
        self.doc.stylist.remove_stylesheet(stylesheet, &guard);
        self.doc
            .stylist
            .force_stylesheet_origins_dirty(OriginSet::all());

        self.doc.nodes_to_stylesheet.remove(&node_id);
    }

    fn load_image(&mut self, target_id: usize) {
        let Some(node) = self.doc.get_node(target_id) else {
            debug_assert!(false, "load_image: stale target_id={target_id}");
            return;
        };
        if let Some(raw_src) = node.attr(local_name!("src")) {
            if !raw_src.is_empty() {
                let Some(src) = self.doc.resolve_url(raw_src) else {
                    return;
                };
                let src_string = src.as_str();

                // Check cache first (clone out so the image_cache borrow is released
                // before the subsequent get_node_mut borrow takes effect).
                if let Some(cached_image) = self.doc.image_cache.get(src_string).cloned() {
                    #[cfg(feature = "tracing")]
                    tracing::info!("Loading image {src_string} from cache");
                    if let Some(node) = self.doc.get_node_mut(target_id) {
                        if let Some(element_data) = node.element_data_mut() {
                            element_data.special_data =
                                SpecialElementData::Image(Box::new(cached_image));
                        } else {
                            debug_assert!(false, "load_image (cache): node {target_id} not element");
                            return;
                        }
                        node.cache.clear();
                        node.insert_damage(ALL_DAMAGE);
                    }
                    return;
                }

                // Check if there's already a pending request for this URL
                if let Some(waiting_list) = self.doc.pending_images.get_mut(src_string) {
                    #[cfg(feature = "tracing")]
                    tracing::info!("Image {src_string} already pending, queueing node {target_id}");
                    waiting_list.push((target_id, ImageType::Image));
                    return;
                }

                // Start fetch and track as pending
                #[cfg(feature = "tracing")]
                tracing::info!("Fetching image {src_string}");
                self.doc
                    .pending_images
                    .insert(src_string.to_string(), vec![(target_id, ImageType::Image)]);

                self.doc.net_provider.fetch(
                    self.doc.id(),
                    Request::get(src),
                    ResourceHandler::boxed(
                        self.doc.tx.clone(),
                        self.doc.id(),
                        None, // Don't pass node_id, we'll handle it via pending_images
                        self.doc.shell_provider.clone(),
                        ImageHandler::new(ImageType::Image),
                    ),
                );
            }
        }
    }

    fn load_custom_paint_src(&mut self, target_id: usize) {
        let Some(node) = self.doc.get_node_mut(target_id) else {
            debug_assert!(false, "load_custom_paint_src: stale target_id={target_id}");
            return;
        };
        if let Some(raw_src) = node.attr(local_name!("src")) {
            if let Ok(custom_paint_source_id) = raw_src.parse::<u64>() {
                self.recompute_is_animating = true;
                let canvas_data = SpecialElementData::Canvas(CanvasData {
                    custom_paint_source_id,
                });
                if let Some(element_data) = node.element_data_mut() {
                    element_data.special_data = canvas_data;
                } else {
                    debug_assert!(false, "load_custom_paint_src: node {target_id} not element");
                }
            }
        }
    }

    fn process_button_input(&mut self, target_id: usize) {
        let Some(node) = self.doc.get_node(target_id) else {
            debug_assert!(false, "process_button_input: stale target_id={target_id}");
            return;
        };
        let Some(data) = node.element_data() else {
            return;
        };

        let tagname = data.name.local.as_ref();
        let type_attr = data.attr(local_name!("type"));
        let value = data.attr(local_name!("value"));

        // Add content of "value" attribute as a text node child if:
        //   - Tag name is
        if let ("input", Some("button" | "submit" | "reset"), Some(value)) =
            (tagname, type_attr, value)
        {
            let value = value.to_string();
            let id = self.create_text_node(&value);
            self.append_children(target_id, &[id]);
        }
        #[cfg(feature = "file_input")]
        if let ("input", Some("file")) = (tagname, type_attr) {
            let button_id = self.create_element(
                qual_name!("button", html),
                vec![
                    Attribute {
                        name: qual_name!("type", html),
                        value: "button".to_string(),
                    },
                    Attribute {
                        name: qual_name!("tabindex", html),
                        value: "-1".to_string(),
                    },
                ],
            );
            let label_id = self.create_element(qual_name!("label", html), vec![]);
            let text_id = self.create_text_node("No File Selected");
            let button_text_id = self.create_text_node("Browse");
            self.append_children(target_id, &[button_id, label_id]);
            self.append_children(label_id, &[text_id]);
            self.append_children(button_id, &[button_text_id]);
        }
    }
}

/// Set 'checked' state on an input based on given attributevalue
fn set_input_checked_state(element: &mut ElementData, value: String) {
    let Ok(checked) = value.parse() else {
        return;
    };
    match element.special_data {
        SpecialElementData::CheckboxInput(ref mut checked_mut) => *checked_mut = checked,
        // If we have just constructed the element, set the node attribute,
        // and NodeSpecificData will be created from that later
        // this simulates the checked attribute being set in html,
        // and the element's checked property being set from that
        SpecialElementData::None => element.attrs.push(Attribute {
            name: qual_name!("checked", html),
            value: checked.to_string(),
        }),
        _ => {}
    }
}

/// Type that allows mutable access to the viewport
/// And syncs it back to stylist on drop.
pub struct ViewportMut<'doc> {
    doc: &'doc mut BaseDocument,
    initial_viewport: Viewport,
}
impl ViewportMut<'_> {
    pub fn new(doc: &mut BaseDocument) -> ViewportMut<'_> {
        let initial_viewport = doc.viewport.clone();
        ViewportMut {
            doc,
            initial_viewport,
        }
    }
}
impl Deref for ViewportMut<'_> {
    type Target = Viewport;

    fn deref(&self) -> &Self::Target {
        &self.doc.viewport
    }
}
impl DerefMut for ViewportMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.doc.viewport
    }
}
impl Drop for ViewportMut<'_> {
    fn drop(&mut self) {
        if self.doc.viewport == self.initial_viewport {
            return;
        }

        self.doc
            .set_stylist_device(make_device(&self.doc.viewport, self.doc.font_ctx.clone()));
        self.doc.scroll_viewport_by(0.0, 0.0); // Clamp scroll offset

        let scale_has_changed =
            self.doc.viewport().scale_f64() != self.initial_viewport.scale_f64();
        if scale_has_changed {
            self.doc.invalidate_inline_contexts();
            self.doc.shell_provider.request_redraw();
        }
    }
}

#[cfg(test)]
mod test {
    use style_dom::ElementState;

    use crate::{Attribute, BaseDocument, DocumentConfig, ElementData, NodeData, qual_name};

    #[test]
    fn mutator_remove_disabled() {
        let mut document = BaseDocument::new(DocumentConfig::default());
        let id = document.create_node(NodeData::Element(ElementData::new(
            qual_name!("button"),
            vec![Attribute {
                name: qual_name!("disabled"),
                value: "".into(),
            }],
        )));

        let node = document.get_node(id).unwrap();
        assert!(
            node.element_state.contains(ElementState::DISABLED),
            "form node is disabled"
        );
        assert!(
            !node.element_state.contains(ElementState::ENABLED),
            "form node is not enabled yet"
        );

        let mut mutator = document.mutate();
        mutator.clear_attribute(id, qual_name!("disabled"));
        drop(mutator);

        let node = document.get_node(id).unwrap();
        assert!(
            !node.element_state.contains(ElementState::DISABLED),
            "form node is no longer disabled"
        );
        assert!(
            node.element_state.contains(ElementState::ENABLED),
            "form node is enabled"
        );
    }

    #[test]
    fn mutator_set_disabled() {
        let mut document = BaseDocument::new(DocumentConfig::default());
        let id = document.create_node(NodeData::Element(ElementData::new(
            qual_name!("button"),
            vec![],
        )));

        let node = document.get_node(id).unwrap();
        assert!(
            !node.element_state.contains(ElementState::DISABLED),
            "form node is not disabled"
        );
        assert!(
            node.element_state.contains(ElementState::ENABLED),
            "form node is enabled"
        );

        let mut mutator = document.mutate();
        mutator.set_attribute(id, qual_name!("disabled"), "");
        drop(mutator);

        let node = document.get_node(id).unwrap();

        assert!(
            node.element_state.contains(ElementState::DISABLED),
            "form node is disabled"
        );
        assert!(
            !node.element_state.contains(ElementState::ENABLED),
            "form node is no longer enabled enabled"
        );
    }

    #[test]
    fn mutator_set_disabled_invalid_node() {
        let mut document = BaseDocument::new(DocumentConfig::default());
        let id = document.create_node(NodeData::Element(ElementData::new(qual_name!("a"), vec![])));

        let node = document.get_node(id).unwrap();
        assert!(
            !node.element_state.contains(ElementState::DISABLED),
            "form node is not disabled"
        );
        assert!(
            !node.element_state.contains(ElementState::ENABLED),
            "form node is enabled"
        );

        let mut mutator = document.mutate();
        mutator.set_attribute(id, qual_name!("disabled"), "");
        drop(mutator);

        let node = document.get_node(id).unwrap();
        assert!(
            !node.element_state.contains(ElementState::DISABLED),
            "form node is not disabled"
        );
        assert!(
            !node.element_state.contains(ElementState::ENABLED),
            "form node is enabled"
        );
    }
}
