use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;
use tokio::signal;

use meleys::actions::download::DownloadRegistry;
use meleys::actions::search::SearchRegistry;
use meleys::config::Config;
use meleys::session::SessionManager;
use meleys::transport::http::{AppState, build_router};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("meleys=info".parse()?)
        )
        .init();

    tracing::info!("Meleys browser runtime starting...");

    // Load configuration
    let config = Config::load().unwrap_or_default();
    tracing::info!("Configuration loaded: HTTP {}:{}", config.server.http_bind, config.server.http_port);

    // Create shared resources
    let session_manager = Arc::new(SessionManager::new(config.clone()));
    let search_registry = Arc::new(SearchRegistry::new(&config.search.default_engine));
    let downloads: DownloadRegistry = Arc::new(Mutex::new(Default::default()));
    let allow_js = config.limits.allow_evaluate_js;

    // Parse top-level command. Supported invocations:
    //   meleys                -> HTTP mode (default)
    //   meleys --mcp          -> MCP stdio mode
    //   meleys setup ...      -> agent config registration (installer hook)
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|a| a.as_str()) == Some("setup") {
        return meleys::setup::run(&args[2..]);
    }
    if args.iter().any(|a| a == "--mcp") {
        tracing::info!("Starting MCP stdio transport");
        let session_manager_clone = session_manager.clone();
        let search_registry_clone = search_registry.clone();
        let downloads_clone = downloads.clone();

        // Handle shutdown
        let shutdown = tokio::spawn(async move {
            signal::ctrl_c().await.ok();
            tracing::info!("Shutting down...");
        });

        let mcp_task = tokio::spawn(async move {
            if let Err(e) = meleys::transport::mcp::run_stdio_server(
                session_manager_clone,
                search_registry_clone,
                downloads_clone,
                allow_js,
            ).await {
                tracing::error!("MCP server error: {}", e);
            }
        });

        tokio::select! {
            _ = shutdown => {}
            _ = mcp_task => {}
        }
    } else {
        // HTTP mode (default)
        let state = AppState {
            session_manager: session_manager.clone(),
            search_registry,
            downloads,
            allow_js,
        };

        let router = build_router(state);
        let addr_str = format!("{}:{}", config.server.http_bind, config.server.http_port);
        let addr: SocketAddr = addr_str.parse()
            .map_err(|e| anyhow::anyhow!("Invalid bind address {}: {}", addr_str, e))?;

        tracing::info!("Starting HTTP server on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;

        // Graceful shutdown
        let session_manager_shutdown = session_manager.clone();
        let shutdown_fut = async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm = signal(SignalKind::terminate()).unwrap();
                let mut sigint = signal(SignalKind::interrupt()).unwrap();
                tokio::select! {
                    _ = sigterm.recv() => tracing::info!("Received SIGTERM"),
                    _ = sigint.recv() => tracing::info!("Received SIGINT"),
                }
            }
            #[cfg(not(unix))]
            {
                signal::ctrl_c().await.ok();
            }
            tracing::info!("Initiating graceful shutdown...");
            session_manager_shutdown.close_all().await;
            tracing::info!("All sessions closed.");
        };

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_fut)
            .await?;
    }

    Ok(())
}
