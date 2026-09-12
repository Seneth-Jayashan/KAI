use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCall {
    pub function: ToolCallFunction,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Serialize, Clone, Debug)]
pub struct Tool {
    pub r#type: String, // usually "function"
    pub function: ToolFunction,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

#[derive(Deserialize, Debug)]
pub struct ChatResponse {
    pub message: ChatMessage,
}

pub struct LlmClient {
    client: Client,
    base_url: String,
    pub model: String,
    pub tools: Option<Vec<Tool>>,
}

impl Default for LlmClient {
    fn default() -> Self {
        Self {
            client: Client::new(),
            base_url: "http://localhost:11434".to_string(),
            // For tools, Llama 3.1 is recommended
            model: "llama3.1".to_string(),
            tools: None,
        }
    }
}

impl LlmClient {
    pub fn new(model: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: "http://localhost:11434".to_string(),
            model: model.to_string(),
            tools: None,
        }
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub async fn chat(&self, app: &tauri::AppHandle, messages: Vec<ChatMessage>) -> Result<ChatMessage, String> {
        use tauri::Emitter;
        use futures_util::StreamExt;

        let req = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            tools: self.tools.clone(),
        };

        let response = self
            .client
            .post(&format!("{}/api/chat", self.base_url))
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Ollama API error {}: {}", status, text));
        }

        let mut full_content = String::new();
        let mut tool_calls = None;

        let mut stream = response.bytes_stream();
        while let Some(chunk_res) = stream.next().await {
            if let Ok(chunk) = chunk_res {
                let chunk_str = String::from_utf8_lossy(&chunk);
                for line in chunk_str.lines() {
                    if line.trim().is_empty() { continue; }
                    if let Ok(partial) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(msg) = partial.get("message") {
                            if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                                full_content.push_str(content);
                                let _ = app.emit("llm-token", content);
                            }
                            if let Some(tcs) = msg.get("tool_calls") {
                                if let Ok(parsed_tcs) = serde_json::from_value::<Vec<ToolCall>>(tcs.clone()) {
                                    tool_calls = Some(parsed_tcs);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ChatMessage {
            role: "assistant".to_string(),
            content: full_content,
            images: None,
            tool_calls,
        })
    }
}
