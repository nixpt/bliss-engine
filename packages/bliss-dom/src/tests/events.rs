//! Tests for synthetic input (`EventDriver::synthesize_*`) — BLISS-HEADLESS-1.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bliss_traits::events::{DomEvent, DomEventData, EventState};
use bliss_traits::shell::{ColorScheme, Viewport};

use crate::{BaseDocument, Document, DocumentConfig, EventDriver, EventHandler, qual_name};

/// Counts the events it sees, so a test can assert dispatch happened.
struct CountingHandler {
    count: Arc<AtomicUsize>,
}

impl EventHandler for CountingHandler {
    fn handle_event(
        &mut self,
        _chain: &[usize],
        _event: &mut DomEvent,
        _doc: &mut dyn Document,
        _event_state: &mut EventState,
    ) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

/// A document with a 200x100 box at the origin, laid out.
fn doc_with_box() -> (BaseDocument, usize) {
    let config = DocumentConfig {
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    };
    let mut doc = BaseDocument::new(config);
    let mut mutator = doc.mutate();
    let div = mutator.create_element(qual_name!("div"), vec![]);
    mutator.set_style_property(div, "width", "200px");
    mutator.set_style_property(div, "height", "100px");
    mutator.append_children(0, &[div]);
    drop(mutator);
    doc.resolve(0.0);
    (doc, div)
}

#[test]
fn synthetic_pointer_move_hovers() {
    let (mut doc, _div) = doc_with_box();
    let count = Arc::new(AtomicUsize::new(0));
    {
        let mut driver = EventDriver::new(&mut doc, CountingHandler { count: count.clone() });
        driver.synthesize_pointer_move(10.0, 10.0);
    }
    assert!(doc.hover_node_id.is_some(), "pointer move should set hover");
}

#[test]
fn synthetic_click_dispatches_events() {
    let (mut doc, _div) = doc_with_box();
    let count = Arc::new(AtomicUsize::new(0));
    {
        let mut driver = EventDriver::new(&mut doc, CountingHandler { count: count.clone() });
        driver.synthesize_click(10.0, 10.0);
    }
    assert!(
        count.load(Ordering::SeqCst) > 0,
        "click should dispatch handler events"
    );
}

#[test]
fn synthetic_wheel_dispatches_events() {
    let (mut doc, _div) = doc_with_box();
    let count = Arc::new(AtomicUsize::new(0));
    {
        let mut driver = EventDriver::new(&mut doc, CountingHandler { count: count.clone() });
        driver.synthesize_wheel(10.0, 10.0, 0.0, -40.0);
    }
    assert!(
        count.load(Ordering::SeqCst) > 0,
        "wheel should dispatch handler events"
    );
}

/// A handler that marks the clicked node, so a test can drive the DOM with
/// synthetic input and read the result back off the tree dump.
struct FlipsAttribute;

impl EventHandler for FlipsAttribute {
    fn handle_event(
        &mut self,
        _chain: &[usize],
        event: &mut DomEvent,
        doc: &mut dyn Document,
        _event_state: &mut EventState,
    ) {
        if matches!(event.data, DomEventData::Click(_)) {
            let target = event.target;
            let mut inner = doc.inner_mut();
            inner.mutate().set_attribute(target, qual_name!("title"), "clicked");
        }
    }
}

#[test]
fn headless_input_then_dump() {
    // input -> state -> dump: a synthetic click runs through to the handler,
    // which marks the node; the tree dump shows the mark.
    let (mut doc, _div) = doc_with_box();
    {
        let mut driver = EventDriver::new(&mut doc, FlipsAttribute);
        driver.synthesize_click(10.0, 10.0);
    }
    let json = doc.document_tree_json();
    assert!(json.contains("\"title\":\"clicked\""), "dump missing the click: {json}");
}
