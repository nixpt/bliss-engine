use std::fmt::Write as _;

use parley::layout::PositionedLayoutItem;

use crate::{BaseDocument, NodeData};

impl BaseDocument {
    pub fn print_taffy_tree(&self) {
        taffy::print_tree(self, taffy::NodeId::from(0usize));
    }

    pub fn debug_log_node(&self, node_id: usize) {
        let node = &self.nodes[node_id];

        #[cfg(feature = "tracing")]
        {
            tracing::info!("Layout: {:?}", &node.final_layout);
            tracing::info!("Style: {:?}", &node.style);
        }

        println!("\nNode {} {}", node.id, node.node_debug_str());

        println!("Attrs:");

        for attr in node.attrs().into_iter().flatten() {
            println!("    {}: {}", attr.name.local, attr.value);
        }

        if node.flags.is_inline_root() {
            let inline_layout = &node
                .data
                .downcast_element()
                .unwrap()
                .inline_layout_data
                .as_ref()
                .unwrap();

            println!(
                "Size: {}x{}",
                inline_layout.layout.width(),
                inline_layout.layout.height()
            );
            println!("Text content: {:?}", inline_layout.text);
            println!("Inline Boxes:");
            for ibox in inline_layout.layout.inline_boxes() {
                print!("(id: {}) ", ibox.id);
            }
            println!();
            println!("Lines:");
            for (i, line) in inline_layout.layout.lines().enumerate() {
                let metrics = line.metrics();
                let x = metrics.inline_min_coord;
                let y = metrics.block_min_coord;
                let w = metrics.inline_max_coord - metrics.inline_min_coord;
                let h = metrics.block_max_coord - metrics.block_min_coord;
                println!("Line {i}: x:{x} y:{y} width:{w} height:{h}");
                for item in line.items() {
                    print!("  ");
                    match item {
                        PositionedLayoutItem::GlyphRun(run) => {
                            print!(
                                "RUN (x: {}, w: {}) ",
                                run.offset().round(),
                                run.run().advance()
                            )
                        }
                        PositionedLayoutItem::InlineBox(ibox) => print!(
                            "BOX {:?} (id: {} x: {} y: {} w: {}, h: {})",
                            ibox.kind,
                            ibox.id,
                            ibox.x.round(),
                            ibox.y.round(),
                            ibox.width.round(),
                            ibox.height.round()
                        ),
                    }
                    println!();
                }
            }
        }

        let layout = &node.final_layout;
        println!("Layout:");
        println!(
            "  x: {x} y: {y} w: {width} h: {height} content_w: {content_width} content_h: {content_height}",
            x = layout.location.x,
            y = layout.location.y,
            width = layout.size.width,
            height = layout.size.height,
            content_width = layout.content_size.width,
            content_height = layout.content_size.height,
        );
        println!(
            "  border: l:{l} r:{r} t:{t} b:{b}",
            l = layout.border.left,
            r = layout.border.right,
            t = layout.border.top,
            b = layout.border.bottom,
        );
        println!(
            "  padding: l:{l} r:{r} t:{t} b:{b}",
            l = layout.padding.left,
            r = layout.padding.right,
            t = layout.padding.top,
            b = layout.padding.bottom,
        );
        println!(
            "  margin: l:{l} r:{r} t:{t} b:{b}",
            l = layout.margin.left,
            r = layout.margin.right,
            t = layout.margin.top,
            b = layout.margin.bottom,
        );
        println!("Parent: {:?}", node.parent);

        let children: Vec<_> = node
            .children
            .iter()
            .map(|id| &self.nodes[*id])
            .map(|node| (node.id, node.order(), node.node_debug_str()))
            .collect();
        println!("Children: {children:?}");

        println!("Layout Parent: {:?}", node.layout_parent.get());

        let layout_children: Option<Vec<_>> = node.layout_children.borrow().as_ref().map(|lc| {
            lc.iter()
                .map(|id| &self.nodes[*id])
                .map(|node| (node.id, node.order(), node.node_debug_str()))
                .collect()
        });
        if let Some(layout_children) = layout_children {
            println!("Layout Children: {layout_children:?}");
        }

        let paint_children: Option<Vec<_>> = node.paint_children.borrow().as_ref().map(|lc| {
            lc.iter()
                .map(|id| &self.nodes[*id])
                .map(|node| (node.id, node.order(), node.node_debug_str()))
                .collect()
        });
        if let Some(paint_children) = paint_children {
            println!("Paint Children: {paint_children:?}");
        }
        // taffy::print_tree(&self.dom, node_id.into());
    }

    /// Serialize the resolved DOM + layout tree as compact JSON.
    ///
    /// Each node is an object: a `tag` (an element name, or `"#document"`), or
    /// `text` / `comment` / `shadow_root` for those node kinds; an optional
    /// `attrs` map and `anonymous: true` for Bliss's anonymous block boxes; a
    /// `rect` of `[x, y, w, h]` from `Node::final_layout`; and, for container
    /// nodes, a `children` array.
    ///
    /// Rects are **block-layout** boxes — inline content (text runs and inline
    /// elements) reports a zero rect, because inline geometry lives in the
    /// element's inline layout data, not `final_layout`. Rects are zero before
    /// layout has run (call `resolve` first). Read-only and dependency-free;
    /// this is engine-side agent tooling (see the `BLISS-HEADLESS-1` ticket).
    pub fn document_tree_json(&self) -> String {
        let mut out = String::with_capacity(1024);
        self.write_node_json(self.root_node().id, &mut out);
        out
    }

    fn write_node_json(&self, id: usize, out: &mut String) {
        let node = &self.nodes[id];
        out.push('{');
        let mut wrote = false;

        match &node.data {
            NodeData::Text(text) => {
                field(out, &mut wrote, "text");
                write_json_str(out, &text.content);
            }
            NodeData::Comment => {
                field(out, &mut wrote, "comment");
                out.push_str("null");
            }
            NodeData::Document => {
                field(out, &mut wrote, "tag");
                write_json_str(out, "#document");
            }
            NodeData::ShadowRoot { host } => {
                field(out, &mut wrote, "shadow_root");
                let _ = write!(out, "{host}");
            }
            NodeData::Element(_) | NodeData::AnonymousBlock(_) => {
                if let Some(element) = node.data.downcast_element() {
                    field(out, &mut wrote, "tag");
                    write_json_str(out, &element.name.local);
                }
                if matches!(node.data, NodeData::AnonymousBlock(_)) {
                    field(out, &mut wrote, "anonymous");
                    out.push_str("true");
                }
                if let Some(attrs) = node.attrs().filter(|attrs| !attrs.is_empty()) {
                    field(out, &mut wrote, "attrs");
                    out.push('{');
                    for (i, attr) in attrs.iter().enumerate() {
                        if i > 0 {
                            out.push(',');
                        }
                        write_json_str(out, &attr.name.local);
                        out.push(':');
                        write_json_str(out, &attr.value);
                    }
                    out.push('}');
                }
            }
        }

        let layout = &node.final_layout;
        field(out, &mut wrote, "rect");
        out.push('[');
        write_json_num(out, layout.location.x);
        out.push(',');
        write_json_num(out, layout.location.y);
        out.push(',');
        write_json_num(out, layout.size.width);
        out.push(',');
        write_json_num(out, layout.size.height);
        out.push(']');

        if !matches!(node.data, NodeData::Text(_) | NodeData::Comment) {
            field(out, &mut wrote, "children");
            out.push('[');
            for (i, &child) in node.children.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                self.write_node_json(child, out);
            }
            out.push(']');
        }

        out.push('}');
    }
}

/// Write `"name":` into a JSON object, inserting a comma between fields.
fn field(out: &mut String, wrote: &mut bool, name: &str) {
    if *wrote {
        out.push(',');
    } else {
        *wrote = true;
    }
    write_json_str(out, name);
    out.push(':');
}

/// Write `s` as a JSON string literal, escaping the characters JSON requires.
fn write_json_str(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Write an `f32` as a JSON number (non-finite values become `0`).
fn write_json_num(out: &mut String, value: f32) {
    if value.is_finite() {
        let _ = write!(out, "{value}");
    } else {
        out.push('0');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentConfig, qual_name};

    fn sample() -> BaseDocument {
        let mut doc = BaseDocument::new(DocumentConfig::default());
        let mut mutator = doc.mutate();
        let root = mutator.create_element(qual_name!("div"), vec![]);
        mutator.set_attribute(root, qual_name!("id"), "root");
        let text = mutator.create_text_node("Hello");
        mutator.append_children(root, &[text]);
        mutator.append_children(0, &[root]);
        drop(mutator);
        doc
    }

    #[test]
    fn dumps_the_tree_shape() {
        let json = sample().document_tree_json();
        assert!(json.contains("\"tag\":\"#document\""), "{json}");
        assert!(json.contains("\"tag\":\"div\""), "{json}");
        assert!(json.contains("\"attrs\":{\"id\":\"root\"}"), "{json}");
        assert!(json.contains("\"text\":\"Hello\""), "{json}");
        assert!(json.contains("\"rect\":["), "{json}");
        assert!(json.contains("\"children\":["), "{json}");
    }

    #[test]
    fn escapes_json_strings() {
        let mut out = String::new();
        write_json_str(&mut out, "a\"b\\c\nd\te");
        assert_eq!(out, "\"a\\\"b\\\\c\\nd\\te\"");
    }

    #[test]
    fn numbers_are_finite_json() {
        let mut out = String::new();
        write_json_num(&mut out, 200.0);
        out.push(',');
        write_json_num(&mut out, f32::NAN);
        assert_eq!(out, "200,0");
    }
}
