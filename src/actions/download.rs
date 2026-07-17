use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;

use crate::error::MeleyError;
use crate::observation::{ActionResult, DownloadInfo, Observation};
use crate::session::SessionManager;

/// Global download registry per session.
pub type DownloadRegistry = Arc<Mutex<HashMap<String, Vec<DownloadInfo>>>>;

/// Download a file by navigating to its URL.
pub async fn download_file(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    url: &str,
    save_as: Option<&str>,
    downloads: &DownloadRegistry,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = tokio::time::Duration::from_millis(timeout_ms.unwrap_or(60000));

    let result: Result<(String, DownloadInfo)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        // Determine save path
        let download_dir = session.profile.downloads_dir();
        let filename = save_as.map(|s| s.to_string()).unwrap_or_else(|| {
            url.split('/')
                .next_back()
                .unwrap_or("download")
                .split('?')
                .next()
                .unwrap_or("download")
                .to_string()
        });
        let save_path = download_dir.join(&filename);

        let download_id = uuid::Uuid::new_v4().to_string();
        let started_at = chrono::Utc::now().to_rfc3339();

        // Use reqwest-like fetch via JS
        let fetch_js = format!(
            r#"
            (async function() {{
                try {{
                    var resp = await fetch({});
                    var buf = await resp.arrayBuffer();
                    var bytes = new Uint8Array(buf);
                    var b64 = btoa(String.fromCharCode.apply(null, bytes));
                    return {{ ok: true, data: b64, size: bytes.length }};
                }} catch(e) {{
                    return {{ ok: false, error: e.toString() }};
                }}
            }})()
            "#,
            serde_json::json!(url)
        );

        let result_val = tokio::time::timeout(timeout, async {
            page.evaluate(fetch_js)
                .await
                .map_err(|e| anyhow::anyhow!(MeleyError::DownloadFailed(e.to_string())))
        })
        .await
        .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("download timed out".to_string())))??;

        let val = result_val
            .into_value::<serde_json::Value>()
            .map_err(|e| anyhow::anyhow!(MeleyError::DownloadFailed(e.to_string())))?;

        if !val["ok"].as_bool().unwrap_or(false) {
            return Err(anyhow::anyhow!(MeleyError::DownloadFailed(
                val["error"].as_str().unwrap_or("unknown error").to_string()
            )));
        }

        let b64 = val["data"].as_str().unwrap_or("");
        let size = val["size"].as_u64().unwrap_or(0);

        // Decode and write file
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| anyhow::anyhow!(MeleyError::DownloadFailed(e.to_string())))?;

        tokio::fs::create_dir_all(&download_dir)
            .await
            .map_err(|e| anyhow::anyhow!(MeleyError::DownloadFailed(e.to_string())))?;
        tokio::fs::write(&save_path, &bytes)
            .await
            .map_err(|e| anyhow::anyhow!(MeleyError::DownloadFailed(e.to_string())))?;

        let info = DownloadInfo {
            id: download_id.clone(),
            url: url.to_string(),
            path: save_path.to_string_lossy().to_string(),
            size_bytes: Some(size),
            state: "completed".to_string(),
            started_at: started_at.clone(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
        };

        // Register the download
        {
            let mut dl = downloads.lock().await;
            dl.entry(session_id.to_string())
                .or_insert_with(Vec::new)
                .push(info.clone());
        }

        Ok((actual_tab_id, info))
    }
    .await;

    match result {
        Ok((tid, info)) => Observation::success(
            session_id,
            tid,
            "download_file",
            ActionResult::Download(info),
        ),
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "download_file",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// List downloads for a session.
pub async fn list_downloads(
    session_id: &str,
    tab_id: &str,
    downloads: &DownloadRegistry,
) -> Observation {
    let dl = downloads.lock().await;
    let infos = dl.get(session_id).cloned().unwrap_or_default();
    Observation::success(
        session_id,
        tab_id,
        "list_downloads",
        ActionResult::Tabs(
            infos
                .iter()
                .map(|d| crate::observation::TabInfo {
                    tab_id: d.id.clone(),
                    url: Some(d.url.clone()),
                    title: Some(d.path.clone()),
                    is_active: false,
                    loading: d.state == "in_progress",
                })
                .collect(),
        ),
    )
}

fn error_code(e: &anyhow::Error) -> (&'static str, bool) {
    if let Some(me) = e.downcast_ref::<MeleyError>() {
        (me.error_code(), me.is_retryable())
    } else {
        ("INTERNAL_ERROR", false)
    }
}
