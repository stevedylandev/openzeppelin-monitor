//! Telegram notification implementation.
//!
//! Provides functionality to send formatted messages to Telegram channels
//! via incoming webhooks, supporting message templates with variable substitution.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::{
	models::TriggerTypeConfig,
	services::notification::{format_message, NotificationError, Notifier, WebhookNotifier},
};

/// Implementation of Telegram notifications via webhooks
pub struct TelegramNotifier {
	inner: WebhookNotifier,
}

impl TelegramNotifier {
	/// Creates a new Telegram notifier instance
	///
	/// # Arguments
	/// * `token` - Telegram bot token
	/// * `chat_id` - Telegram chat ID
	/// * `disable_web_preview` - Disable web preview
	/// * `title` - Title to display in the message
	/// * `body_template` - Message template with variables
	pub fn new(
		base_url: Option<String>,
		token: String,
		chat_id: String,
		disable_web_preview: Option<bool>,
		title: String,
		body_template: String,
	) -> Result<Self, Box<NotificationError>> {
		let mut payload_fields = HashMap::new();
		payload_fields.insert("chat_id".to_string(), serde_json::json!(chat_id));
		payload_fields.insert("parse_mode".to_string(), serde_json::json!("markdown"));
		payload_fields.insert(
			"disable_web_page_preview".to_string(),
			serde_json::json!(disable_web_preview.unwrap_or(false)),
		);

		let url = format!(
			"{}/bot{}/sendMessage",
			base_url.unwrap_or("https://api.telegram.org".to_string()),
			token
		);

		let mut headers = HashMap::new();
		headers.insert("Content-Type".to_string(), "application/json".to_string());

		Ok(Self {
			inner: WebhookNotifier::new(
				url,
				title,
				body_template,
				Some("POST".to_string()),
				None,
				Some(headers),
				Some(payload_fields),
			)?,
		})
	}

	/// Formats a message by substituting variables in the template
	///
	/// # Arguments
	/// * `variables` - Map of variable names to values
	///
	/// # Returns
	/// * `String` - Formatted message with variables replaced
	pub fn format_message(&self, variables: &HashMap<String, String>) -> String {
		format_message::<fn(&str, &str) -> String>(
			&self.inner.title,
			&self.inner.body_template,
			variables,
			Some(|title, message| format!("*{}* \n\n{}", title, message)),
		)
	}

	/// Creates a Telegram notifier from a trigger configuration
	///
	/// # Arguments
	/// * `config` - Trigger configuration containing Telegram parameters
	///
	/// # Returns
	/// * `Option<Self>` - Notifier instance if config is Telegram type
	pub fn from_config(config: &TriggerTypeConfig) -> Option<Self> {
		match config {
			TriggerTypeConfig::Telegram {
				token,
				chat_id,
				message,
				disable_web_preview,
			} => {
				let mut payload_fields = HashMap::new();
				payload_fields.insert("chat_id".to_string(), serde_json::json!(chat_id));
				payload_fields.insert("parse_mode".to_string(), serde_json::json!("markdown"));
				payload_fields.insert(
					"disable_web_page_preview".to_string(),
					serde_json::json!(disable_web_preview.unwrap_or(false)),
				);

				let url = format!("{}/bot{}/sendMessage", "https://api.telegram.org", token);

				let mut headers = HashMap::new();
				headers.insert("Content-Type".to_string(), "application/json".to_string());

				Some(Self {
					inner: WebhookNotifier::new(
						url,
						message.title.clone(),
						message.body.clone(),
						Some("POST".to_string()),
						None,
						Some(headers),
						Some(payload_fields),
					)
					.ok()?,
				})
			}
			_ => None,
		}
	}
}

#[async_trait]
impl Notifier for TelegramNotifier {
	/// Sends a formatted message to Telegram
	///
	/// # Arguments
	/// * `message` - The formatted message to send
	///
	/// # Returns
	/// * `Result<(), anyhow::Error>` - Success or error
	async fn notify(&self, message: &str) -> Result<(), anyhow::Error> {
		let custom_payload = serde_json::json!({
			"text": message,
			"chat_id": self.inner.payload_fields.get("chat_id").unwrap(),
			"parse_mode": self.inner.payload_fields.get("parse_mode").unwrap(),
			"disable_web_page_preview": self.inner.payload_fields.get("disable_web_page_preview").unwrap()
		});
		self.inner.notify_with_payload(&custom_payload).await
	}
}

#[cfg(test)]
mod tests {
	use crate::models::NotificationMessage;

	use super::*;

	fn create_test_notifier(body_template: &str) -> TelegramNotifier {
		TelegramNotifier::new(
			None,
			"test-token".to_string(),
			"test-chat-id".to_string(),
			Some(true),
			"Alert".to_string(),
			body_template.to_string(),
		)
		.unwrap()
	}

	fn create_test_telegram_config(body_template: &str) -> TriggerTypeConfig {
		TriggerTypeConfig::Telegram {
			token: "test-token".to_string(),
			chat_id: "test-chat-id".to_string(),
			disable_web_preview: Some(true),
			message: NotificationMessage {
				title: "Alert".to_string(),
				body: body_template.to_string(),
			},
		}
	}

	////////////////////////////////////////////////////////////
	// format_message tests
	////////////////////////////////////////////////////////////

	#[test]
	fn test_format_message() {
		let notifier = create_test_notifier("Value is ${value} and status is ${status}");

		let mut variables = HashMap::new();
		variables.insert("value".to_string(), "100".to_string());
		variables.insert("status".to_string(), "critical".to_string());

		let result = notifier.format_message(&variables);
		assert_eq!(result, "*Alert* \n\nValue is 100 and status is critical");
	}

	#[test]
	fn test_format_message_with_missing_variables() {
		let notifier = create_test_notifier("Value is ${value} and status is ${status}");

		let mut variables = HashMap::new();
		variables.insert("value".to_string(), "100".to_string());
		// status variable is not provided

		let result = notifier.format_message(&variables);
		assert_eq!(result, "*Alert* \n\nValue is 100 and status is ${status}");
	}

	#[test]
	fn test_format_message_with_empty_template() {
		let notifier = create_test_notifier("");

		let variables = HashMap::new();
		let result = notifier.format_message(&variables);
		assert_eq!(result, "*Alert* \n\n");
	}

	////////////////////////////////////////////////////////////
	// construct_url tests
	////////////////////////////////////////////////////////////

	#[test]
	fn test_construct_url() {
		let notifier = create_test_notifier("Test message");
		let url = notifier.inner.url;
		assert_eq!(url, "https://api.telegram.org/bottest-token/sendMessage");
	}

	////////////////////////////////////////////////////////////
	// from_config tests
	////////////////////////////////////////////////////////////

	#[test]
	fn test_from_config_with_telegram_config() {
		let config = create_test_telegram_config("Test message");

		let notifier = TelegramNotifier::from_config(&config);
		assert!(notifier.is_some());

		let notifier = notifier.unwrap();
		assert_eq!(
			notifier.inner.url,
			"https://api.telegram.org/bottest-token/sendMessage"
		);
	}

	////////////////////////////////////////////////////////////
	// notify tests
	////////////////////////////////////////////////////////////

	#[tokio::test]
	async fn test_notify_failure() {
		let notifier = create_test_notifier("Test message");
		let result = notifier.notify("Test message").await;
		assert!(result.is_err());
	}
}
