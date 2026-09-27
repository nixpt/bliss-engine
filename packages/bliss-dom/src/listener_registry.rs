//! Event listener registry for BaseDocument
//!
//! This module provides storage and management of event listeners registered
//! via the DomController API. It supports multiple handler types:
//! - Script callbacks (for ScriptEngine integration)
//! - External handler IDs (for EventSink routing)

use bliss_traits::events::DomEventKind;
use std::collections::HashMap;

/// Unique identifier for an event listener
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerId(pub u64);

/// Type of handler for an event listener
#[derive(Debug, Clone)]
pub enum HandlerType {
    /// Script callback name (for ScriptEngine)
    ScriptCallback(String),
    /// External handler ID (for EventSink routing)
    ExternalHandler(u64),
}

/// Event listener entry
#[derive(Debug, Clone)]
pub struct EventListener {
    pub node_id: usize,
    pub event_kind: DomEventKind,
    pub handler: HandlerType,
    pub listener_id: ListenerId,
    /// Whether this listener is for capture phase
    pub use_capture: bool,
    /// Whether this listener should only fire once
    pub once: bool,
}

/// Registry for event listeners
///
/// Stores listeners keyed by (node_id, event_kind) for efficient lookup during dispatch
pub struct EventListenerRegistry {
    /// Map from (node_id, event_kind) to list of listener IDs
    listeners: HashMap<(usize, DomEventKind), Vec<ListenerId>>,
    /// Map from listener_id to actual listener
    listener_data: HashMap<ListenerId, EventListener>,
    /// Counter for generating unique listener IDs
    next_id: u64,
}

impl Default for EventListenerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EventListenerRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
            listener_data: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add an event listener
    pub fn add_listener(
        &mut self,
        node_id: usize,
        event_kind: DomEventKind,
        handler: HandlerType,
        use_capture: bool,
        once: bool,
    ) -> ListenerId {
        let listener_id = ListenerId(self.next_id);
        self.next_id += 1;

        let listener = EventListener {
            node_id,
            event_kind,
            handler,
            listener_id,
            use_capture,
            once,
        };

        let key = (node_id, event_kind);
        self.listeners.entry(key).or_default().push(listener_id);
        self.listener_data.insert(listener_id, listener);

        listener_id
    }

    /// Remove a specific event listener
    pub fn remove_listener(
        &mut self,
        node_id: usize,
        event_kind: DomEventKind,
        listener_id: ListenerId,
    ) -> bool {
        let key = (node_id, event_kind);

        // Remove from the key->listeners map
        if let Some(listener_list) = self.listeners.get_mut(&key) {
            listener_list.retain(|&id| id != listener_id);
            if listener_list.is_empty() {
                self.listeners.remove(&key);
            }
        }

        // Remove the listener data
        self.listener_data.remove(&listener_id).is_some()
    }

    /// Get all listeners for a specific node and event kind
    pub fn get_listeners(&self, node_id: usize, event_kind: DomEventKind) -> Vec<&EventListener> {
        let key = (node_id, event_kind);
        self.listeners
            .get(&key)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.listener_data.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all listeners for a node (all event kinds)
    pub fn get_node_listeners(&self, node_id: usize) -> Vec<&EventListener> {
        self.listener_data
            .values()
            .filter(|l| l.node_id == node_id)
            .collect()
    }

    /// Remove all listeners for a node (called when node is removed from DOM)
    pub fn remove_node_listeners(&mut self, node_id: usize) -> usize {
        let to_remove: Vec<ListenerId> = self
            .listener_data
            .values()
            .filter(|l| l.node_id == node_id)
            .map(|l| l.listener_id)
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            if let Some(listener) = self.listener_data.remove(&id) {
                let key = (listener.node_id, listener.event_kind);
                if let Some(list) = self.listeners.get_mut(&key) {
                    list.retain(|&lid| lid != id);
                    if list.is_empty() {
                        self.listeners.remove(&key);
                    }
                }
            }
        }

        count
    }

    /// Remove all listeners marked as "once" after they've been triggered
    pub fn remove_once_listeners(&mut self, listener_ids: &[ListenerId]) {
        for id in listener_ids {
            if let Some(listener) = self.listener_data.get(id) {
                if listener.once {
                    let key = (listener.node_id, listener.event_kind);
                    if let Some(list) = self.listeners.get_mut(&key) {
                        list.retain(|&lid| lid != *id);
                        if list.is_empty() {
                            self.listeners.remove(&key);
                        }
                    }
                    self.listener_data.remove(id);
                }
            }
        }
    }

    /// Clear all listeners
    pub fn clear(&mut self) {
        self.listeners.clear();
        self.listener_data.clear();
        self.next_id = 1;
    }

    /// Get the total number of registered listeners
    pub fn len(&self) -> usize {
        self.listener_data.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.listener_data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_remove_listener() {
        let mut registry = EventListenerRegistry::new();

        let id = registry.add_listener(
            1,
            DomEventKind::Click,
            HandlerType::ExternalHandler(42),
            false,
            false,
        );

        assert_eq!(registry.len(), 1);

        let removed = registry.remove_listener(1, DomEventKind::Click, id);
        assert!(removed);
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_get_listeners() {
        let mut registry = EventListenerRegistry::new();

        registry.add_listener(
            1,
            DomEventKind::Click,
            HandlerType::ExternalHandler(42),
            false,
            false,
        );

        registry.add_listener(
            1,
            DomEventKind::Click,
            HandlerType::ScriptCallback("onClick".to_string()),
            false,
            false,
        );

        registry.add_listener(
            1,
            DomEventKind::MouseOver,
            HandlerType::ExternalHandler(43),
            false,
            false,
        );

        let click_listeners = registry.get_listeners(1, DomEventKind::Click);
        assert_eq!(click_listeners.len(), 2);

        let mouseover_listeners = registry.get_listeners(1, DomEventKind::MouseOver);
        assert_eq!(mouseover_listeners.len(), 1);
    }

    #[test]
    fn test_remove_node_listeners() {
        let mut registry = EventListenerRegistry::new();

        registry.add_listener(
            1,
            DomEventKind::Click,
            HandlerType::ExternalHandler(1),
            false,
            false,
        );
        registry.add_listener(
            1,
            DomEventKind::MouseOver,
            HandlerType::ExternalHandler(2),
            false,
            false,
        );
        registry.add_listener(
            2,
            DomEventKind::Click,
            HandlerType::ExternalHandler(3),
            false,
            false,
        );

        let removed = registry.remove_node_listeners(1);
        assert_eq!(removed, 2);
        assert_eq!(registry.len(), 1);

        let remaining = registry.get_listeners(2, DomEventKind::Click);
        assert_eq!(remaining.len(), 1);
    }
}
