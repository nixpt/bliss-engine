//! DOM control abstraction for external manipulation of a Bliss document
//!
//! This trait provides a stable API for querying and mutating the DOM
//! from outside the rendering engine (e.g., from Exosphere's dom-capability crate).

use std::collections::HashMap;

pub type NodeId = usize;

#[derive(Debug, Clone)]
pub enum DomControlError {
    NodeNotFound(NodeId),
    InvalidSelector(String),
    InvalidMutation(String),
    PermissionDenied(String),
}

impl std::fmt::Display for DomControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(f, "node not found: {}", id),
            Self::InvalidSelector(s) => write!(f, "invalid selector: {}", s),
            Self::InvalidMutation(m) => write!(f, "invalid mutation: {}", m),
            Self::PermissionDenied(r) => write!(f, "permission denied: {}", r),
        }
    }
}

impl std::error::Error for DomControlError {}

pub type DomControlResult<T> = Result<T, DomControlError>;

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: NodeId,
    pub tag_name: Option<String>,
    pub text_content: Option<String>,
    pub attributes: HashMap<String, String>,
}

pub trait DomController {
    fn query_selector(&self, selector: &str) -> DomControlResult<Option<NodeId>>;

    fn query_selector_all(&self, selector: &str) -> DomControlResult<Vec<NodeId>>;

    fn get_element_by_id(&self, id: &str) -> Option<NodeId>;

    fn get_node_info(&self, node_id: NodeId) -> DomControlResult<NodeInfo>;

    fn set_attribute(&mut self, node_id: NodeId, name: &str, value: &str) -> DomControlResult<()>;

    fn remove_attribute(&mut self, node_id: NodeId, name: &str) -> DomControlResult<()>;

    fn set_text_content(&mut self, node_id: NodeId, text: &str) -> DomControlResult<()>;

    fn set_style_property(
        &mut self,
        node_id: NodeId,
        property: &str,
        value: &str,
    ) -> DomControlResult<()>;

    fn remove_style_property(&mut self, node_id: NodeId, property: &str) -> DomControlResult<()>;

    fn set_inner_html(&mut self, node_id: NodeId, html: &str) -> DomControlResult<()>;

    fn add_event_listener(
        &mut self,
        node_id: NodeId,
        event: &str,
        handler_id: u64,
    ) -> DomControlResult<()>;

    fn remove_event_listener(
        &mut self,
        node_id: NodeId,
        event: &str,
        handler_id: u64,
    ) -> DomControlResult<()>;
}

pub trait DomCapabilityPolicy: Send + Sync {
    fn allow_query(&self, doc_id: usize, selector: &str) -> bool;
    fn allow_mutation(&self, doc_id: usize, node_id: NodeId, op: &str) -> bool;
    fn allow_event_listener(&self, doc_id: usize, node_id: NodeId, event: &str) -> bool;
}

pub struct DefaultDomPolicy;
impl DomCapabilityPolicy for DefaultDomPolicy {
    fn allow_query(&self, _doc_id: usize, _selector: &str) -> bool {
        true
    }
    fn allow_mutation(&self, _doc_id: usize, _node_id: NodeId, _op: &str) -> bool {
        true
    }
    fn allow_event_listener(&self, _doc_id: usize, _node_id: NodeId, _event: &str) -> bool {
        true
    }
}
