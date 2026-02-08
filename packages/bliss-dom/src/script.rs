//! Script engine abstraction for bliss-dom
//!
//! This trait allows pluggable script execution engines (Boa, V8, NanoVM, etc.)
//! to be integrated with the DOM. Script engines can execute JavaScript, Lua,
//! Python, or other languages and handle DOM events.

use crate::BaseDocument;
use bliss_traits::events::DomEvent;

/// Result of script execution
#[derive(Debug, Clone)]
pub enum ScriptValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Object(Vec<(String, ScriptValue)>),
    Promise(u64), // Async result handle
}

/// Script execution errors
#[derive(Debug, Clone)]
pub enum ScriptError {
    ParseError(String),
    RuntimeError(String),
    CapabilityDenied { operation: String, reason: String },
    Timeout,
    MemoryLimitExceeded,
    UnsupportedLanguage(String),
}

/// Whether an event was handled by script
pub enum EventHandled {
    Handled,   // Script consumed the event
    Propagate, // Pass to normal DOM handling
}

/// Language identifiers
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptLanguage {
    JavaScript,
    TypeScript,
    Lua,
    Python,
    Casm, // Direct CASM execution for optimized code
}

impl std::fmt::Display for ScriptLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptLanguage::JavaScript => write!(f, "javascript"),
            ScriptLanguage::TypeScript => write!(f, "typescript"),
            ScriptLanguage::Lua => write!(f, "lua"),
            ScriptLanguage::Python => write!(f, "python"),
            ScriptLanguage::Casm => write!(f, "casm"),
        }
    }
}

/// Context for script execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub source_url: Option<String>,
    pub line_number: u32,
    pub is_module: bool,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            source_url: None,
            line_number: 1,
            is_module: false,
        }
    }
}

/// Type for script error callbacks
pub type ScriptErrorCallback = Box<dyn Fn(&ScriptError) + Send + Sync>;

/// Script engine trait - implement for Boa, V8, NanoVM, etc.
pub trait ScriptEngine: Send {
    /// Initialize the engine with a document
    fn init(&mut self, document: &mut BaseDocument);

    /// Execute code in the specified language
    fn execute(
        &mut self,
        code: &str,
        language: ScriptLanguage,
        context: &ExecutionContext,
    ) -> Result<ScriptValue, ScriptError>;

    /// Handle a DOM event (keyboard, mouse, etc.)
    /// Returns whether the event was consumed
    fn handle_event(&mut self, event: &DomEvent) -> EventHandled;

    /// Poll for async work - called by document poll()
    /// Returns true if more work pending
    fn tick(&mut self) -> Result<bool, ScriptError>;

    /// Register an error callback
    fn set_error_handler(&mut self, callback: Option<ScriptErrorCallback>);
}

/// A no-op script engine for when no scripting is needed
pub struct NoopScriptEngine;

impl ScriptEngine for NoopScriptEngine {
    fn init(&mut self, _document: &mut BaseDocument) {}

    fn execute(
        &mut self,
        _code: &str,
        language: ScriptLanguage,
        _context: &ExecutionContext,
    ) -> Result<ScriptValue, ScriptError> {
        Err(ScriptError::UnsupportedLanguage(language.to_string()))
    }

    fn handle_event(&mut self, _event: &DomEvent) -> EventHandled {
        EventHandled::Propagate
    }

    fn tick(&mut self) -> Result<bool, ScriptError> {
        Ok(false)
    }

    fn set_error_handler(&mut self, _callback: Option<ScriptErrorCallback>) {}
}

/// Boxed script engine for storage in documents
pub type BoxedScriptEngine = Box<dyn ScriptEngine>;
