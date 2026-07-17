use std::sync::Arc;

use anyhow::Result;
use base64::Engine;
use chromiumoxide::cdp::browser_protocol::page::{CaptureScreenshotFormat, PrintToPdfParams};
use chromiumoxide::page::ScreenshotParams;

use crate::error::MeleyError;
use crate::observation::{ActionResult, Observation};
use crate::selector::Selector;
use crate::session::SessionManager;

/// Take a screenshot.
pub async fn screenshot(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    selector: Option<&Selector>,
    full_page: bool,
    format: Option<&str>,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = tokio::time::Duration::from_millis(timeout_ms.unwrap_or(30000));

    let result: Result<(String, Option<String>, Option<String>, ActionResult)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        let action_result = tokio::time::timeout(timeout, async {
            let fmt = format.unwrap_or("png");
            let cdp_format = if fmt == "jpeg" || fmt == "jpg" {
                CaptureScreenshotFormat::Jpeg
            } else {
                CaptureScreenshotFormat::Png
            };

            // Get viewport/full-page dimensions
            let (width, height) = if full_page {
                let dims = page.evaluate(
                    "({width: document.documentElement.scrollWidth, height: document.documentElement.scrollHeight})"
                ).await
                .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?
                .into_value::<serde_json::Value>()
                .unwrap_or(serde_json::json!({"width": 1280, "height": 800}));
                (
                    dims["width"].as_u64().unwrap_or(1280) as u32,
                    dims["height"].as_u64().unwrap_or(800) as u32,
                )
            } else {
                let dims = page.evaluate(
                    "({width: window.innerWidth, height: window.innerHeight})"
                ).await
                .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?
                .into_value::<serde_json::Value>()
                .unwrap_or(serde_json::json!({"width": 1280, "height": 800}));
                (
                    dims["width"].as_u64().unwrap_or(1280) as u32,
                    dims["height"].as_u64().unwrap_or(800) as u32,
                )
            };

            let mut params_builder = ScreenshotParams::builder()
                .format(cdp_format)
                .full_page(full_page);

            // If selector is provided, get element clip
            if let Some(Selector::Css(css)) = selector {
                let clip_js = format!(
                    r#"(function() {{
                        var el = document.querySelector({});
                        if (!el) return null;
                        var r = el.getBoundingClientRect();
                        return {{x: r.x, y: r.y, width: r.width, height: r.height}};
                    }})()"#,
                    serde_json::json!(css)
                );
                let val = page.evaluate(clip_js).await
                    .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?
                    .into_value::<serde_json::Value>()
                    .unwrap_or(serde_json::Value::Null);
                if !val.is_null() {
                    params_builder = params_builder.clip(
                        chromiumoxide::cdp::browser_protocol::page::Viewport {
                            x: val["x"].as_f64().unwrap_or(0.0),
                            y: val["y"].as_f64().unwrap_or(0.0),
                            width: val["width"].as_f64().unwrap_or(1280.0),
                            height: val["height"].as_f64().unwrap_or(800.0),
                            scale: 1.0,
                        }
                    );
                }
            }

            let screenshot_data = page.screenshot(params_builder.build()).await
                .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;

            let b64 = base64::engine::general_purpose::STANDARD.encode(&screenshot_data);

            Ok::<ActionResult, anyhow::Error>(ActionResult::Screenshot {
                format: fmt.to_string(),
                base64: b64,
                width,
                height,
            })
        }).await
        .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("screenshot timed out".to_string())))??;

        let url = page.url().await.ok().flatten();
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title, action_result))
    }.await;

    match result {
        Ok((tid, url, title, action_result)) => {
            let mut obs = Observation::success(session_id, tid, "screenshot", action_result);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "screenshot",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// Export page as PDF.
pub async fn export_pdf(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    landscape: bool,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = tokio::time::Duration::from_millis(timeout_ms.unwrap_or(30000));

    let result: Result<(String, Option<String>, Option<String>, ActionResult)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        let pdf_bytes = tokio::time::timeout(timeout, async {
            page.pdf(PrintToPdfParams::builder().landscape(landscape).build())
                .await
                .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))
        })
        .await
        .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("export_pdf timed out".to_string())))??;

        // Save to profile downloads dir
        let filename = format!("page-{}.pdf", chrono::Utc::now().format("%Y%m%d%H%M%S"));
        let save_path = session.profile.downloads_dir().join(&filename);
        tokio::fs::write(&save_path, &pdf_bytes)
            .await
            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;

        let download_info = crate::observation::DownloadInfo {
            id: uuid::Uuid::new_v4().to_string(),
            url: "about:pdf".to_string(),
            path: save_path.to_string_lossy().to_string(),
            size_bytes: Some(pdf_bytes.len() as u64),
            state: "completed".to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
        };

        let url = page.url().await.ok().flatten();
        let title = page.get_title().await.ok().flatten();
        Ok((
            actual_tab_id,
            url,
            title,
            ActionResult::Download(download_info),
        ))
    }
    .await;

    match result {
        Ok((tid, url, title, action_result)) => {
            let mut obs = Observation::success(session_id, tid, "export_pdf", action_result);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "export_pdf",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

fn error_code(e: &anyhow::Error) -> (&'static str, bool) {
    if let Some(me) = e.downcast_ref::<MeleyError>() {
        (me.error_code(), me.is_retryable())
    } else {
        ("INTERNAL_ERROR", false)
    }
}
