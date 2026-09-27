//! Document scheduling abstraction
//!
//! Provides a trait-based interface for posting document events,
//! replacing the hardcoded mpsc channel with a more flexible system.

use std::sync::Arc;

use crate::document::DocumentEvent;

/// Trait for document schedulers
///
/// Implementations can route events through different mechanisms:
/// - mpsc channels (default, for backward compatibility)
/// - Tokio channels (for async integration)
/// - Service event bus (for Exosphere integration)
/// - Direct callbacks (for in-process communication)
pub trait DocumentScheduler: Send + Sync + 'static {
    /// Post an event to a document
    ///
    /// The doc_id allows the scheduler to route to the correct document
    /// if managing multiple documents.
    fn post(&self, doc_id: usize, event: DocumentEvent);

    /// Check if the scheduler is still alive/connected
    fn is_alive(&self) -> bool {
        true
    }
}

/// Default scheduler implementation using mpsc channels
///
/// This maintains backward compatibility with the existing bliss-dom behavior.
pub struct MpscDocumentScheduler {
    tx: std::sync::mpsc::Sender<DocumentEvent>,
}

impl MpscDocumentScheduler {
    /// Create a new mpsc scheduler
    pub fn new(tx: std::sync::mpsc::Sender<DocumentEvent>) -> Self {
        Self { tx }
    }

    /// Create a new scheduler with a channel
    pub fn with_channel() -> (Self, std::sync::mpsc::Receiver<DocumentEvent>) {
        let (tx, rx) = std::sync::mpsc::channel();
        (Self::new(tx), rx)
    }
}

impl DocumentScheduler for MpscDocumentScheduler {
    fn post(&self, _doc_id: usize, event: DocumentEvent) {
        // doc_id is ignored for mpsc - the receiver knows which document it belongs to
        let _ = self.tx.send(event);
    }

    fn is_alive(&self) -> bool {
        self.tx.send(DocumentEvent::Ping).is_ok()
    }
}

/// Tokio-based async scheduler
#[cfg(feature = "tokio")]
pub struct TokioDocumentScheduler {
    tx: tokio::sync::mpsc::UnboundedSender<(usize, DocumentEvent)>,
}

#[cfg(feature = "tokio")]
impl TokioDocumentScheduler {
    /// Create a new tokio scheduler
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<(usize, DocumentEvent)>) -> Self {
        Self { tx }
    }

    /// Create a new scheduler with a channel
    pub fn with_channel() -> (
        Self,
        tokio::sync::mpsc::UnboundedReceiver<(usize, DocumentEvent)>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (Self::new(tx), rx)
    }
}

#[cfg(feature = "tokio")]
impl DocumentScheduler for TokioDocumentScheduler {
    fn post(&self, doc_id: usize, event: DocumentEvent) {
        let _ = self.tx.send((doc_id, event));
    }

    fn is_alive(&self) -> bool {
        !self.tx.is_closed()
    }
}

/// Callback-based scheduler for direct integration
pub struct CallbackDocumentScheduler {
    callback: Box<dyn Fn(usize, DocumentEvent) + Send + Sync>,
}

impl CallbackDocumentScheduler {
    /// Create a new callback scheduler
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(usize, DocumentEvent) + Send + Sync + 'static,
    {
        Self {
            callback: Box::new(callback),
        }
    }
}

impl DocumentScheduler for CallbackDocumentScheduler {
    fn post(&self, doc_id: usize, event: DocumentEvent) {
        (self.callback)(doc_id, event);
    }
}

/// No-op scheduler that drops all events
pub struct NoopDocumentScheduler;

impl DocumentScheduler for NoopDocumentScheduler {
    fn post(&self, _doc_id: usize, _event: DocumentEvent) {
        // Intentionally drop all events
    }
}

/// Shared document scheduler type
pub type SharedDocumentScheduler = Arc<dyn DocumentScheduler>;

/// Create a default mpsc scheduler
pub fn create_default_scheduler() -> (
    SharedDocumentScheduler,
    std::sync::mpsc::Receiver<DocumentEvent>,
) {
    let (scheduler, rx) = MpscDocumentScheduler::with_channel();
    (Arc::new(scheduler), rx)
}

/// Create a callback-based scheduler
pub fn create_callback_scheduler<F>(callback: F) -> SharedDocumentScheduler
where
    F: Fn(usize, DocumentEvent) + Send + Sync + 'static,
{
    Arc::new(CallbackDocumentScheduler::new(callback))
}

/// Create a no-op scheduler
pub fn create_noop_scheduler() -> SharedDocumentScheduler {
    Arc::new(NoopDocumentScheduler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::{Resource, ResourceLoadResponse};

    fn make_test_event() -> DocumentEvent {
        DocumentEvent::ResourceLoad(ResourceLoadResponse {
            request_id: 0,
            node_id: Some(42),
            resolved_url: Some("test.png".to_string()),
            result: Ok(Resource::None),
        })
    }

    #[test]
    fn test_mpsc_scheduler() {
        let (scheduler, rx) = MpscDocumentScheduler::with_channel();

        scheduler.post(1, make_test_event());

        // Receive the event
        let received = rx.recv().unwrap();
        match received {
            DocumentEvent::ResourceLoad(recv) => {
                assert_eq!(recv.node_id, Some(42));
                assert_eq!(recv.resolved_url.as_deref(), Some("test.png"));
            }
            DocumentEvent::Ping => panic!("Expected ResourceLoad, got Ping"),
        }
    }

    #[test]
    fn test_callback_scheduler() {
        let received = std::sync::Arc::new(std::sync::Mutex::new(None));
        let received_clone = received.clone();

        let scheduler = CallbackDocumentScheduler::new(move |doc_id, event| {
            *received_clone.lock().unwrap_or_else(|e| e.into_inner()) = Some((doc_id, event));
        });

        scheduler.post(123, make_test_event());

        let result = received.lock().unwrap_or_else(|e| e.into_inner()).take();
        assert!(result.is_some());
        let (doc_id, _) = result.unwrap();
        assert_eq!(doc_id, 123);
    }

    #[test]
    fn test_noop_scheduler() {
        let scheduler = NoopDocumentScheduler;

        // Should not panic or do anything
        scheduler.post(1, make_test_event());
    }

    #[test]
    fn test_shared_scheduler() {
        let (scheduler, rx) = create_default_scheduler();

        scheduler.post(1, make_test_event());

        // Should be able to receive
        let received = rx.recv();
        assert!(received.is_ok());
    }
}
