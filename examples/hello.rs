fn main() {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <style>
                body {
                    background-color: #050508;
                    color: #e0e0e0;
                    font-family: ui-sans-serif, system-ui, -apple-system, sans-serif;
                    display: flex;
                    flex-direction: column;
                    align-items: center;
                    justify-content: center;
                    height: 100vh;
                    margin: 0;
                }
                .container {
                    padding: 40px;
                    background: rgba(255, 255, 255, 0.03);
                    border: 1px solid rgba(255, 255, 255, 0.1);
                    border-radius: 24px;
                    backdrop-filter: blur(10px);
                    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
                    text-align: center;
                    max-width: 500px;
                }
                h1 {
                    font-size: 3rem;
                    margin: 0;
                    background: linear-gradient(135deg, #00d2ff 0%, #3a7bd5 100%);
                    -webkit-background-clip: text;
                    -webkit-text-fill-color: transparent;
                }
                p {
                    font-size: 1.25rem;
                    color: #888;
                    margin-top: 16px;
                }
                .tag {
                    display: inline-block;
                    padding: 4px 12px;
                    background: rgba(168, 85, 247, 0.2);
                    border: 1px solid rgba(168, 85, 247, 0.4);
                    border-radius: 100px;
                    color: #a855f7;
                    font-size: 0.875rem;
                    margin-top: 24px;
                    font-weight: 600;
                }
            </style>
        </head>
        <body>
            <div class="container">
                <h2>Hello World!</h2>
                <h1>Bliss Engine</h1>
                <p>Agent-Native Rendering for the Exosphere.</p>
                <div class="tag">Alpha v0.1.0</div>
            </div>
        </body>
        </html>
    "#;

    bliss::launch_static_html(html);
}
