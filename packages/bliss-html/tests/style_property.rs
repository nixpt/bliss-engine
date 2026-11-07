//! `set_style_property` and `remove_style_property` must take effect on the
//! next `resolve`, on elements that were already styled. These run on parsed
//! HTML and assert computed style and layout, not the stored declaration.

use bliss_dom::{BaseDocument, DocumentConfig};
use bliss_html::HtmlDocument;
use bliss_traits::dom_control::DomController;

fn resolved(html: &str) -> HtmlDocument {
    let mut doc = HtmlDocument::from_html(html, DocumentConfig::default());
    doc.resolve(0.0);
    doc
}

fn id_of(doc: &HtmlDocument, id: &str) -> usize {
    doc.get_element_by_id(id).expect("element in document")
}

fn width(doc: &HtmlDocument, id: &str) -> f32 {
    doc.get_node(id_of(doc, id))
        .unwrap()
        .final_layout
        .size
        .width
}

fn opacity(doc: &HtmlDocument, id: &str) -> f32 {
    doc.get_node(id_of(doc, id))
        .unwrap()
        .primary_styles()
        .unwrap()
        .clone_opacity()
}

#[test]
fn set_style_property_applies_on_next_resolve() {
    let mut doc = resolved(r#"<div id="a" style="width:10px;opacity:.5"></div>"#);
    assert_eq!(width(&doc, "a"), 10.0);

    let a = id_of(&doc, "a");
    {
        let mut m = doc.mutate();
        m.set_style_property(a, "width", "300px");
        m.set_style_property(a, "opacity", "0.25");
    }
    doc.resolve(0.0);

    assert_eq!(width(&doc, "a"), 300.0);
    assert_eq!(opacity(&doc, "a"), 0.25);
}

#[test]
fn set_style_property_applies_on_every_frame() {
    let mut doc = resolved(r#"<div id="a" style="width:10px"></div>"#);
    let a = id_of(&doc, "a");

    for frame in 1..=5 {
        let px = frame * 20;
        doc.mutate()
            .set_style_property(a, "width", &format!("{px}px"));
        doc.resolve(frame as f64 / 60.0);
        assert_eq!(width(&doc, "a"), px as f32);
    }
}

#[test]
fn remove_style_property_falls_back_to_the_stylesheet() {
    let mut doc =
        resolved(r#"<style>#a { width: 40px }</style><div id="a" style="width:10px"></div>"#);
    assert_eq!(width(&doc, "a"), 10.0);

    let a = id_of(&doc, "a");
    doc.mutate().remove_style_property(a, "width");
    doc.resolve(0.0);

    assert_eq!(width(&doc, "a"), 40.0);
}

#[test]
fn inherited_change_reaches_children() {
    let mut doc = resolved(
        r#"<div id="a" style="font-size:10px"><div id="b" style="width:2em"></div></div>"#,
    );
    assert_eq!(width(&doc, "b"), 20.0);

    let a = id_of(&doc, "a");
    doc.mutate().set_style_property(a, "font-size", "20px");
    doc.resolve(0.0);

    assert_eq!(width(&doc, "b"), 40.0);
}

#[test]
fn dom_control_style_changes_apply_on_next_resolve() {
    let mut doc = resolved(r#"<div id="a" style="width:10px;opacity:.5"></div>"#);
    let a = id_of(&doc, "a");

    let base: &mut BaseDocument = &mut doc;
    DomController::set_style_property(base, a, "width", "300px").unwrap();
    DomController::set_style_property(base, a, "opacity", "0.25").unwrap();
    doc.resolve(0.0);
    assert_eq!(width(&doc, "a"), 300.0);
    assert_eq!(opacity(&doc, "a"), 0.25);

    let base: &mut BaseDocument = &mut doc;
    DomController::remove_style_property(base, a, "width").unwrap();
    doc.resolve(0.0);
    assert_ne!(width(&doc, "a"), 300.0);
}
