//! Inter-document messaging
//!
//! Provides a mechanism for documents to communicate with each other,
//! similar to postMessage() between iframes in a browser.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Message that can be sent between documents
#[derive(Debug, Clone)]
pub enum DocumentMessage {
    /// JSON-encoded message
    Json(String),
    /// Raw binary data
    Bytes(Vec<u8>),
    /// Custom message type (for future extensibility)
    Custom { type_id: String, payload: Vec<u8> },
}

/// Trait for document messaging
///
/// Enables communication between documents, useful for:
/// - Capsule-to-capsule communication
/// - Parent-child document relationships (like iframes)
/// - Broadcasting events across documents
pub trait DocumentMessaging: Send + Sync + 'static {
    /// Post a message to a target document
    ///
    /// # Arguments
    /// * `source_doc` - The document ID sending the message
    /// * `target_doc` - The document ID receiving the message
    /// * `message` - The message to send
    fn post_message(&self, source_doc: usize, target_doc: usize, message: DocumentMessage);

    /// Register a message handler for a document
    ///
    /// The handler will be called when messages are received for the document.
    fn register_handler(
        &self,
        doc_id: usize,
        handler: Box<dyn Fn(usize, DocumentMessage) + Send + Sync>,
    );

    /// Unregister the message handler for a document
    fn unregister_handler(&self, doc_id: usize);

    /// Check if a document has a registered handler
    fn has_handler(&self, doc_id: usize) -> bool;

    /// Get the number of registered handlers
    fn handler_count(&self) -> usize;
}

/// Simple in-memory document messaging implementation
pub struct InMemoryDocumentMessaging {
    handlers: Mutex<HashMap<usize, Box<dyn Fn(usize, DocumentMessage) + Send + Sync>>>,
}

impl InMemoryDocumentMessaging {
    /// Create a new in-memory messaging system
    pub fn new() -> Self {
        Self {
            handlers: Mutex::new(HashMap::new()),
        }
    }

    /// Create a new messaging system wrapped in an Arc
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl Default for InMemoryDocumentMessaging {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentMessaging for InMemoryDocumentMessaging {
    fn post_message(&self, source_doc: usize, target_doc: usize, message: DocumentMessage) {
        let handlers = self.handlers.lock().unwrap_or_else(|e| e.into_inner());

        if let Some(handler) = handlers.get(&target_doc) {
            // Call the handler with the source document ID
            handler(source_doc, message);
        }
        // If no handler registered, message is silently dropped
        // (similar to postMessage to non-existent iframe)
    }

    fn register_handler(
        &self,
        doc_id: usize,
        handler: Box<dyn Fn(usize, DocumentMessage) + Send + Sync>,
    ) {
        let mut handlers = self.handlers.lock().unwrap_or_else(|e| e.into_inner());
        handlers.insert(doc_id, handler);
    }

    fn unregister_handler(&self, doc_id: usize) {
        let mut handlers = self.handlers.lock().unwrap_or_else(|e| e.into_inner());
        handlers.remove(&doc_id);
    }

    fn has_handler(&self, doc_id: usize) -> bool {
        let handlers = self.handlers.lock().unwrap_or_else(|e| e.into_inner());
        handlers.contains_key(&doc_id)
    }

    fn handler_count(&self) -> usize {
        let handlers = self.handlers.lock().unwrap_or_else(|e| e.into_inner());
        handlers.len()
    }
}

/// Message router that can filter or transform messages
pub struct FilteringDocumentMessaging<F> {
    inner: Arc<dyn DocumentMessaging>,
    filter: F,
}

impl<F> FilteringDocumentMessaging<F>
where
    F: Fn(usize, usize, &DocumentMessage) -> bool + Send + Sync + 'static,
{
    /// Create a new filtering messaging wrapper
    pub fn new(inner: Arc<dyn DocumentMessaging>, filter: F) -> Self {
        Self { inner, filter }
    }
}

impl<F> DocumentMessaging for FilteringDocumentMessaging<F>
where
    F: Fn(usize, usize, &DocumentMessage) -> bool + Send + Sync + 'static,
{
    fn post_message(&self, source_doc: usize, target_doc: usize, message: DocumentMessage) {
        if (self.filter)(source_doc, target_doc, &message) {
            self.inner.post_message(source_doc, target_doc, message);
        }
    }

    fn register_handler(
        &self,
        doc_id: usize,
        handler: Box<dyn Fn(usize, DocumentMessage) + Send + Sync>,
    ) {
        self.inner.register_handler(doc_id, handler);
    }

    fn unregister_handler(&self, doc_id: usize) {
        self.inner.unregister_handler(doc_id);
    }

    fn has_handler(&self, doc_id: usize) -> bool {
        self.inner.has_handler(doc_id)
    }

    fn handler_count(&self) -> usize {
        self.inner.handler_count()
    }
}

/// Create a filtering messaging system that only allows same-origin messages
pub fn create_same_origin_filter(inner: Arc<dyn DocumentMessaging>) -> Arc<dyn DocumentMessaging> {
    Arc::new(FilteringDocumentMessaging::new(
        inner,
        |_source, _target, _msg| {
            // In a real implementation, this would check if documents
            // share the same origin/capsule
            true // Allow all for now
        },
    ))
}

/// Shared document messaging type
pub type SharedDocumentMessaging = Arc<dyn DocumentMessaging>;

/// Create default in-memory messaging
pub fn create_default_messaging() -> SharedDocumentMessaging {
    InMemoryDocumentMessaging::new_shared()
}

/// Message envelope with metadata
#[derive(Debug, Clone)]
pub struct MessageEnvelope {
    /// Source document ID
    pub source: usize,
    /// Target document ID
    pub target: usize,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
    /// The actual message
    pub message: DocumentMessage,
}

/// Helper functions for creating common message types
pub mod message_helpers {
    use super::*;

    /// Create a binary message
    pub fn bytes(data: Vec<u8>) -> DocumentMessage {
        DocumentMessage::Bytes(data)
    }

    /// Create a text message
    pub fn text(text: impl Into<String>) -> DocumentMessage {
        DocumentMessage::Json(format!("\"{}\"", text.into()))
    }

    /// Create a simple event message
    pub fn event(event_type: impl Into<String>) -> DocumentMessage {
        DocumentMessage::Json(format!("{{\"type\":\"{}\"}}", event_type.into()))
    }

    /// Create a JSON message from a pre-serialized string
    pub fn json(json_str: impl Into<String>) -> DocumentMessage {
        DocumentMessage::Json(json_str.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_messaging() {
        let messaging = InMemoryDocumentMessaging::new();

        let received = Arc::new(Mutex::new(None));
        let received_clone = received.clone();

        // Register handler for document 2
        messaging.register_handler(
            2,
            Box::new(move |source, msg| {
                *received_clone.lock().unwrap_or_else(|e| e.into_inner()) = Some((source, msg));
            }),
        );

        // Send message from doc 1 to doc 2
        let message = DocumentMessage::Json(r#"{"hello": "world"}"#.to_string());
        messaging.post_message(1, 2, message.clone());

        // Check message was received
        let result = received.lock().unwrap_or_else(|e| e.into_inner()).clone();
        assert!(result.is_some());
        let (source, received_msg) = result.unwrap();
        assert_eq!(source, 1);
        match received_msg {
            DocumentMessage::Json(json) => {
                assert_eq!(json, r#"{"hello": "world"}"#);
            }
            _ => panic!("Expected JSON message"),
        }
    }

    #[test]
    fn test_unregister_handler() {
        let messaging = InMemoryDocumentMessaging::new();

        messaging.register_handler(1, Box::new(|_, _| {}));
        assert!(messaging.has_handler(1));

        messaging.unregister_handler(1);
        assert!(!messaging.has_handler(1));
    }

    #[test]
    fn test_message_to_nonexistent_handler() {
        let messaging = InMemoryDocumentMessaging::new();

        // Should not panic when sending to non-existent handler
        let message = DocumentMessage::Json(r#"{"test": true}"#.to_string());
        messaging.post_message(1, 999, message);

        // Handler count should be 0
        assert_eq!(messaging.handler_count(), 0);
    }

    #[test]
    fn test_handler_count() {
        let messaging = InMemoryDocumentMessaging::new();

        assert_eq!(messaging.handler_count(), 0);

        messaging.register_handler(1, Box::new(|_, _| {}));
        assert_eq!(messaging.handler_count(), 1);

        messaging.register_handler(2, Box::new(|_, _| {}));
        assert_eq!(messaging.handler_count(), 2);

        messaging.unregister_handler(1);
        assert_eq!(messaging.handler_count(), 1);
    }

    #[test]
    fn test_filtering_messaging() {
        let inner = create_default_messaging();

        // Create a filter that blocks messages to doc 3
        let filter = FilteringDocumentMessaging::new(inner.clone(), |_, target, _| target != 3);

        let received_doc2 = Arc::new(Mutex::new(false));
        let received_doc2_clone = received_doc2.clone();

        let received_doc3 = Arc::new(Mutex::new(false));
        let received_doc3_clone = received_doc3.clone();

        inner.register_handler(
            2,
            Box::new(move |_, _| {
                *received_doc2_clone.lock().unwrap_or_else(|e| e.into_inner()) = true;
            }),
        );

        inner.register_handler(
            3,
            Box::new(move |_, _| {
                *received_doc3_clone.lock().unwrap_or_else(|e| e.into_inner()) = true;
            }),
        );

        let message = DocumentMessage::Json(r#"{}"#.to_string());

        // This should go through
        filter.post_message(1, 2, message.clone());

        // This should be blocked
        filter.post_message(1, 3, message);

        assert!(*received_doc2.lock().unwrap_or_else(|e| e.into_inner()));
        assert!(!*received_doc3.lock().unwrap_or_else(|e| e.into_inner())); // Should remain false
    }

    #[test]
    fn test_message_helpers() {
        // Test text message
        let msg = message_helpers::text("hello");
        match msg {
            DocumentMessage::Json(json) => assert!(json.contains("hello")),
            _ => panic!("Expected JSON"),
        }

        // Test bytes message
        let msg = message_helpers::bytes(vec![1, 2, 3]);
        match msg {
            DocumentMessage::Bytes(bytes) => assert_eq!(bytes, vec![1, 2, 3]),
            _ => panic!("Expected Bytes"),
        }

        // Test event message
        let msg = message_helpers::event("click");
        match msg {
            DocumentMessage::Json(json) => assert!(json.contains("click")),
            _ => panic!("Expected JSON"),
        }

        // Test json message
        let msg = message_helpers::json(r#"{"custom": true}"#);
        match msg {
            DocumentMessage::Json(json) => assert_eq!(json, r#"{"custom": true}"#),
            _ => panic!("Expected JSON"),
        }
    }
}
