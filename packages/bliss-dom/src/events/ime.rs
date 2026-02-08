use bliss_traits::events::{BlissImeEvent, BlissInputEvent, DomEvent, DomEventData};

use crate::BaseDocument;

pub(crate) fn handle_ime_event<F: FnMut(DomEvent)>(
    doc: &mut BaseDocument,
    event: BlissImeEvent,
    mut dispatch_event: F,
) {
    if let Some(node_id) = doc.focus_node_id {
        let node = &mut doc.nodes[node_id];
        let text_input_data = node
            .data
            .downcast_element_mut()
            .and_then(|el| el.text_input_data_mut());
        if let Some(input_data) = text_input_data {
            let editor = &mut input_data.editor;
            let mut font_ctx = doc.font_ctx.lock().unwrap();
            let mut driver = editor.driver(&mut font_ctx, &mut doc.layout_ctx);

            match event {
                BlissImeEvent::Enabled => { /* Do nothing */ }
                BlissImeEvent::Disabled => {
                    driver.clear_compose();
                    doc.shell_provider.request_redraw();
                }
                BlissImeEvent::Commit(text) => {
                    driver.insert_or_replace_selection(&text);
                    let value = input_data.editor.raw_text().to_string();
                    dispatch_event(DomEvent::new(
                        node_id,
                        DomEventData::Input(BlissInputEvent { value }),
                    ));
                    doc.shell_provider.request_redraw();
                }
                BlissImeEvent::Preedit(text, cursor) => {
                    if text.is_empty() {
                        driver.clear_compose();
                    } else {
                        driver.set_compose(&text, cursor);
                    }
                    doc.shell_provider.request_redraw();
                }
                BlissImeEvent::DeleteSurrounding {
                    before_bytes,
                    after_bytes,
                } => {
                    let _ = before_bytes;
                    let _ = after_bytes;
                    // TODO
                }
            }
            println!("Sent ime event to {node_id}");
        }
    }
}
