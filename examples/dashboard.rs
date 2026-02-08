fn main() {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <title>Bliss Dashboard</title>
        <head>
            <style>
                body {
                    background: #020205;
                    color: #fff;
                    font-family: 'Inter', sans-serif;
                    margin: 0;
                    padding: 0;
                    display: flex;
                    height: 100vh;
                }
                .sidebar {
                    width: 240px;
                    background: #0a0a0f;
                    border-right: 1px solid #1a1a25;
                    padding: 24px;
                }
                .main {
                    flex: 1;
                    padding: 40px;
                    overflow-y: auto;
                }
                .nav-item {
                    padding: 12px;
                    margin-bottom: 8px;
                    border-radius: 8px;
                    color: #888;
                    cursor: pointer;
                    transition: all 0.2s;
                }
                .nav-item:hover {
                    background: #1a1a25;
                    color: #fff;
                }
                .nav-item.active {
                    background: #2a2a35;
                    color: #fff;
                    font-weight: 600;
                }
                .card-grid {
                    display: grid;
                    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
                    gap: 24px;
                    margin-top: 32px;
                }
                .card {
                    background: #0f0f18;
                    border: 1px solid #1a1a25;
                    border-radius: 16px;
                    padding: 24px;
                    transition: transform 0.2s, border-color 0.2s;
                }
                .card:hover {
                    transform: translateY(-4px);
                    border-color: #3b82f6;
                }
                .card-header {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 16px;
                }
                .status-badge {
                    padding: 4px 10px;
                    border-radius: 100px;
                    font-size: 11px;
                    text-transform: uppercase;
                    letter-spacing: 0.05em;
                }
                .status-active { background: rgba(34, 197, 94, 0.2); color: #4ade80; }
                .status-idle { background: rgba(234, 179, 8, 0.2); color: #facc15; }
                
                .h-title { font-size: 24px; font-weight: 700; margin: 0; }
                .h-subtitle { color: #666; font-size: 14px; margin-top: 4px; }
            </style>
        </head>
        <body>
            <div class="sidebar">
                <div style="font-weight: 800; font-size: 20px; color: #3b82f6; margin-bottom: 32px;">BLISS</div>
                <div class="nav-item active">Dashboard</div>
                <div class="nav-item">Apps</div>
                <div class="nav-item">Network</div>
                <div class="nav-item">Security</div>
                <div class="nav-item">Settings</div>
            </div>
            <div class="main">
                <h1 class="h-title">Active Apps</h1>
                <p class="h-subtitle">Monitoring 4 active agent instances</p>
                
                <div class="card-grid">
                    <div class="card">
                        <div class="card-header">
                            <div style="font-weight: 600;">Agent</div>
                            <span class="status-badge status-active">Active</span>
                        </div>
                        <div style="font-size: 13px; color: #888;">Monitoring edge security policies.</div>
                    </div>
                    <div class="card">
                        <div class="card-header">
                            <div style="font-weight: 600;">Cortex-N1</div>
                            <span class="status-badge status-active">Active</span>
                        </div>
                        <div style="font-size: 13px; color: #888;">Running inference for system optimization.</div>
                    </div>
                    <div class="card">
                        <div class="card-header">
                            <div style="font-weight: 600;">Bridge-Service</div>
                            <span class="status-badge status-idle">Idle</span>
                        </div>
                        <div style="font-size: 13px; color: #888;">Handling IPC between legacy modules.</div>
                    </div>
                </div>
            </div>
        </body>
        </html>
    "#;

    bliss::launch_static_html(html);
}
