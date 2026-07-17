use meleys::config::Config;
use meleys::session::SessionManager;

#[tokio::test]
#[ignore = "requires full Chrome/Chromium installation with shared libraries"]
async fn test_session_lifecycle() {
    let config = Config::default();
    let manager = SessionManager::new(config);

    // Create session
    let session = manager
        .create_session(None, Some(true), None, None)
        .await
        .expect("Failed to create session");

    let session_id = session.session_id.clone();
    assert!(!session_id.is_empty());

    // Verify session in manager
    let sessions = manager.list_sessions().await;
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, session_id);

    // Verify tabs
    let tabs = session.list_tabs().await.expect("Failed to list tabs");
    assert_eq!(tabs.len(), 1);
    let initial_tab_id = tabs[0].tab_id.clone();

    // Create new tab
    let new_tab_id = session
        .new_tab(None)
        .await
        .expect("Failed to create new tab");
    assert_eq!(session.tab_count().await, 2);

    // Switch tab
    session
        .switch_tab(&new_tab_id)
        .await
        .expect("Failed to switch tab");

    // Close tab
    session
        .close_tab(&initial_tab_id)
        .await
        .expect("Failed to close tab");
    assert_eq!(session.tab_count().await, 1);

    // Close session
    manager
        .close_session(&session_id)
        .await
        .expect("Failed to close session");

    assert_eq!(manager.list_sessions().await.len(), 0);
}
