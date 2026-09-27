#![cfg_attr(docsrs, feature(doc_cfg))]

//! Bliss is a modular, embeddable web engine with a native Rust API.
//!
//! It powers the [`dioxus-native`] UI framework.
//!
//! This crate exists to collect the most important functionality for users together in one place.
//! It does not bring any unique functionality, but rather, it re-exports the relevant crates as modules.
//! The exported crate corresponding to each module is also available in a stand-alone manner, i.e. [`bliss-dom`] as [`bliss::dom`](crate::dom).
//!
//! [`dioxus-native`]: https://docs.rs/dioxus-native
//! [`bliss-dom`]: https://docs.rs/bliss-dom

use std::sync::Arc;

use anyrender_vello::VelloWindowRenderer as WindowRenderer;
use bliss_dom::DocumentConfig;
use bliss_html::HtmlDocument;
use bliss_shell::{
    BlissApplication, BlissShellProxy, Config, EventLoop, WindowConfig, create_default_event_loop,
};
use bliss_traits::net::NetProvider;

#[doc(inline)]
/// Re-export of [`bliss_dom`].
pub use bliss_dom as dom;
#[doc(inline)]
/// Re-export of [`bliss_html`]. HTML parsing on top of bliss-dom
pub use bliss_html as html;
#[cfg(feature = "net")]
#[doc(inline)]
/// Re-export of [`bliss_net`].
pub use bliss_net as net;
#[doc(inline)]
/// Re-export of [`bliss_paint`].
pub use bliss_paint as paint;
#[doc(inline)]
/// Re-export of [`bliss_shell`].
pub use bliss_shell as shell;
#[doc(inline)]
/// Re-export of [`bliss_traits`](https://docs.rs/bliss-traits). Base types and traits for interoperability between modules
pub use bliss_traits as traits;

// ---------------------------------------------------------------------------
// Convenience re-exports used by cece-code and other downstream consumers
// ---------------------------------------------------------------------------

pub mod style;
pub mod element;

/// Keyboard key identifier.
pub use keyboard_types::Key;
/// Keyboard modifier flags.
pub use keyboard_types::Modifiers;

/// A colour value for use with [`style::Style`] and inline CSS.
///
/// Supports sRGB colours via the [`Rgb`] and [`Rgba`] variants.  When
/// serialised to CSS, opaque colours produce `#rrggbb` hex and colours with
/// alpha produce `rgba(r,g,b,a)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    /// RGB colour with all channels 0–255, alpha = 255.
    Rgb(u8, u8, u8),
    /// RGBA colour with all channels 0–255.
    Rgba(u8, u8, u8, u8),
}

impl Color {
    /// Create an opaque colour from its red, green and blue components (each 0–255).
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color::Rgb(r, g, b)
    }

    /// Create a colour with an alpha (transparency) channel from components (each 0–255).
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color::Rgba(r, g, b, a)
    }

    /// Serialise to a CSS colour string.
    pub fn to_css(&self) -> String {
        match self {
            Color::Rgb(r, g, b) | Color::Rgba(r, g, b, 255) => {
                format!("#{:02x}{:02x}{:02x}", r, g, b)
            }
            Color::Rgba(r, g, b, a) => {
                format!("rgba({},{},{},{:.3})", r, g, b, *a as f32 / 255.0)
            }
        }
    }
}

// Re-export the builder types at the crate root for ergonomic access.
pub use element::{Element, Window};

#[cfg(feature = "net")]
pub fn launch_url(url: &str) {
    let url = url.to_owned();
    let url = url::Url::parse(&url).expect("Invalid url");

    // Turn on the runtime and enter it
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();

    let event_loop = create_default_event_loop();
    let (proxy, receiver) = BlissShellProxy::new(event_loop.create_proxy());
    let net_provider = create_net_provider(proxy.clone());
    let application = BlissApplication::new(proxy, receiver);

    let (url, bytes) = rt
        .block_on(net_provider.fetch_async(bliss_traits::net::Request::get(url)))
        .unwrap();
    let html = std::str::from_utf8(bytes.as_ref()).unwrap();

    launch_internal(
        html,
        Config {
            stylesheets: Vec::new(),
            base_url: Some(url),
        },
        event_loop,
        application,
        net_provider,
    )
}

pub fn launch_static_html(html: &str) {
    launch_static_html_cfg(html, Config::default())
}

pub fn launch_static_html_cfg(html: &str, cfg: Config) {
    // Turn on the runtime and enter it
    #[cfg(feature = "net")]
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    #[cfg(feature = "net")]
    let _guard = rt.enter();

    let event_loop = create_default_event_loop();
    let (proxy, receiver) = BlissShellProxy::new(event_loop.create_proxy());
    let net_provider = create_net_provider(proxy.clone());
    let application = BlissApplication::new(proxy, receiver);

    launch_internal(html, cfg, event_loop, application, net_provider)
}

fn launch_internal(
    html: &str,
    cfg: Config,
    event_loop: EventLoop,
    mut application: BlissApplication<WindowRenderer>,
    net_provider: Arc<dyn NetProvider>,
) {
    let doc = HtmlDocument::from_html(
        html,
        DocumentConfig {
            base_url: cfg.base_url,
            ua_stylesheets: Some(cfg.stylesheets),
            net_provider: Some(net_provider),
            ..Default::default()
        },
    );
    let renderer = WindowRenderer::new();
    let window = WindowConfig::new(Box::new(doc) as _, renderer);

    // Create application

    application.add_window(window);

    // Run event loop
    event_loop.run_app(application).unwrap()
}

#[cfg(feature = "net")]
type EnabledNetProvider = bliss_net::Provider;
#[cfg(not(feature = "net"))]
type EnabledNetProvider = bliss_traits::net::DummyNetProvider;

fn create_net_provider(proxy: BlissShellProxy) -> Arc<EnabledNetProvider> {
    #[cfg(feature = "net")]
    let net_provider = Arc::new(bliss_net::Provider::new(Some(Arc::new(proxy))));
    #[cfg(not(feature = "net"))]
    let net_provider = {
        let _ = proxy;
        Arc::new(bliss_traits::net::DummyNetProvider)
    };

    net_provider
}
