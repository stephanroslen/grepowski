use serde::Serialize;
use serde_json::Value;
use std::fmt::Debug;

pub trait AiQueryConfig: Debug + Send {
    fn system_prompt(&self) -> String;
    fn response_format(&self) -> Value;
    fn max_tokens(&self) -> usize;
    fn extract_result(&self, content: &str) -> anyhow::Result<f32>;
}

impl<T: AiQueryConfig + 'static> From<T> for Box<dyn AiQueryConfig> {
    fn from(t: T) -> Self {
        Box::new(t)
    }
}

#[derive(Clone, Debug)]
pub struct DefaultAiQueryConfig;

impl AiQueryConfig for DefaultAiQueryConfig {
    fn system_prompt(&self) -> String {
        "You are an evaluation model. For the output use the provided schema. Make the score a floating point number in the range 0 to 1 with up to three decimal places. The number must measure how strongly the question stated in the system prompt applies to the code fragment provided in the user prompt. The code is cut arbitrarily from the source file. Use the scale as follows: 0.000 → the statement is entirely false for the code. 0.250 → weak indication. 0.500 → partially true / ambiguous. 0.750 → strongly supported. 1.000 → fully and unambiguously true. Do not default to the given values, but spread your output value across the full range from 0 to 1 interpolating between the values according to your assessment.".to_string()
    }

    fn response_format(&self) -> Value {
        serde_json::json!({"type": "json_schema",
        "json_schema": {
            "strict": true,
            "name": "score",
            "schema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "reason": { "type": "string" },
                    "score": { "type": "number" }
                },
                "required": ["reason", "score"]
            }
        }})
    }

    fn max_tokens(&self) -> usize {
        10000
    }

    fn extract_result(&self, content: &str) -> anyhow::Result<f32> {
        let content: Value = serde_json::from_str(content)
            .map_err(|e| anyhow::anyhow!("error parsing {}: {}", content, e))?;
        let result = content["score"]
            .as_f64()
            .ok_or(anyhow::anyhow!("Score not found in response {}", content))?
            as f32;

        Ok(result)
    }
}

#[derive(Serialize, Clone, Debug)]
struct ChatRequestMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Clone, Debug)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatRequestMessage>,
    temperature: Option<f32>,
    max_completion_tokens: usize,
    stream: bool,
    response_format: Value,
}

#[derive(Debug)]
struct ChatRequestFactory {
    model: String,
    temperature: Option<f32>,
    ai_query_config: Box<dyn AiQueryConfig>,
    question: String,
}

impl ChatRequestFactory {
    fn new(
        model: String,
        temperature: Option<f32>,
        ai_query_config: impl Into<Box<dyn AiQueryConfig>>,
        question: String,
    ) -> Self {
        let ai_query_config = ai_query_config.into();
        Self {
            model,
            temperature,
            ai_query_config,
            question,
        }
    }

    fn create_system_message(&self) -> ChatRequestMessage {
        ChatRequestMessage {
            role: "system".to_string(),
            content: format!(
                "{} Question: {}",
                self.ai_query_config.system_prompt(),
                self.question
            ),
        }
    }

    fn create_user_message(&self, content: String) -> ChatRequestMessage {
        ChatRequestMessage {
            role: "user".to_string(),
            content,
        }
    }

    fn create(&self, code: impl Into<String>) -> ChatRequest {
        let messages = vec![
            self.create_system_message(),
            self.create_user_message(code.into()),
        ];
        let response_format = self.ai_query_config.response_format();
        let max_completion_tokens = self.ai_query_config.max_tokens();
        ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: self.temperature,
            max_completion_tokens,
            stream: false,
            response_format,
        }
    }

    fn create_json(&self, code: impl Into<String>) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&self.create(code))?)
    }
}

pub struct AI {
    chat_request_factory: ChatRequestFactory,
    client: reqwest::Client,
    url: String,
    auth_token: Option<String>,
}

impl AI {
    pub fn new(
        model: impl Into<String>,
        url: impl Into<String>,
        auth_token: Option<String>,
        temperature: Option<f32>,
        ai_query_config: impl Into<Box<dyn AiQueryConfig>>,
        question: impl Into<String>,
    ) -> Self {
        let chat_request_factory =
            ChatRequestFactory::new(model.into(), temperature, ai_query_config, question.into());
        let client = reqwest::Client::new();
        let url = url.into();
        Self {
            chat_request_factory,
            client,
            url,
            auth_token,
        }
    }

    pub async fn query(&self, code: impl AsRef<str>) -> anyhow::Result<f32> {
        let chat_request = self.chat_request_factory.create_json(code.as_ref())?;

        let url = reqwest::Url::parse(&format!("{}/chat/completions", self.url))?;

        let request = self
            .client
            .post(url)
            .body(chat_request)
            .header("Content-Type", "application/json");
        let request = match &self.auth_token {
            Some(auth_token) => request.bearer_auth(auth_token),
            None => request,
        };
        let request = request.build()?;

        let response = self.client.execute(request).await?;
        let response: Value = serde_json::from_str(&response.text().await?)?;
        let response = response
            .get("choices")
            .ok_or(anyhow::anyhow!("No choices in response: {:?}", response))?;
        let response = response
            .get(0)
            .ok_or(anyhow::anyhow!("No choice in response: {:?}", response))?;
        let response = response
            .get("message")
            .ok_or(anyhow::anyhow!("No message in response: {:?}", response))?;
        let response = response
            .get("content")
            .ok_or(anyhow::anyhow!("No content in response: {:?}", response))?;
        let response = response.as_str().ok_or(anyhow::anyhow!(
            "No string content in response: {:?}",
            response
        ))?;

        self.chat_request_factory
            .ai_query_config
            .extract_result(response)
    }
}

#[cfg(test)]
mod tests {
    use super::{AiQueryConfig, DefaultAiQueryConfig};

    #[test]
    fn extract_result_parses_score() {
        let config = DefaultAiQueryConfig;
        let score = config
            .extract_result(r#"{"score":0.42}"#)
            .expect("score parsed");
        assert!((score - 0.42).abs() < f32::EPSILON);
    }
}
