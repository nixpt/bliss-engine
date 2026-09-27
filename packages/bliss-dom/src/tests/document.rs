//! Tests for document.rs - mutation round-trips

use crate::{Attribute, BaseDocument, DocumentConfig, DocumentMutator, local_name, qual_name};

fn setup_doc() -> BaseDocument {
    let config = DocumentConfig::default();
    BaseDocument::new(config)
}

// ── Node Creation and Removal Round-Trips ──────────────────────────────────────

#[test]
fn test_create_and_remove_element() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let root = mutator.create_element(qual_name!("div"), vec![]);
    let child = mutator.create_element(qual_name!("span"), vec![]);
    mutator.append_children(root, &[child]);

    assert!(mutator.doc.nodes.get(root).is_some());
    assert!(mutator.doc.nodes.get(child).is_some());

    mutator.remove_and_drop_node(child);

    assert!(mutator.doc.nodes.get(root).is_some());
    assert!(mutator.doc.nodes.get(child).is_none());
    drop(mutator);
}

#[test]
fn test_create_text_node_and_get_content() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let text_node = mutator.create_text_node("Hello, world!");
    assert!(mutator.doc.nodes.get(text_node).is_some());
    
    if let Some(node) = mutator.doc.nodes.get(text_node) {
        if let Some(text_data) = node.text_data() {
            assert_eq!(text_data.content, "Hello, world!");
        } else {
            panic!("Expected text node");
        }
    }
    drop(mutator);
}

#[test]
fn test_set_node_text() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let text_node = mutator.create_text_node("initial");
    mutator.set_node_text(text_node, "updated");

    if let Some(node) = mutator.doc.nodes.get(text_node) {
        if let Some(text_data) = node.text_data() {
            assert_eq!(text_data.content, "updated");
        } else {
            panic!("Expected text node");
        }
    }
    drop(mutator);
}

// ── Attribute Round-Trips ────────────────────────────────────────────────────

#[test]
fn test_set_and_get_attribute() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let elem = mutator.create_element(qual_name!("div"), vec![]);
    mutator.set_attribute(elem, qual_name!("id"), "my-element");
    
    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(node.attr(local_name!("id")), Some("my-element"));
    } else {
        panic!("Element not found");
    }
    drop(mutator);
}

#[test]
fn test_set_and_clear_attribute() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let elem = mutator.create_element(qual_name!("div"), vec![]);
    mutator.set_attribute(elem, qual_name!("class"), "active");
    
    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(node.attr(local_name!("class")), Some("active"));
    }
    
    mutator.clear_attribute(elem, qual_name!("class"));
    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(node.attr(local_name!("class")), None);
    }
    drop(mutator);
}

#[test]
fn test_multiple_attributes() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let elem = mutator.create_element(qual_name!("div"), vec![]);
    mutator.set_attribute(elem, qual_name!("id"), "test");
    mutator.set_attribute(elem, qual_name!("class"), "foo bar");
    mutator.set_attribute(elem, qual_name!("title"), "My Title");
    
    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(node.attr(local_name!("id")), Some("test"));
        assert_eq!(node.attr(local_name!("class")), Some("foo bar"));
        assert_eq!(node.attr(local_name!("title")), Some("My Title"));
    }
    drop(mutator);
}

// ── Tree Structure Round-Trips ────────────────────────────────────────────────

#[test]
fn test_append_children() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let parent = mutator.create_element(qual_name!("div"), vec![]);
    let child1 = mutator.create_element(qual_name!("span"), vec![]);
    let child2 = mutator.create_element(qual_name!("p"), vec![]);
    
    mutator.append_children(parent, &[child1, child2]);
    
    let children = mutator.child_ids(parent);
    assert_eq!(children.len(), 2);
    assert!(children.contains(&child1));
    assert!(children.contains(&child2));
    drop(mutator);
}

#[test]
fn test_insert_before() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let parent = mutator.create_element(qual_name!("div"), vec![]);
    let anchor = mutator.create_element(qual_name!("span"), vec![]);
    let new_node = mutator.create_element(qual_name!("p"), vec![]);
    
    mutator.append_children(parent, &[anchor]);
    mutator.insert_nodes_before(anchor, &[new_node]);
    
    let children = mutator.child_ids(parent);
    assert_eq!(children.len(), 2);
    let pos_new = children.iter().position(|&id| id == new_node).unwrap();
    let pos_anchor = children.iter().position(|&id| id == anchor).unwrap();
    assert!(pos_new < pos_anchor);
    drop(mutator);
}

#[test]
fn test_insert_after() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let parent = mutator.create_element(qual_name!("div"), vec![]);
    let anchor = mutator.create_element(qual_name!("span"), vec![]);
    let new_node = mutator.create_element(qual_name!("p"), vec![]);
    
    mutator.append_children(parent, &[anchor]);
    mutator.insert_nodes_after(anchor, &[new_node]);
    
    let children = mutator.child_ids(parent);
    assert_eq!(children.len(), 2);
    let pos_anchor = children.iter().position(|&id| id == anchor).unwrap();
    let pos_new = children.iter().position(|&id| id == new_node).unwrap();
    assert!(pos_anchor < pos_new);
    drop(mutator);
}

#[test]
fn test_replace_node_with() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let parent = mutator.create_element(qual_name!("div"), vec![]);
    let old_node = mutator.create_element(qual_name!("span"), vec![]);
    let new_node = mutator.create_element(qual_name!("p"), vec![]);
    
    mutator.append_children(parent, &[old_node]);
    mutator.replace_node_with(old_node, &[new_node]);
    
    let children = mutator.child_ids(parent);
    assert_eq!(children.len(), 1);
    assert!(children.contains(&new_node));
    assert!(!children.contains(&old_node));
    drop(mutator);
}

#[test]
fn test_remove_all_children() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let parent = mutator.create_element(qual_name!("div"), vec![]);
    let child1 = mutator.create_element(qual_name!("span"), vec![]);
    let child2 = mutator.create_element(qual_name!("p"), vec![]);
    
    mutator.append_children(parent, &[child1, child2]);
    assert_eq!(mutator.child_ids(parent).len(), 2);
    
    mutator.remove_and_drop_all_children(parent);
    assert_eq!(mutator.child_ids(parent).len(), 0);
    drop(mutator);
}

#[test]
fn test_reparent_children() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let old_parent = mutator.create_element(qual_name!("div"), vec![]);
    let new_parent = mutator.create_element(qual_name!("section"), vec![]);
    let child1 = mutator.create_element(qual_name!("span"), vec![]);
    let child2 = mutator.create_element(qual_name!("p"), vec![]);
    
    mutator.append_children(old_parent, &[child1, child2]);
    assert_eq!(mutator.child_ids(old_parent).len(), 2);
    assert_eq!(mutator.child_ids(new_parent).len(), 0);
    
    mutator.reparent_children(old_parent, new_parent);
    
    assert_eq!(mutator.child_ids(old_parent).len(), 0);
    assert_eq!(mutator.child_ids(new_parent).len(), 2);
    drop(mutator);
}

// ── Inner HTML Round-Trips ────────────────────────────────────────────────────
// Note: These tests are skipped because they require HTML parser configuration
// which is not available in the default test setup.

#[ignore = "Requires HTML parser provider"]
#[test]
fn test_set_inner_html() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let elem = mutator.create_element(qual_name!("div"), vec![]);
    mutator.set_inner_html(elem, "<span>Hello</span>");
    
    let children = mutator.child_ids(elem);
    assert_eq!(children.len(), 1);
    drop(mutator);
}

#[ignore = "Requires HTML parser provider"]
#[test]
fn test_set_inner_html_with_multiple_elements() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let elem = mutator.create_element(qual_name!("div"), vec![]);
    mutator.set_inner_html(elem, "<span>First</span><span>Second</span>");
    
    let children = mutator.child_ids(elem);
    assert_eq!(children.len(), 2);
    drop(mutator);
}

// ── Complex Round-Trip: Build and Verify Tree ────────────────────────────────

#[test]
fn test_complex_document_round_trip() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let root = mutator.create_element(qual_name!("html"), vec![]);
    let head = mutator.create_element(qual_name!("head"), vec![]);
    let body = mutator.create_element(qual_name!("body"), vec![]);
    let title = mutator.create_element(qual_name!("title"), vec![]);
    let h1 = mutator.create_element(qual_name!("h1"), vec![]);
    let p = mutator.create_element(qual_name!("p"), vec![]);
    let text1 = mutator.create_text_node("Hello");
    let text2 = mutator.create_text_node("World");
    
    mutator.set_attribute(root, qual_name!("lang"), "en");
    
    mutator.append_children(root, &[head, body]);
    mutator.append_children(head, &[title]);
    mutator.append_children(body, &[h1, p]);
    mutator.append_children(p, &[text1, text2]);
    
    assert_eq!(mutator.child_ids(root).len(), 2);
    assert_eq!(mutator.child_ids(head).len(), 1);
    assert_eq!(mutator.child_ids(body).len(), 2);
    assert_eq!(mutator.child_ids(p).len(), 2);
    
    if let Some(node) = mutator.doc.nodes.get(root) {
        assert_eq!(node.attr(local_name!("lang")), Some("en"));
    }
    
    assert!(mutator.doc.nodes.get(text1).is_some());
    assert!(mutator.doc.nodes.get(text2).is_some());
    drop(mutator);
}

// ── ElementData::id refresh on post-construction set_attribute ──────────────
//
// The second `assert_eq!` in this test (`ElementData::id == Some("renamed")`
// after `set_attribute(elem, "id", "renamed")`) is the load-bearing one — it's
// the post-construction mutator path that was the historically-failed case in
// older refactors. Any future refactor that regresses this assertion fails CI.
//
// Locks in the `ElementData::id` orphan-field invariant: `ElementData.id`
// must stay in sync with `node.attr(local_name!("id"))` across construction,
// `DocumentMutator::set_attribute`, and `DocumentMutator::clear_attribute`.
//
// mutator.rs::DocumentMutator::set_attribute handles the id-name case by
// snapshotting the node and writing `element.id = Some(Atom::from(value))`
// when the attribute's local name is `id`. clear_attribute analogously
// resets `element.id = None` for the id case. Without those guards,
// `ElementData::id` would diverge from `node.attr("id")` — the orphan-field
// hazard this test exists to prevent.
//
// If a future change regresses this, the test fails at the second assertion
// while the first still passes (proving the divergence).
#[test]
fn test_elementdata_id_field_tracks_set_attribute() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    // 1) construct with id at creation time — both accessors must agree.
    let elem = mutator.create_element(
        qual_name!("div"),
        vec![Attribute {
            name: qual_name!("id"),
            value: "initial".to_string(),
        }],
    );

    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(
            node.attr(local_name!("id")),
            Some("initial"),
            "attr() round-trip on construction-time id"
        );
        if let Some(el_data) = node.element_data() {
            assert_eq!(
                el_data.id.as_deref(),
                Some("initial"),
                "ElementData::id reads back 'initial' at construction"
            );
        } else {
            panic!("Element at {elem} has no element_data");
        }
    } else {
        panic!("Element not found after construction");
    }

    // 2) post-construction set_attribute("id", "renamed") — both accessors
    //    must agree on the new value.
    mutator.set_attribute(elem, qual_name!("id"), "renamed");

    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(
            node.attr(local_name!("id")),
            Some("renamed"),
            "attr() round-trip after set_attribute"
        );
        if let Some(el_data) = node.element_data() {
            assert_eq!(
                el_data.id.as_deref(),
                Some("renamed"),
                "ElementData::id reads back 'renamed' after set_attribute"
            );
        } else {
            panic!("Element at {elem} lost element_data after set_attribute");
        }
    } else {
        panic!("Element not found after set_attribute");
    }

    // 3) post-construction clear_attribute("id") — both accessors must
    //    reflect the absence.
    mutator.clear_attribute(elem, qual_name!("id"));

    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(
            node.attr(local_name!("id")),
            None,
            "attr() round-trip after clear_attribute"
        );
        if let Some(el_data) = node.element_data() {
            assert_eq!(
                el_data.id, None,
                "ElementData::id becomes None after clear_attribute"
            );
        } else {
            panic!("Element at {elem} lost element_data after clear_attribute");
        }
    } else {
        panic!("Element not found after clear_attribute");
    }

    // 4) Empty-string id → `ElementData::id` must round-trip as Some("")
    //    (not silently coerced to None). Regression target: a future
    //    refactor that special-cases empty-string-as-None would pass
    //    sub-blocks 2/3 above while silently dropping empty ids here.
    mutator.set_attribute(elem, qual_name!("id"), "");

    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(
            node.attr(local_name!("id")),
            Some(""),
            "attr() round-trip after set_attribute(\"\")"
        );
        if let Some(el_data) = node.element_data() {
            assert_eq!(
                el_data.id.as_deref(),
                Some(""),
                "ElementData::id round-trips empty string"
            );
        } else {
            panic!("Element at {elem} lost element_data after set_attribute(\"\")");
        }
    } else {
        panic!("Element not found after set_attribute(\"\")");
    }
    drop(mutator);
}

// Companion negative test: `clear_attribute("class")` (or any non-id attr)
// MUST NOT touch `ElementData::id`. This guards against a regression where
// someone broadens the clear path to reset `element.id = None` regardless
// of attr name. Without it, the positive test above could pass while still
// regressing the negative contract.
#[test]
fn test_clear_attribute_does_not_reset_id_for_non_id_attrs() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let elem = mutator.create_element(
        qual_name!("div"),
        vec![Attribute {
            name: qual_name!("id"),
            value: "survivor".to_string(),
        }],
    );

    // Pre-condition: ElementData::id == attr(id) == "survivor".
    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(node.attr(local_name!("id")), Some("survivor"));
        if let Some(el_data) = node.element_data() {
            assert_eq!(el_data.id.as_deref(), Some("survivor"));
        } else {
            panic!("Element at {elem} has no element_data");
        }
    }

    // Action: clear the class attribute (not id).
    mutator.clear_attribute(elem, qual_name!("class"));

    // Post-condition: ElementData::id is STILL "survivor".
    if let Some(node) = mutator.doc.nodes.get(elem) {
        if let Some(el_data) = node.element_data() {
            assert_eq!(
                el_data.id.as_deref(),
                Some("survivor"),
                "ElementData::id must NOT be reset when clearing a non-id attribute"
            );
        } else {
            panic!("Element at {elem} lost element_data after clear_attribute(\"class\")");
        }
    } else {
        panic!("Element not found after clear_attribute(\"class\")");
    }
    drop(mutator);
}

// Companion test: lifecycle where the element is constructed WITHOUT an id
// attribute — `ElementData.id` starts as None at construction time and is
// first populated by a subsequent `DocumentMutator::set_attribute(elem, "id", ...)`
// call. This exercises a meaningfully different code path inside set_attribute:
// the snapshot must transition `element.id` from None → Some, not just
// refresh an already-populated field. Without this test, a regression that
// only handles "replace existing id with another id" would still pass the
// positive test above while breaking first-id-set-after-construction.
#[test]
fn test_elementdata_id_first_set_after_no_id_construction() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    // Construct with NO id attribute.
    let elem = mutator.create_element(qual_name!("div"), vec![]);

    // Pre-condition: ElementData::id == attr(id) == None.
    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(node.attr(local_name!("id")), None);
        if let Some(el_data) = node.element_data() {
            assert_eq!(
                el_data.id, None,
                "freshly created element has ElementData::id == None"
            );
        } else {
            panic!("Element at {elem} has no element_data");
        }
    } else {
        panic!("Element not found after construction");
    }

    // Action: first-ever id set after construction.
    mutator.set_attribute(elem, qual_name!("id"), "first-id");

    // Post-condition: both accessors agree on "first-id".
    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(node.attr(local_name!("id")), Some("first-id"));
        if let Some(el_data) = node.element_data() {
            assert_eq!(
                el_data.id.as_deref(),
                Some("first-id"),
                "ElementData::id transitions None → Some on first set_attribute(\"id\", ...)"
            );
        } else {
            panic!("Element at {elem} lost element_data after first set_attribute(\"id\", ...)");
        }
    } else {
        panic!("Element not found after first set_attribute(\"id\", ...)");
    }

    // Action: clear the id we just set.
    mutator.clear_attribute(elem, qual_name!("id"));

    // Post-condition: both accessors agree on None again.
    if let Some(node) = mutator.doc.nodes.get(elem) {
        assert_eq!(node.attr(local_name!("id")), None);
        if let Some(el_data) = node.element_data() {
            assert_eq!(
                el_data.id, None,
                "ElementData::id transitions back to None on clear_attribute(\"id\", ...)"
            );
        } else {
            panic!("Element at {elem} lost element_data after clear_attribute");
        }
    } else {
        panic!("Element not found after clear_attribute");
    }
    drop(mutator);
}

// ── D-2c-followup safety backstop: empty-doc root_element() NoPanic regression ─

#[test]
fn test_root_element_none_safety() {
    // D-2c-followup safety backstop: an empty BaseDocument (no HTML parsed,
    // no element child of the document root) must not panic in any migrated
    // root_element() call site. The cascade previously chained unwrap-panics
    // from `first_element_child().unwrap().as_element().unwrap()`; this
    // verifies the new `Option<&Node>` shape propagates None to each migrated
    // caller with sensible empty / null result behavior.
    let mut doc = setup_doc();

    // try_root_element() / root_element() both return None on empty doc.
    assert!(doc.try_root_element().is_none());
    assert!(doc.root_element().is_none());

    // hit() returns None on no root (?-propagation in migrated site).
    assert!(doc.hit(10.0, 10.0).is_none());

    // scroll_viewport_by_has_changed() returns false (content_size degrades
    // to taffy::Size::default() — zero dimensions).
    assert!(!doc.scroll_viewport_by_has_changed(0.0, 0.0));

    // resolve() short-circuits via the upstream `is_none` guard + early return;
    // the debug_assert! inside resolve() must hold — this is also a load-bearing
    // test confirming the upstream guard still functions.
    doc.resolve(0.0);

    // scroll_node_by_has_changed() with a stale / non-Element node id is a
    // no-op return false (root_element: Some(root) guard no-ops scroll_node).
    let mut scroll_event_seen = false;
    assert!(!doc.scroll_node_by_has_changed(0, 1.0, 1.0, |_| {
        scroll_event_seen = true;
    }));
    assert!(!scroll_event_seen);

    // set_layout / scroll_event are no-ops on the (non-Element) document root.
    // clear_focus returns () (RAII blur the cached focus target); clear_hover
    // returns bool. Both must run without panic on empty doc (focus_node_id /
    // hover_node_id = None).
    doc.clear_focus();
    assert!(!doc.clear_hover());
}
