#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use clap::Parser;
    use crate::{Cli, ManagementState};

    #[test]
    fn test_cli_parsing() {
        let args = vec!["rutify-panel", "--server", "http://localhost:8080"];
        let cli = Cli::try_parse_from(args).unwrap();
        
        assert_eq!(cli.server, "http://localhost:8080");
    }

    #[test]
    fn test_cli_default_server() {
        let args = vec!["rutify-panel"];
        let cli = Cli::try_parse_from(args).unwrap();
        
        assert_eq!(cli.server, "http://localhost:8080");
    }

    #[test]
    fn test_management_state_creation() {
        let state = ManagementState::new("http://localhost:3000");
        
        // Test that the state was created successfully
        assert_eq!(state.client.base_url, "http://localhost:3000");
        assert_eq!(state.notifications.lock().unwrap().len(), 0);
        assert!(state.stats.lock().unwrap().is_none());
        assert_eq!(state.tokens.lock().unwrap().len(), 0);
        assert_eq!(state.devices.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_management_state_with_different_server() {
        let state = ManagementState::new("http://example.com:8080");
        assert_eq!(state.client.base_url, "http://example.com:8080");
    }

    #[test]
    fn test_management_state_notifications_capacity() {
        let state = ManagementState::new("http://localhost:3000");
        assert_eq!(state.notifications.lock().unwrap().capacity(), 0);
        assert_eq!(state.tokens.lock().unwrap().capacity(), 0);
        assert_eq!(state.devices.lock().unwrap().capacity(), 0);
    }

    #[test]
    fn test_management_state_add_notification() {
        let state = ManagementState::new("http://localhost:3000");
        let mut guard = state.notifications.lock().unwrap();
        
        let item = rutify_sdk::NotifyItem {
            id: 1,
            title: "Test".to_string(),
            notify: "Message".to_string(),
            device: "Device".to_string(),
            received_at: chrono::Utc::now(),
        };
        
        guard.push(item);
        assert_eq!(guard.len(), 1);
        assert_eq!(guard[0].id, 1);
    }

    #[test]
    fn test_management_state_add_token() {
        let state = ManagementState::new("http://localhost:3000");
        let mut guard = state.tokens.lock().unwrap();
        
        let token = rutify_sdk::TokenItem {
            id: 1,
            token_hash: "abc123".to_string(),
            usage: "api".to_string(),
            created_at: chrono::Utc::now(),
        };
        
        guard.push(token);
        assert_eq!(guard.len(), 1);
        assert_eq!(guard[0].id, 1);
    }

    #[test]
    fn test_management_state_add_device() {
        let state = ManagementState::new("http://localhost:3000");
        let mut guard = state.devices.lock().unwrap();
        
        let device = rutify_sdk::DeviceInfo {
            id: Some(123),
            name: "Test Device".to_string(),
            last_seen: Some(chrono::Utc::now()),
            is_active: true,
        };
        
        guard.push(device);
        assert_eq!(guard.len(), 1);
        assert_eq!(guard[0].id, Some(123));
    }

    #[test]
    fn test_management_state_concurrent_access() {
        let state = ManagementState::new("http://localhost:3000");
        
        // Test concurrent access to different collections
        let notifications = Arc::clone(&state.notifications);
        let stats = Arc::clone(&state.stats);
        let tokens = Arc::clone(&state.tokens);
        let devices = Arc::clone(&state.devices);
        
        // These should not panic
        let _notifications = notifications.lock().unwrap();
        let _stats = stats.lock().unwrap();
        let _tokens = tokens.lock().unwrap();
        let _devices = devices.lock().unwrap();
    }

    #[test]
    fn test_management_state_client_methods() {
        let state = ManagementState::new("http://localhost:3000");
        
        // Test that the client was created successfully
        assert_eq!(state.client.base_url, "http://localhost:3000");
        
        // Test timeout configuration
        let client_with_timeout = state.client.with_timeout(std::time::Duration::from_secs(60));
        assert_eq!(client_with_timeout.timeout, std::time::Duration::from_secs(60));
    }

    #[test]
    fn test_management_state_arc_clone() {
        let state = ManagementState::new("http://localhost:3000");
        
        // Test that the state can be cloned
        let cloned_state = ManagementState {
            client: state.client.clone(),
            notifications: Arc::clone(&state.notifications),
            stats: Arc::clone(&state.stats),
            tokens: Arc::clone(&state.tokens),
            devices: Arc::clone(&state.devices),
        };
        
        assert_eq!(cloned_state.client.base_url, state.client.base_url);
        assert_eq!(cloned_state.notifications.lock().unwrap().len(), 0);
        assert!(cloned_state.stats.lock().unwrap().is_none());
    }
}
