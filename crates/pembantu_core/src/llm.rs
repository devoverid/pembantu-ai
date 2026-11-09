use std::time::Duration;

use async_openai::{config::OpenAIConfig, types::chat::ChatCompletionTools, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::PembantuError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LlmProvider {
    OpenRouter,
    Gemini,
}

impl LlmProvider {
    fn api_base(self) -> &'static str {
        match self {
            LlmProvider::OpenRouter => "https://openrouter.ai/api/v1",
            LlmProvider::Gemini => "https://generativelanguage.googleapis.com/v1beta/openai",
        }
    }
}

/// Non-streaming chat-completions client for any OpenAI-compatible endpoint.
#[derive(Clone)]
pub struct LlmClient {
    client: Client<OpenAIConfig>,
    model: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct AssistantReply {
    /// The assistant message exactly as it must be echoed back in follow-up requests.
    /// Gemini 3+ (`extra_content.google.thought_signature`) and OpenRouter (`reasoning_details`)
    /// reject tool follow-ups that drop provider-specific fields, so this stays untyped.
    pub raw_message: Value,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Value],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    tools: &'a [ChatCompletionTools],
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Value,
}

impl LlmClient {
    pub fn new(provider: LlmProvider, api_key: String, model: String) -> Self {
        let config = OpenAIConfig::new()
            .with_api_base(provider.api_base())
            .with_api_key(api_key);
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build HTTP client");
        Self {
            client: Client::with_config(config).with_http_client(http),
            model,
        }
    }

    pub async fn complete(
        &self,
        messages: &[Value],
        tools: &[ChatCompletionTools],
    ) -> Result<AssistantReply, PembantuError> {
        let req = ChatRequest {
            model: &self.model,
            messages,
            tools,
            stream: false,
        };
        let resp: ChatResponse = self.client.chat().create_byot(&req).await?;
        let choice = resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| PembantuError::Generate("empty choices".into()))?;
        parse_reply(choice.message)
    }
}

fn parse_reply(message: Value) -> Result<AssistantReply, PembantuError> {
    let Value::Object(mut obj) = message else {
        return Err(PembantuError::Generate("assistant message is not an object".into()));
    };
    obj.retain(|_, v| !v.is_null());
    obj.insert("role".into(), Value::String("assistant".into()));

    let content = match obj.get("content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect(),
        _ => String::new(),
    };

    let mut tool_calls = Vec::new();
    if let Some(Value::Array(calls)) = obj.get_mut("tool_calls") {
        for (index, call) in calls.iter_mut().enumerate() {
            let id = match call.get("id").and_then(Value::as_str) {
                Some(id) if !id.is_empty() => id.to_owned(),
                _ => {
                    let id = format!("call_{index}");
                    if let Value::Object(call) = call {
                        call.insert("id".into(), Value::String(id.clone()));
                    }
                    id
                }
            };
            let function = call.get("function");
            let name = function
                .and_then(|f| f.get("name"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let arguments = match function.and_then(|f| f.get("arguments")) {
                Some(Value::String(s)) => s.clone(),
                None | Some(Value::Null) => String::new(),
                Some(other) => other.to_string(),
            };
            tool_calls.push(ToolCall { id, name, arguments });
        }
    }
    // OpenAI-compatible APIs reject `tool_calls: []` when the message is sent back.
    if tool_calls.is_empty() {
        obj.remove("tool_calls");
    }

    Ok(AssistantReply {
        raw_message: Value::Object(obj),
        content,
        tool_calls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keeps_provider_fields_and_fills_missing_tool_call_ids() {
        let reply = parse_reply(json!({
            "content": null,
            "reasoning_details": [{"type": "reasoning.encrypted", "data": "abc"}],
            "tool_calls": [
                {"type": "function", "function": {"name": "remember", "arguments": {"fact": "x"}},
                 "extra_content": {"google": {"thought_signature": "sig"}}},
                {"id": "c2", "type": "function", "function": {"name": "forget", "arguments": "{\"memory_id\":1}"}}
            ]
        }))
        .unwrap();

        assert_eq!(reply.content, "");
        assert_eq!(
            reply.tool_calls,
            vec![
                ToolCall { id: "call_0".into(), name: "remember".into(), arguments: r#"{"fact":"x"}"#.into() },
                ToolCall { id: "c2".into(), name: "forget".into(), arguments: r#"{"memory_id":1}"#.into() },
            ]
        );
        let raw = &reply.raw_message;
        assert_eq!(raw["role"], "assistant");
        assert!(raw.get("content").is_none());
        assert_eq!(raw["tool_calls"][0]["id"], "call_0");
        assert_eq!(raw["tool_calls"][0]["extra_content"]["google"]["thought_signature"], "sig");
        assert_eq!(raw["reasoning_details"][0]["data"], "abc");
    }

    #[test]
    fn joins_array_content_and_drops_empty_tool_calls() {
        let reply = parse_reply(json!({
            "role": "model",
            "content": [{"type": "text", "text": "Hello "}, {"type": "text", "text": "world"}],
            "tool_calls": []
        }))
        .unwrap();
        assert_eq!(reply.content, "Hello world");
        assert!(reply.tool_calls.is_empty());
        assert_eq!(reply.raw_message["role"], "assistant");
        assert!(reply.raw_message.get("tool_calls").is_none());
    }
}
