use boa_engine::{Context, Source};

fn main() {
    // 1. Setup Boa JS Engine
    let mut context = Context::default();

    // 2. Define a "React-like" render function in JS
    let js_code = r#"
        const state = { 
            user: "Agent-001",
            apps: ["Sentry", "Cortex", "XIP-Bridge"],
            status: "Stellar"
        };
            
        function render() {
            return `
                <div style="padding: 40px; background: #0c0c14; border: 1px solid #27272a; border-radius: 24px; color: white; width: 400px; text-align: center;">
                    <h2 style="margin: 0; color: #a855f7;">Polyglot React</h2>
                    <p style="color: #71717a; margin-bottom: 24px;">Rendered via Boa Engine</p>
                    
                    <div style="background: rgba(168, 85, 247, 0.1); padding: 16px; border-radius: 12px; margin-bottom: 16px;">
                        <div style="font-size: 12px; color: #a855f7; text-transform: uppercase;">Active Session</div>
                        <div style="font-size: 20px; font-weight: 600;">${state.user}</div>
                    </div>

                    <div style="text-align: left; background: #050508; padding: 16px; border-radius: 12px;">
                        <div style="font-size: 11px; color: #52525b; margin-bottom: 8px;">ACTIVE CAPSULES</div>
                        ${state.apps.map(c => `
                            <div style="display: flex; align-items: center; margin-bottom: 4px;">
                                <div style="width: 6px; height: 6px; background: #22c55e; border-radius: 50%; margin-right: 8px;"></div>
                                <span style="font-size: 13px;">${c}</span>
                            </div>
                        `).join('')}
                    </div>
                    
                    <div style="margin-top: 24px; font-size: 12px; color: #52525b;">
                        System Status: <span style="color: #22c55e;">${state.status}</span>
                    </div>
                </div>
            `;
        }

        render();
    "#;

    // 3. Execute JS and get the HTML string
    let result = context.eval(Source::from_bytes(js_code.as_bytes())).expect("JS Execution failed");
    let component_html = result.as_string().unwrap().to_std_string_escaped();

    // 4. Wrap in a full HTML document
    let full_html = format!(
        r#"<!DOCTYPE html>
        <html>
        <title>Bliss React</title>
        <head>
            <style>
                body {{ 
                    background: #050508; 
                    display: flex; 
                    align-items: center; 
                    justify-content: center; 
                    height: 100vh; 
                    margin: 0; 
                    font-family: ui-sans-serif, system-ui, -apple-system, sans-serif;
                }}
            </style>
        </head>
        <body>
            {}
        </body>
        </html>"#,
        component_html
    );

    // 5. Launch Bliss
    println!("Bliss: Launching JS-rendered component...");
    bliss::launch_static_html(&full_html);
}
