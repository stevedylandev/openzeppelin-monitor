use openzeppelin_monitor::services::notification::{Notifier, TelegramNotifier};
use std::collections::HashMap;

#[tokio::test]
async fn test_telegram_notification_success() {
	// Setup async mock server
	let mut server = mockito::Server::new_async().await;
	// Mock the Telegram API endpoint
	let mock = server
		.mock("POST", "/bottest_token/sendMessage")
		.match_header("Content-Type", "application/json")
		.match_body(mockito::Matcher::Json(serde_json::json!({
			"chat_id": "test_chat_id",
			"disable_web_page_preview": false,
			"parse_mode": "markdown",
			"text": "*Test Alert* \n\nTest message with value 42"
		})))
		.with_status(200)
		.with_body(r#"{"ok": true, "result": {}}"#)
		.create_async()
		.await;

	let notifier = TelegramNotifier::new(
		Some(server.url()),
		"test_token".to_string(),
		"test_chat_id".to_string(),
		None,
		"Test Alert".to_string(),
		"Test message with value ${value}".to_string(),
	)
	.unwrap();

	// Prepare and send test message
	let mut variables = HashMap::new();
	variables.insert("value".to_string(), "42".to_string());
	let message = notifier.format_message(&variables);

	let result = notifier.notify(&message, Some(true)).await;

	assert!(result.is_ok());
	mock.assert();
}

#[tokio::test]
async fn test_telegram_notification_failure() {
	// Setup async mock server to simulate failure
	let mut server = mockito::Server::new_async().await;
	let mock = server
		.mock("POST", "/bottest_token/sendMessage")
		.match_header("Content-Type", "application/json")
		.match_body(mockito::Matcher::Json(serde_json::json!({
			"chat_id": "test_chat_id",
			"disable_web_page_preview": false,
			"parse_mode": "markdown",
			"text": "*Test Alert* \n\nTest message with value 42"
		})))
		.with_status(500)
		.with_body("Internal Server Error")
		.create_async()
		.await;

	let notifier = TelegramNotifier::new(
		Some(server.url()),
		"test_token".to_string(),
		"test_chat_id".to_string(),
		None,
		"Test Alert".to_string(),
		"Test message with value ${value}".to_string(),
	)
	.unwrap();

	// Prepare and send test message
	let mut variables = HashMap::new();
	variables.insert("value".to_string(), "42".to_string());
	let message = notifier.format_message(&variables);

	let result = notifier.notify(&message, Some(true)).await;

	assert!(result.is_err());
	mock.assert();
}
