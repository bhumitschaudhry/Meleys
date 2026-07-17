use meleys::actions::extraction::get_text;
use meleys::actions::navigation::navigate;
use meleys::config::Config;
use meleys::session::SessionManager;
use std::sync::Arc;

#[tokio::test]
#[ignore = "requires full Chrome/Chromium installation with shared libraries"]
async fn test_navigation_and_extraction() {
    let config = Config::default();
    let manager = Arc::new(SessionManager::new(config));

    // Create session
    let session = manager
        .create_session(None, Some(true), None, None)
        .await
        .expect("Failed to create session");
    let session_id = session.session_id.clone();

    // Navigate to a data URL containing simple HTML
    let url = "data:text/html,<html><head><title>Test Page</title></head><body><h1>Hello from Meleys</h1><p>Welcome to the agent-first browser runtime.</p><a href=\"https://github.com\">GitHub Link</a></body></html>";
    let obs = navigate(&manager, &session_id, None, url, None, None).await;
    assert!(obs.success, "Navigation failed: {:?}", obs.error);
    assert_eq!(obs.title.as_deref(), Some("Test Page"));

    // Extract text
    let text_obs = get_text(&manager, &session_id, None, None, None).await;
    assert!(text_obs.success, "Get text failed: {:?}", text_obs.error);
    if let meleys::observation::ActionResult::Text(text) = text_obs.result {
        assert!(text.contains("Hello from Meleys"));
        assert!(text.contains("Welcome to the agent-first browser runtime"));
    } else {
        panic!("Expected ActionResult::Text, got {:?}", text_obs.result);
    }

    // Clean up
    manager
        .close_session(&session_id)
        .await
        .expect("Failed to close session");
}
