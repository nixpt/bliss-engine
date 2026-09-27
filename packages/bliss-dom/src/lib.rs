//! The core DOM abstraction in Bliss
//!
//! This crate implements a flexible headless DOM ([`BaseDocument`]), designed to be embedded in and driven by external code.
//! Most users will want to use a wrapper:
//!
//! - [`HtmlDocument`](https://docs.rs/bliss-html/latest/bliss_html/struct.HtmlDocument.html) from [bliss-html](https://docs.rs/bliss-html) — parse HTML into a `BaseDocument`
//! - [`DioxusDocument`](https://docs.rs/dioxus-native/latest/dioxus_native/struct.DioxusDocument.html) from [dioxus-native](https://docs.rs/dioxus-native) — dynamic rendering with Dioxus
//!
//! This crate provides: DOM tree construction and mutation, CSS parsing and style resolution,
//! layout computation (via taffy), text layout (via parley), event handling, form submission,
//! script execution, text selection, and URL resolution.
//!
//! Companion crates: [bliss-html](https://docs.rs/bliss-html) (parsing), [bliss-net](https://docs.rs/bliss-net) (networking),
//! [bliss-paint](https://docs.rs/bliss-paint) (rendering), [bliss-shell](https://docs.rs/bliss-shell) (windowing).
//!
//! ## Feature Flags
//!
//! - `tracing`: Enable `tracing` spans for performance instrumentation
//! - `incremental`: Enable incremental layout with damage propagation (default: full relayout)
//! - `parallel-construct`: Parallelize inline layout construction with rayon
//! - `accessibility`: Enable accessibility tree support
//! - `file_input`: Enable file upload form controls
//! - `serde`: Enable serialization support

pub const DEFAULT_CSS: &str = include_str!("../assets/default.css");
pub const BULLET_FONT: &[u8] = include_bytes!("../assets/moz-bullet-font.otf");

const INCREMENTAL: bool = cfg!(feature = "incremental");
const NON_INCREMENTAL: bool = !INCREMENTAL;

/// The core DOM implementation: [`BaseDocument`], [`DocGuard`], [`DocGuardMut`], [`Document`], [`PlainDocument`].
///
/// `BaseDocument` manages the node tree, style system, layout state, event handlers,
/// and all cross-cutting concerns. Most of the high-level APIs live as methods on this struct.
mod document;

/// DOM node types: [`Node`], [`NodeData`], [`ElementData`], [`Attribute`], [`TextNodeData`].
///
/// Nodes are stored in [`BaseDocument`]'s arena and referenced by `usize` IDs.
/// `ElementData` holds tag names, attributes, computed styles, layout state, and inline text data.
pub mod node;

/// Document construction options: [`DocumentConfig`].
///
/// Controls viewport, base URL, user-agent stylesheets, font context, and provider
/// implementations for networking, navigation, shell, and HTML parsing.
mod config;
/// Debug and inspection utilities: `print_taffy_tree`, `debug_log_node`.
///
/// Methods on [`BaseDocument`] for dumping layout trees, inline content,
/// and node metadata to stdout for debugging purposes.
mod debug;
/// Policy-based DOM controller: [`PolicyDomController`].
///
/// Provides sandboxing and security restrictions on DOM mutations.
/// Used to enforce content-security policies and prevent unauthorized
/// script-driven modifications.
mod dom_control;
/// DOM event system: [`EventDriver`], [`EventHandler`], [`NoopEventHandler`].
///
/// Sub-modules:
/// - `driver`: Event dispatch and lifecycle
/// - `focus`: Focus management (tab navigation, focusin/focusout)
/// - `keyboard`: Keyboard event handling
/// - `pointer`: Pointer/mouse/touch event handling
/// - `ime`: Input method editor composition events
mod events;
/// Font metrics resolution for text measurement.
///
/// Caches parley `FontMetrics` per font family and style, used during
/// inline layout construction to size text runs accurately.
mod font_metrics;
/// HTML form submission: form data construction, encoding, and navigation.
///
/// Implements the [form submission algorithm](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#form-submission-algorithm)
/// including form data set construction, URL-encoded and multipart encoding,
/// and navigation to the action URL.
mod form;
/// HTML parsing trait: [`HtmlParserProvider`], [`DummyHtmlParserProvider`].
///
/// Allows pluggable parsing of `innerHTML` into the DOM tree.
/// The `DummyHtmlParserProvider` is a no-op placeholder; real parsing
/// comes from [bliss-html](https://docs.rs/bliss-html).
mod html;
/// Layout tree construction, computation, and damage tracking.
///
/// Sub-modules:
/// - `construct`: Builds the layout tree (inline layout, anonymous blocks, layout children)
/// - `damage`: Damage flags for incremental layout
/// - `inline`: Inline/block layout via parley
/// - `list`: List-item marker generation
/// - `replaced`: Replaced element sizing (images, canvas)
/// - `table`: Table layout integration
///
/// Uses [taffy](https://docs.rs/taffy) as the block/flex/grid layout engine.
mod layout;
/// High-level DOM mutation API: [`DocumentMutator`].
///
/// Provides safe, ergonomic methods for creating, inserting, moving, and removing nodes,
/// setting attributes, managing inline styles, and handling form control state.
/// Acquired via [`BaseDocument::mutate()`].
mod mutator;
/// CSS selector matching: `querySelector` / `querySelectorAll`.
///
/// Finds elements in the DOM tree matching a CSS selector string,
/// using Servo's `selectors` crate for parsing and matching.
mod query_selector;
/// Style resolution and layout computation pipeline.
///
/// Orchestrates: message processing → scroll animation → style resolution →
/// damage propagation → layout children construction → deferred tasks →
/// style-to-taffy flush → layout computation → damage clearing.
///
/// Entry point: [`BaseDocument::resolve()`].
mod resolve;
/// Script engine abstraction: [`ScriptEngine`], [`NoopScriptEngine`], [`BoxedScriptEngine`].
///
/// Provides a pluggable interface for running JavaScript or other scripting languages
/// in a sandboxed execution context. The `NoopScriptEngine` is the default placeholder.
mod script;
/// Text selection state for non-input elements.
///
/// Tracks anchor/focus endpoints across inline roots, including anonymous blocks
/// whose IDs may change across layout reconstructions. Used by event handling
/// to implement click-to-select and drag-to-extend.
mod selection;
/// Integration with Servo's style engine (stylo).
///
/// Bridges Servo's CSS selector matching, property declaration resolution,
/// and cascade logic into bliss-dom's node system. Provides the computed
/// style data used by layout.
mod stylo;
/// Conversion from Servo's `cursor` CSS property values to system cursor icons.
///
/// Maps CSS cursor keywords (pointer, text, grab, etc.) to
/// platform-specific cursor representations for [`bliss-shell`].
mod stylo_to_cursor_icon;
/// Conversion from Servo's computed styles to parley's text layout styles.
///
/// Translates CSS font properties (font-family, size, weight, style, letter-spacing,
/// line-height, text-decoration, etc.) into parley's `Style` and `StyleProperty` types
/// for text layout and shaping.
mod stylo_to_parley;
/// DOM tree traversal utilities: [`TreeTraverser`], [`AncestorTraverser`].
///
/// Provides iterators for pre-order DOM traversal, ancestor chain walking,
/// document-order comparison, inline-root collection across anonymous blocks,
/// and subtree mutation iteration.
mod traversal;

/// Engine tests
#[cfg(test)]
mod tests;
/// Document URL resolution and Servo `UrlExtraData` integration.
///
/// Wraps `url::Url` in a thread-safe `ServoArc` for use by Servo's style system.
/// Provides relative URL resolution for `<a href>`, `<img src>`, `@import`, etc.
mod url;

/// Networking and resource loading.
///
/// Types for HTTP requests, resource handling (CSS, images, fonts),
/// and the [`StylesheetHandler`] that loads external stylesheets.
/// Requires the `net` provider configured in [`DocumentConfig`].
pub mod net;
/// Shared utility types.
///
/// Exports [`Point`] and other geometry primitives used across the bliss crate family.
pub mod util;

#[cfg(feature = "accessibility")]
mod accessibility;

pub use config::DocumentConfig;
pub use document::{BaseDocument, DocGuard, DocGuardMut, Document, PlainDocument};
pub use dom_control::PolicyDomController;
pub use markup5ever::{
    LocalName, Namespace, NamespaceStaticSet, Prefix, PrefixStaticSet, QualName, local_name,
    namespace_prefix, namespace_url, ns,
};
pub use mutator::DocumentMutator;
pub use node::{Attribute, ElementData, Node, NodeData, TextNodeData};
pub use parley::FontContext;
pub use script::{
    BoxedScriptEngine, EventHandled, ExecutionContext, NoopScriptEngine, ScriptEngine, ScriptError,
    ScriptErrorCallback, ScriptLanguage, ScriptValue,
};
pub use style::Atom;
pub use style::invalidation::element::restyle_hints::RestyleHint;
pub type SelectorList = selectors::SelectorList<style::selector_parser::SelectorImpl>;
pub use events::{EventDriver, EventHandler, NoopEventHandler};
pub use html::{DummyHtmlParserProvider, HtmlParserProvider};
pub use util::Point;
