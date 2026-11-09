//! Web search and page extraction through the Tavily API.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{error::PembantuError, truncate_chars};

#[derive(Clone)]
pub struct TavilyClient {
    api_key: String,
    http: reqwest::Client,
}

impl TavilyClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    async fn post<B: Serialize, R: for<'de> Deserialize<'de>>(&self, path: &str, body: &B) -> Result<R, PembantuError> {
        let response = self
            .http
            .post(format!("https://api.tavily.com/{path}"))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(PembantuError::Generate(format!(
                "Tavily {path} failed with {status}: {}",
                truncate_chars(&text, 500)
            )));
        }
        Ok(serde_json::from_str(&text)?)
    }
}

pub mod web_search {
    use async_openai::types::chat::ChatCompletionTools;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::TavilyClient;
    use crate::{
        error::PembantuError,
        tools::{function, parse_args},
        truncate_chars,
    };

    pub const NAME: &str = "web_search";

    pub fn definition() -> ChatCompletionTools {
        function(
            NAME,
            "Search the web for current information. Returns a short answer and the top results with URLs; cite the URLs you use.",
            json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "topic": {"type": "string", "enum": ["general", "news"], "description": "Use news for recent events."}
                },
                "required": ["query"]
            }),
        )
    }

    #[derive(Deserialize)]
    struct Args {
        query: String,
        topic: Option<String>,
    }

    #[derive(Serialize)]
    struct Request<'a> {
        query: &'a str,
        topic: &'a str,
        max_results: u8,
        include_answer: &'a str,
    }

    #[derive(Deserialize)]
    struct Response {
        answer: Option<String>,
        #[serde(default)]
        results: Vec<SearchResult>,
    }

    #[derive(Deserialize)]
    struct SearchResult {
        title: String,
        url: String,
        #[serde(default)]
        content: String,
    }

    pub async fn run(args: &str, client: &TavilyClient) -> Result<String, PembantuError> {
        let Args { query, topic } = parse_args(args)?;
        let topic = match topic.as_deref() {
            Some("news") => "news",
            _ => "general",
        };
        let request = Request { query: &query, topic, max_results: 5, include_answer: "basic" };
        let response: Response = client.post("search", &request).await?;
        let results: Vec<_> = response
            .results
            .iter()
            .map(|r| json!({"title": r.title, "url": r.url, "content": truncate_chars(&r.content, 1000)}))
            .collect();
        Ok(json!({"answer": response.answer, "results": results}).to_string())
    }
}

pub mod read_webpage {
    use async_openai::types::chat::ChatCompletionTools;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};

    use super::TavilyClient;
    use crate::{
        error::PembantuError,
        tools::{function, parse_args},
    };

    pub const NAME: &str = "read_webpage";

    pub fn definition() -> ChatCompletionTools {
        function(
            NAME,
            "Fetch a web page and return its main content as Markdown.",
            json!({
                "type": "object",
                "properties": {"url": {"type": "string"}},
                "required": ["url"]
            }),
        )
    }

    #[derive(Deserialize)]
    struct Args {
        url: String,
    }

    #[derive(Serialize)]
    struct Request<'a> {
        urls: [&'a str; 1],
        format: &'a str,
    }

    #[derive(Deserialize)]
    struct Response {
        #[serde(default)]
        results: Vec<Extracted>,
        #[serde(default)]
        failed_results: Vec<Value>,
    }

    #[derive(Deserialize)]
    struct Extracted {
        #[serde(default)]
        raw_content: String,
    }

    pub async fn run(args: &str, client: &TavilyClient) -> Result<String, PembantuError> {
        let Args { url } = parse_args(args)?;
        let response: Response = client.post("extract", &Request { urls: [&url], format: "markdown" }).await?;
        match response.results.into_iter().next() {
            Some(page) => Ok(page.raw_content),
            None => {
                let reason = response
                    .failed_results
                    .first()
                    .and_then(|f| f.get("error"))
                    .and_then(Value::as_str)
                    .unwrap_or("no content returned");
                Err(PembantuError::Generate(format!("could not read {url}: {reason}")))
            }
        }
    }
}
