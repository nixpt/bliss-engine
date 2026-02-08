#![cfg_attr(docsrs, feature(doc_cfg))]

//! Bliss is a modular, embeddable web engine with a native Rust API.
//!
//! It powers the Exosphere Browser Engine.
//!
//! This crate exists to collect the most important functionality for users together in one place.
//! It does not bring any unique functionality, but rather, it re-exports the relevant crates as modules.
//! The exported crate corresponding to each module is also available in a stand-alone manner, i.e. [`bliss-dom`] as [`bliss::dom`](crate::dom).
//!
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

#[cfg(feature = "net")]
pub fn launch_url(url: &str) {
    // Assert that url is valid
    println!("{url}");
    let url = url.to_owned();
    let url = url::Url::parse(&url).expect("Invalid url");

    // Turn on the runtime and enter it
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();

    let event_loop = create_default_event_loop();
    let (proxy, reciever) = BlissShellProxy::new(event_loop.create_proxy());
    let net_provider = create_net_provider(proxy.clone());
    let application = BlissApplication::new(proxy, reciever);

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
    let (proxy, reciever) = BlissShellProxy::new(event_loop.create_proxy());
    let net_provider = create_net_provider(proxy.clone());
    let application = BlissApplication::new(proxy, reciever);

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
        use bliss_traits::net::DummyNetProvider;

        // This isn't used without the net feature, so ignore it here to not
        // get unnused warnings.
        let _ = event_loop;
        Arc::new(DummyNetProvider::default())
    };

    net_provider
}
