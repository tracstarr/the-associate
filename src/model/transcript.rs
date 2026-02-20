use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

/// A single line from a .jsonl transcript file.
#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptEnvelope {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub message: Option<TranscriptMessage>,
    /// Catch-all for fields we don't model explicitly.
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranscriptMessage {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: MessageContent,
}

/// Content can be a plain string or an array of content blocks.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl Default for MessageContent {
    fn default() -> Self {
        MessageContent::Text(String::new())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        input: Option<Value>,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        #[serde(default)]
        content: Option<Value>,
    },
    #[serde(other)]
    Other,
}

/// Processed transcript item for display.
#[derive(Debug, Clone)]
pub struct TranscriptItem {
    pub timestamp: Option<DateTime<Utc>>,
    pub kind: TranscriptItemKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TranscriptItemKind {
    User,
    Assistant,
    ToolUse,
    ToolResult,
    System,
    Progress,
    Other,
}

impl TranscriptItemKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::User => "USER",
            Self::Assistant => "ASST",
            Self::ToolUse => "TOOL",
            Self::ToolResult => "RSLT",
            Self::System => "SYS ",
            Self::Progress => "PROG",
            Self::Other => "    ",
        }
    }
}

/// Parse a JSONL line into zero or more TranscriptItems.
pub fn parse_envelope(envelope: &TranscriptEnvelope) -> Vec<TranscriptItem> {
    let ts = envelope.timestamp;

    match envelope.kind.as_str() {
        "user" => parse_message_items(envelope, ts, TranscriptItemKind::User),
        "assistant" => parse_message_items(envelope, ts, TranscriptItemKind::Assistant),
        "system" => {
            let text = extract_message_text(envelope);
            if text.is_empty() {
                return vec![];
            }
            vec![TranscriptItem {
                timestamp: ts,
                kind: TranscriptItemKind::System,
                text,
            }]
        }
        "progress" => {
            let text = envelope
                .extra
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                return vec![];
            }
            vec![TranscriptItem {
                timestamp: ts,
                kind: TranscriptItemKind::Progress,
                text,
            }]
        }
        _ => vec![],
    }
}

fn parse_message_items(
    envelope: &TranscriptEnvelope,
    ts: Option<DateTime<Utc>>,
    default_kind: TranscriptItemKind,
) -> Vec<TranscriptItem> {
    let msg = match &envelope.message {
        Some(m) => m,
        None => return vec![],
    };

    match &msg.content {
        MessageContent::Text(s) => {
            if s.is_empty() {
                return vec![];
            }
            vec![TranscriptItem {
                timestamp: ts,
                kind: default_kind,
                text: s.clone(),
            }]
        }
        MessageContent::Blocks(blocks) => {
            let mut items = Vec::new();
            for block in blocks {
                match block {
                    ContentBlock::Text { text } => {
                        if !text.is_empty() {
                            items.push(TranscriptItem {
                                timestamp: ts,
                                kind: default_kind.clone(),
                                text: text.clone(),
                            });
                        }
                    }
                    ContentBlock::ToolUse { name, input } => {
                        let tool_name = name.as_deref().unwrap_or("unknown");
                        let summary = match input {
                            Some(Value::Object(map)) => {
                                // Show first string field as context
                                map.iter()
                                    .find_map(|(k, v)| {
                                        v.as_str().map(|s| {
                                            let truncated: String = s.chars().take(50).collect();
                                            format!("{}: {}", k, truncated)
                                        })
                                    })
                                    .unwrap_or_default()
                            }
                            _ => String::new(),
                        };
                        let text = if summary.is_empty() {
                            tool_name.to_string()
                        } else {
                            format!("{} ({})", tool_name, summary)
                        };
                        items.push(TranscriptItem {
                            timestamp: ts,
                            kind: TranscriptItemKind::ToolUse,
                            text,
                        });
                    }
                    ContentBlock::ToolResult { content } => {
                        let text = match content {
                            Some(Value::String(s)) => {
                                let truncated: String = s.chars().take(80).collect();
                                truncated
                            }
                            Some(Value::Array(arr)) => {
                                // Array of content blocks in tool results
                                arr.iter()
                                    .filter_map(|v| {
                                        v.get("text")
                                            .and_then(|t| t.as_str())
                                            .map(|s| s.chars().take(80).collect::<String>())
                                    })
                                    .next()
                                    .unwrap_or_else(|| "[result]".to_string())
                            }
                            _ => "[result]".to_string(),
                        };
                        items.push(TranscriptItem {
                            timestamp: ts,
                            kind: TranscriptItemKind::ToolResult,
                            text,
                        });
                    }
                    ContentBlock::Other => {}
                }
            }
            items
        }
    }
}

fn extract_message_text(envelope: &TranscriptEnvelope) -> String {
    if let Some(ref msg) = envelope.message {
        match &msg.content {
            MessageContent::Text(s) => return s.clone(),
            MessageContent::Blocks(blocks) => {
                for block in blocks {
                    if let ContentBlock::Text { text } = block {
                        return text.clone();
                    }
                }
            }
        }
    }
    String::new()
}
