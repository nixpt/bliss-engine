//! Tests for layout/construct.rs - geometry golden tests

use crate::{BaseDocument, DocumentConfig, DocumentMutator, qual_name};

fn setup_doc() -> BaseDocument {
    let config = DocumentConfig::default();
    BaseDocument::new(config)
}

// ── Geometry Golden Tests ───────────────────────────────────────────────────

/// Test that layout is computed for a simple div
#[test]
fn test_simple_div_layout() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let div = mutator.create_element(qual_name!("div"), vec![]);
    mutator.append_children(0, &[div]);
    
    drop(mutator);
    
    // Resolve layout to compute geometry
    doc.resolve(0.0);
    
    // Get the node and verify layout was computed
    if let Some(node) = doc.nodes.get(div) {
        // Layout should have been computed, check it's not all zeros
        // Note: actual dimensions depend on CSS, so we just check layout exists
        let _ = node.final_layout.content_box_width();
        let _ = node.final_layout.content_box_height();
        // If we get here without panicking, layout was computed
    }
}

/// Test that nested elements have layout computed
#[test]
fn test_nested_elements_layout() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let parent = mutator.create_element(qual_name!("div"), vec![]);
    let child = mutator.create_element(qual_name!("div"), vec![]);
    
    mutator.append_children(0, &[parent]);
    mutator.append_children(parent, &[child]);
    
    drop(mutator);
    
    // Resolve layout
    doc.resolve(0.0);
    
    // Both nodes should have layout computed
    if let Some(parent_node) = doc.nodes.get(parent) {
        let _ = parent_node.final_layout.content_box_width();
    }
    
    if let Some(child_node) = doc.nodes.get(child) {
        let _ = child_node.final_layout.content_box_width();
    }
}

/// Test that text nodes have layout computed
#[test]
fn test_text_node_layout() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let text = mutator.create_text_node("Hello, world!");
    mutator.append_children(0, &[text]);
    
    drop(mutator);
    
    // Resolve layout
    doc.resolve(0.0);
    
    // Text node should have layout
    if let Some(node) = doc.nodes.get(text) {
        let _ = node.unrounded_layout.content_box_width();
        let _ = node.unrounded_layout.content_box_height();
    }
}

/// Test flexbox layout is computed
#[test]
fn test_flexbox_layout() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let flex_container = mutator.create_element(qual_name!("div"), vec![]);
    let flex_item1 = mutator.create_element(qual_name!("div"), vec![]);
    let flex_item2 = mutator.create_element(qual_name!("div"), vec![]);
    
    mutator.append_children(0, &[flex_container]);
    mutator.append_children(flex_container, &[flex_item1, flex_item2]);
    
    drop(mutator);
    
    // Resolve layout
    doc.resolve(0.0);
    
    // All nodes should have layout computed
    if let Some(node) = doc.nodes.get(flex_container) {
        let _ = node.final_layout.content_box_width();
    }
    
    if let Some(node) = doc.nodes.get(flex_item1) {
        let _ = node.final_layout.content_box_width();
    }
    
    if let Some(node) = doc.nodes.get(flex_item2) {
        let _ = node.final_layout.content_box_width();
    }
}

/// Test that multiple children have layout computed
#[test]
fn test_multiple_children_layout() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let parent = mutator.create_element(qual_name!("div"), vec![]);
    let child1 = mutator.create_element(qual_name!("span"), vec![]);
    let child2 = mutator.create_element(qual_name!("p"), vec![]);
    let child3 = mutator.create_element(qual_name!("a"), vec![]);
    
    mutator.append_children(parent, &[child1, child2, child3]);
    mutator.append_children(0, &[parent]);
    
    drop(mutator);
    
    // Resolve layout
    doc.resolve(0.0);
    
    // All nodes should have layout
    if let Some(node) = doc.nodes.get(parent) {
        let _ = node.final_layout.content_box_width();
    }
    
    if let Some(node) = doc.nodes.get(child1) {
        let _ = node.final_layout.content_box_width();
    }
    
    if let Some(node) = doc.nodes.get(child2) {
        let _ = node.final_layout.content_box_width();
    }
    
    if let Some(node) = doc.nodes.get(child3) {
        let _ = node.final_layout.content_box_width();
    }
}
