//! Tests for query_selector.rs

use crate::{BaseDocument, DocumentConfig, DocumentMutator, qual_name};

fn setup_doc() -> BaseDocument {
    let config = DocumentConfig::default();
    BaseDocument::new(config)
}

// ── query_selector Tests ─────────────────────────────────────────────────────

#[test]
fn test_query_selector_by_tag_name() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let div1 = mutator.create_element(qual_name!("div"), vec![]);
    let span = mutator.create_element(qual_name!("span"), vec![]);
    let div2 = mutator.create_element(qual_name!("div"), vec![]);
    
    // Build tree: root -> [div1, span, div2]
    mutator.append_children(0, &[div1, span, div2]);
    drop(mutator);

    // Query for first div
    let result = doc.query_selector("div").unwrap();
    assert!(result.is_some());
    assert_eq!(result, Some(div1));
}

#[test]
fn test_query_selector_all_by_tag_name() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let div1 = mutator.create_element(qual_name!("div"), vec![]);
    let span = mutator.create_element(qual_name!("span"), vec![]);
    let div2 = mutator.create_element(qual_name!("div"), vec![]);
    
    // Build tree: root -> [div1, span, div2]
    mutator.append_children(0, &[div1, span, div2]);
    drop(mutator);

    // Query for all divs
    let result = doc.query_selector_all("div").unwrap();
    assert_eq!(result.len(), 2);
    assert!(result.contains(&div1));
    assert!(result.contains(&div2));
}

#[test]
fn test_query_selector_by_id() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let div1 = mutator.create_element(qual_name!("div"), vec![]);
    let div2 = mutator.create_element(qual_name!("div"), vec![]);
    
    // Set IDs
    mutator.set_attribute(div1, qual_name!("id"), "first");
    mutator.set_attribute(div2, qual_name!("id"), "second");
    
    // Build tree
    mutator.append_children(0, &[div1, div2]);
    drop(mutator);

    // Query by ID
    let result = doc.query_selector("#first").unwrap();
    assert_eq!(result, Some(div1));
    
    let result2 = doc.query_selector("#second").unwrap();
    assert_eq!(result2, Some(div2));
}

#[test]
fn test_query_selector_by_class() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let div1 = mutator.create_element(qual_name!("div"), vec![]);
    let div2 = mutator.create_element(qual_name!("div"), vec![]);
    let div3 = mutator.create_element(qual_name!("div"), vec![]);
    
    // Set classes
    mutator.set_attribute(div1, qual_name!("class"), "active");
    mutator.set_attribute(div2, qual_name!("class"), "active");
    // div3 has no class
    
    // Build tree
    mutator.append_children(0, &[div1, div2, div3]);
    drop(mutator);

    // Query by class
    let result = doc.query_selector_all(".active").unwrap();
    assert_eq!(result.len(), 2);
    assert!(result.contains(&div1));
    assert!(result.contains(&div2));
    assert!(!result.contains(&div3));
}

#[test]
fn test_query_selector_by_attribute() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let div1 = mutator.create_element(qual_name!("div"), vec![]);
    let div2 = mutator.create_element(qual_name!("div"), vec![]);
    
    // Set attributes
    mutator.set_attribute(div1, qual_name!("role"), "button");
    mutator.set_attribute(div2, qual_name!("role"), "button");
    
    // Build tree
    mutator.append_children(0, &[div1, div2]);
    drop(mutator);

    // Query by attribute presence
    let result = doc.query_selector_all("[role=button]").unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn test_query_selector_nested() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let root_div = mutator.create_element(qual_name!("div"), vec![]);
    let parent_div = mutator.create_element(qual_name!("div"), vec![]);
    let child_span = mutator.create_element(qual_name!("span"), vec![]);
    
    // Set IDs
    mutator.set_attribute(root_div, qual_name!("id"), "root");
    mutator.set_attribute(child_span, qual_name!("id"), "child");
    
    // Build nested tree: root -> root_div -> parent_div -> child_span
    mutator.append_children(0, &[root_div]);
    mutator.append_children(root_div, &[parent_div]);
    mutator.append_children(parent_div, &[child_span]);
    drop(mutator);

    // Query for nested span
    let result = doc.query_selector("#root span").unwrap();
    assert_eq!(result, Some(child_span));
    
    // Query for nested span by ID
    let result2 = doc.query_selector("div div #child").unwrap();
    assert_eq!(result2, Some(child_span));
}

#[test]
fn test_query_selector_all_nested() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let root_div = mutator.create_element(qual_name!("div"), vec![]);
    let span1 = mutator.create_element(qual_name!("span"), vec![]);
    let span2 = mutator.create_element(qual_name!("span"), vec![]);
    let nested_div = mutator.create_element(qual_name!("div"), vec![]);
    let span3 = mutator.create_element(qual_name!("span"), vec![]);
    
    // Build tree: root -> [root_div, span1] and root_div -> [span2, nested_div] and nested_div -> [span3]
    mutator.append_children(0, &[root_div, span1]);
    mutator.append_children(root_div, &[span2, nested_div]);
    mutator.append_children(nested_div, &[span3]);
    drop(mutator);

    // Query for all spans
    let result = doc.query_selector_all("span").unwrap();
    assert_eq!(result.len(), 3);
    assert!(result.contains(&span1));
    assert!(result.contains(&span2));
    assert!(result.contains(&span3));
}

#[test]
fn test_query_selector_no_match() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let div = mutator.create_element(qual_name!("div"), vec![]);
    mutator.append_children(0, &[div]);
    drop(mutator);

    // Query for non-existent element
    let result = doc.query_selector("span").unwrap();
    assert_eq!(result, None);
    
    // Query all for non-existent element
    let result_all = doc.query_selector_all("span").unwrap();
    assert_eq!(result_all.len(), 0);
}

#[test]
fn test_query_selector_complex() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let div1 = mutator.create_element(qual_name!("div"), vec![]);
    let div2 = mutator.create_element(qual_name!("div"), vec![]);
    let span = mutator.create_element(qual_name!("span"), vec![]);
    
    // Set attributes
    mutator.set_attribute(div1, qual_name!("class"), "container");
    mutator.set_attribute(div2, qual_name!("class"), "item");
    mutator.set_attribute(span, qual_name!("id"), "special");
    
    // Build tree
    mutator.append_children(0, &[div1, div2, span]);
    drop(mutator);

    // Query for div with class container
    let result = doc.query_selector("div.container").unwrap();
    assert_eq!(result, Some(div1));
    
    // Query for div.item
    let result2 = doc.query_selector("div.item").unwrap();
    assert_eq!(result2, Some(div2));
}

#[test]
fn test_get_element_by_id() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let div1 = mutator.create_element(qual_name!("div"), vec![]);
    let div2 = mutator.create_element(qual_name!("div"), vec![]);
    
    // Set IDs
    mutator.set_attribute(div1, qual_name!("id"), "test-id");
    mutator.set_attribute(div2, qual_name!("id"), "other-id");
    
    // Build tree
    mutator.append_children(0, &[div1, div2]);
    drop(mutator);

    // Get by ID
    let result = doc.get_element_by_id("test-id");
    assert_eq!(result, Some(div1));
    
    let result2 = doc.get_element_by_id("other-id");
    assert_eq!(result2, Some(div2));
    
    // Non-existent ID
    let result3 = doc.get_element_by_id("does-not-exist");
    assert_eq!(result3, None);
}

// ── matches / closest Tests ─────────────────────────────────────────────────

#[test]
fn test_matches_by_tag_and_class() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let div = mutator.create_element(qual_name!("div"), vec![]);
    let span = mutator.create_element(qual_name!("span"), vec![]);
    mutator.set_attribute(div, qual_name!("class"), "active");

    mutator.append_children(0, &[div, span]);
    drop(mutator);

    assert_eq!(doc.matches(div, "div").unwrap(), true);
    assert_eq!(doc.matches(div, ".active").unwrap(), true);
    assert_eq!(doc.matches(div, "span").unwrap(), false);
    assert_eq!(doc.matches(span, ".active").unwrap(), false);
}

#[test]
fn test_matches_nonexistent_node() {
    let doc = setup_doc();
    assert_eq!(doc.matches(9999, "div").unwrap(), false);
}

#[test]
fn test_closest_walks_ancestors() {
    let mut doc = setup_doc();
    let mut mutator = doc.mutate();

    let container = mutator.create_element(qual_name!("div"), vec![]);
    let inner = mutator.create_element(qual_name!("div"), vec![]);
    let child_span = mutator.create_element(qual_name!("span"), vec![]);
    mutator.set_attribute(container, qual_name!("class"), "container");

    // Build tree: root -> container -> inner -> child_span
    mutator.append_children(0, &[container]);
    mutator.append_children(container, &[inner]);
    mutator.append_children(inner, &[child_span]);
    drop(mutator);

    // Nearest ancestor (inclusive) matching ".container" from a deeply nested node
    let result = doc.closest(child_span, ".container").unwrap();
    assert_eq!(result, Some(container));

    // Matches itself first, without walking further up
    let result2 = doc.closest(container, ".container").unwrap();
    assert_eq!(result2, Some(container));

    // No ancestor matches
    let result3 = doc.closest(child_span, "#does-not-exist").unwrap();
    assert_eq!(result3, None);
}
