//! DomController implementation for BaseDocument
//!
//! Provides a stable API for querying and mutating the DOM from outside the rendering engine.

use bliss_traits::dom_control::{
    DomControlError, DomControlResult, DomController, NodeId, NodeInfo,
};
use markup5ever::local_name;

use crate::{BaseDocument, NodeData};

impl DomController for BaseDocument {
    fn query_selector(&self, selector: &str) -> DomControlResult<Option<NodeId>> {
        self.query_selector(selector)
            .map_err(|e| DomControlError::InvalidSelector(format!("{:?}", e)))
    }

    fn query_selector_all(&self, selector: &str) -> DomControlResult<Vec<NodeId>> {
        self.query_selector_all(selector)
            .map_err(|e| DomControlError::InvalidSelector(format!("{:?}", e)))
            .map(|smallvec| smallvec.into_vec())
    }

    fn get_element_by_id(&self, id: &str) -> Option<NodeId> {
        self.get_element_by_id(id)
    }

    fn get_node_info(&self, node_id: NodeId) -> DomControlResult<NodeInfo> {
        let node = self
            .get_node(node_id)
            .ok_or(DomControlError::NodeNotFound(node_id))?;

        let tag_name = node.element_data().map(|el| el.name.local.to_string());

        let text_content = match &node.data {
            NodeData::Text(text_data) => Some(text_data.content.clone()),
            _ => {
                // For elements, collect text from all child text nodes
                let mut text = String::new();
                for child_id in &node.children {
                    if let Some(child) = self.get_node(*child_id) {
                        if let NodeData::Text(text_data) = &child.data {
                            text.push_str(&text_data.content);
                        }
                    }
                }
                if text.is_empty() { None } else { Some(text) }
            }
        };

        let attributes = node
            .element_data()
            .map(|el| {
                el.attrs()
                    .iter()
                    .map(|attr| (attr.name.local.to_string(), attr.value.clone()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(NodeInfo {
            id: node_id,
            tag_name,
            text_content,
            attributes,
        })
    }

    fn set_attribute(&mut self, node_id: NodeId, name: &str, value: &str) -> DomControlResult<()> {
        let node = self
            .get_node_mut(node_id)
            .ok_or(DomControlError::NodeNotFound(node_id))?;

        let element = node.element_data_mut().ok_or_else(|| {
            DomControlError::InvalidMutation(format!("Node {} is not an element", node_id))
        })?;

        // Check if this is the id attribute - need to update nodes_to_id map
        if name == "id" {
            // Note: We can't update nodes_to_id here because self is already mutably borrowed
            // This would need to be handled at a higher level or with interior mutability
            // For now, just update the element's id
            element.id = Some(style::Atom::from(value));
        }

        // Find and update or add the attribute
        let local_name = local_name!(name);
        if let Some(attr) = element
            .attrs
            .iter_mut()
            .find(|a| a.name.local == local_name)
        {
            attr.value = value.to_string();
        } else {
            use crate::Attribute;
            use crate::qual_name;
            element.attrs.push(Attribute {
                name: qual_name!(name),
                value: value.to_string(),
            });
        }

        // Mark node for restyling if needed
        node.set_restyle_hint(
            style::invalidation::element::restyle_hints::RestyleHint::restyle_subtree(),
        );

        // Update nodes_to_id after releasing the mutable borrow
        if name == "id" {
            self.nodes_to_id.insert(value.to_string(), node_id);
        }

        Ok(())
    }

    fn remove_attribute(&mut self, node_id: NodeId, name: &str) -> DomControlResult<()> {
        let node = self
            .get_node_mut(node_id)
            .ok_or(DomControlError::NodeNotFound(node_id))?;

        let element = node.element_data_mut().ok_or_else(|| {
            DomControlError::InvalidMutation(format!("Node {} is not an element", node_id))
        })?;

        // Check if this is the id attribute
        let is_id = name == "id";
        let old_id = if is_id { element.id.clone() } else { None };

        let local_name = local_name!(name);
        element.attrs.retain(|attr| attr.name.local != local_name);

        if is_id {
            element.id = None;
        }

        // Mark node for restyling
        node.set_restyle_hint(
            style::invalidation::element::restyle_hints::RestyleHint::restyle_subtree(),
        );

        // Update nodes_to_id after releasing the mutable borrow
        if is_id {
            if let Some(id) = old_id {
                self.nodes_to_id.remove(&id.to_string());
            }
        }

        Ok(())
    }

    fn set_text_content(&mut self, node_id: NodeId, text: &str) -> DomControlResult<()> {
        let node = self
            .get_node_mut(node_id)
            .ok_or(DomControlError::NodeNotFound(node_id))?;

        match &mut node.data {
            NodeData::Text(text_data) => {
                text_data.content = text.to_string();
            }
            NodeData::Element(_) | NodeData::AnonymousBlock(_) => {
                // For elements, clear children and add a single text node
                // Note: This is a simplified implementation - a full implementation
                // would need to handle this more carefully
                return Err(DomControlError::InvalidMutation(
                    "set_text_content on elements requires child node manipulation".to_string(),
                ));
            }
            _ => {
                return Err(DomControlError::InvalidMutation(
                    "Cannot set text content on this node type".to_string(),
                ));
            }
        }

        // Mark node for restyle
        node.set_restyle_hint(
            style::invalidation::element::restyle_hints::RestyleHint::restyle_subtree(),
        );

        Ok(())
    }

    fn set_style_property(
        &mut self,
        node_id: NodeId,
        property: &str,
        value: &str,
    ) -> DomControlResult<()> {
        // Clone needed data first to avoid borrow issues
        let guard = self.guard.clone();
        let url_extra_data = self.url.url_extra_data().clone();

        let node = self
            .get_node_mut(node_id)
            .ok_or(DomControlError::NodeNotFound(node_id))?;

        let element = node.element_data_mut().ok_or_else(|| {
            DomControlError::InvalidMutation(format!("Node {} is not an element", node_id))
        })?;

        element.set_style_property(property, value, &guard, url_extra_data);

        // Mark node for restyle
        node.set_restyle_hint(
            style::invalidation::element::restyle_hints::RestyleHint::restyle_subtree(),
        );

        Ok(())
    }

    fn remove_style_property(&mut self, node_id: NodeId, property: &str) -> DomControlResult<()> {
        // Clone needed data first to avoid borrow issues
        let guard = self.guard.clone();
        let url_extra_data = self.url.url_extra_data().clone();

        let node = self
            .get_node_mut(node_id)
            .ok_or(DomControlError::NodeNotFound(node_id))?;

        let element = node.element_data_mut().ok_or_else(|| {
            DomControlError::InvalidMutation(format!("Node {} is not an element", node_id))
        })?;

        element.remove_style_property(property, &guard, url_extra_data);

        // Mark node for restyle
        node.set_restyle_hint(
            style::invalidation::element::restyle_hints::RestyleHint::restyle_subtree(),
        );

        Ok(())
    }

    fn set_inner_html(&mut self, node_id: NodeId, html: &str) -> DomControlResult<()> {
        // Ensure node exists and is an element
        let _ = self
            .get_node(node_id)
            .and_then(|node| node.element_data())
            .ok_or(DomControlError::NodeNotFound(node_id))?;

        let parser = self.html_parser_provider.clone();
        let mut mutr = self.mutate();
        parser.parse_inner_html(&mut mutr, node_id, html);
        Ok(())
    }

    fn add_event_listener(
        &mut self,
        node_id: NodeId,
        event: &str,
        handler_id: u64,
    ) -> DomControlResult<()> {
        // Validate the node exists
        if self.get_node(node_id).is_none() {
            return Err(DomControlError::NodeNotFound(node_id));
        }

        self.event_listeners
            .entry((node_id, event.to_string()))
            .or_default()
            .push(handler_id);

        Ok(())
    }

    fn remove_event_listener(
        &mut self,
        node_id: NodeId,
        event: &str,
        handler_id: u64,
    ) -> DomControlResult<()> {
        // Validate the node exists
        if self.get_node(node_id).is_none() {
            return Err(DomControlError::NodeNotFound(node_id));
        }

        if let Some(handlers) = self.event_listeners.get_mut(&(node_id, event.to_string())) {
            handlers.retain(|&id| id != handler_id);
            if handlers.is_empty() {
                self.event_listeners.remove(&(node_id, event.to_string()));
            }
        }

        Ok(())
    }
}

/// A wrapper that combines BaseDocument with a policy for capability-based control
pub struct PolicyDomController<'a> {
    doc: &'a mut BaseDocument,
    policy: std::sync::Arc<dyn bliss_traits::dom_control::DomCapabilityPolicy>,
    doc_id: usize,
}

impl<'a> PolicyDomController<'a> {
    pub fn new(
        doc: &'a mut BaseDocument,
        policy: std::sync::Arc<dyn bliss_traits::dom_control::DomCapabilityPolicy>,
        doc_id: usize,
    ) -> Self {
        Self {
            doc,
            policy,
            doc_id,
        }
    }
}

impl<'a> DomController for PolicyDomController<'a> {
    fn query_selector(&self, selector: &str) -> DomControlResult<Option<NodeId>> {
        if !self.policy.allow_query(self.doc_id, selector) {
            return Err(DomControlError::PermissionDenied(format!(
                "Query not allowed: {}",
                selector
            )));
        }
        self.doc
            .query_selector(selector)
            .map_err(|e| DomControlError::InvalidSelector(format!("{:?}", e)))
    }

    fn query_selector_all(&self, selector: &str) -> DomControlResult<Vec<NodeId>> {
        if !self.policy.allow_query(self.doc_id, selector) {
            return Err(DomControlError::PermissionDenied(format!(
                "Query not allowed: {}",
                selector
            )));
        }
        self.doc
            .query_selector_all(selector)
            .map_err(|e| DomControlError::InvalidSelector(format!("{:?}", e)))
            .map(|smallvec| smallvec.into_vec())
    }

    fn get_element_by_id(&self, id: &str) -> Option<NodeId> {
        // ID lookup is generally safe, but could be policy-gated if needed
        self.doc.get_element_by_id(id)
    }

    fn get_node_info(&self, node_id: NodeId) -> DomControlResult<NodeInfo> {
        // Node info is read-only, generally safe
        self.doc.get_node_info(node_id)
    }

    fn set_attribute(&mut self, node_id: NodeId, name: &str, value: &str) -> DomControlResult<()> {
        if !self
            .policy
            .allow_mutation(self.doc_id, node_id, "set_attribute")
        {
            return Err(DomControlError::PermissionDenied(
                "set_attribute not allowed".to_string(),
            ));
        }
        self.doc.set_attribute(node_id, name, value)
    }

    fn remove_attribute(&mut self, node_id: NodeId, name: &str) -> DomControlResult<()> {
        if !self
            .policy
            .allow_mutation(self.doc_id, node_id, "remove_attribute")
        {
            return Err(DomControlError::PermissionDenied(
                "remove_attribute not allowed".to_string(),
            ));
        }
        self.doc.remove_attribute(node_id, name)
    }

    fn set_text_content(&mut self, node_id: NodeId, text: &str) -> DomControlResult<()> {
        if !self
            .policy
            .allow_mutation(self.doc_id, node_id, "set_text_content")
        {
            return Err(DomControlError::PermissionDenied(
                "set_text_content not allowed".to_string(),
            ));
        }
        self.doc.set_text_content(node_id, text)
    }

    fn set_style_property(
        &mut self,
        node_id: NodeId,
        property: &str,
        value: &str,
    ) -> DomControlResult<()> {
        if !self
            .policy
            .allow_mutation(self.doc_id, node_id, "set_style_property")
        {
            return Err(DomControlError::PermissionDenied(
                "set_style_property not allowed".to_string(),
            ));
        }
        self.doc.set_style_property(node_id, property, value);
        Ok(())
    }

    fn remove_style_property(&mut self, node_id: NodeId, property: &str) -> DomControlResult<()> {
        if !self
            .policy
            .allow_mutation(self.doc_id, node_id, "remove_style_property")
        {
            return Err(DomControlError::PermissionDenied(
                "remove_style_property not allowed".to_string(),
            ));
        }
        self.doc.remove_style_property(node_id, property);
        Ok(())
    }

    fn set_inner_html(&mut self, node_id: NodeId, html: &str) -> DomControlResult<()> {
        if !self
            .policy
            .allow_mutation(self.doc_id, node_id, "set_inner_html")
        {
            return Err(DomControlError::PermissionDenied(
                "set_inner_html not allowed".to_string(),
            ));
        }
        self.doc.set_inner_html(node_id, html)
    }

    fn add_event_listener(
        &mut self,
        node_id: NodeId,
        event: &str,
        handler_id: u64,
    ) -> DomControlResult<()> {
        if !self
            .policy
            .allow_event_listener(self.doc_id, node_id, event)
        {
            return Err(DomControlError::PermissionDenied(format!(
                "add_event_listener not allowed for {}",
                event
            )));
        }
        self.doc.add_event_listener(node_id, event, handler_id)
    }

    fn remove_event_listener(
        &mut self,
        node_id: NodeId,
        event: &str,
        handler_id: u64,
    ) -> DomControlResult<()> {
        if !self
            .policy
            .allow_event_listener(self.doc_id, node_id, event)
        {
            return Err(DomControlError::PermissionDenied(format!(
                "remove_event_listener not allowed for {}",
                event
            )));
        }
        self.doc.remove_event_listener(node_id, event, handler_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentConfig, ElementData, NodeData, qual_name};

    #[test]
    fn test_dom_controller_query_selector() {
        let mut doc = BaseDocument::new(DocumentConfig::default());

        // Create a simple DOM: <div id="test"><span class="item">Hello</span></div>
        let div = doc.create_node(NodeData::Element(ElementData::new(
            qual_name!("div"),
            vec![],
        )));
        let span = doc.create_node(NodeData::Element(ElementData::new(
            qual_name!("span"),
            vec![],
        )));
        let text = doc.create_text_node("Hello");

        // Build tree structure - connect div to document root first
        doc.root_node_mut().children.push(div);
        doc.get_node_mut(div).unwrap().parent = Some(0); // Document root is at index 0

        // Connect span to div
        doc.get_node_mut(div).unwrap().children.push(span);
        doc.get_node_mut(span).unwrap().parent = Some(div);

        // Connect text to span
        doc.get_node_mut(span).unwrap().children.push(text);
        doc.get_node_mut(text).unwrap().parent = Some(span);

        // Test query_selector
        let result = doc.query_selector("span").unwrap();
        assert_eq!(result, Some(span));

        let result = doc.query_selector(".nonexistent").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_dom_controller_get_node_info() {
        let mut doc = BaseDocument::new(DocumentConfig::default());

        let div = doc.create_node(NodeData::Element(ElementData::new(
            qual_name!("div"),
            vec![],
        )));

        let info = doc.get_node_info(div).unwrap();
        assert_eq!(info.id, div);
        assert_eq!(info.tag_name, Some("div".to_string()));
    }
}
