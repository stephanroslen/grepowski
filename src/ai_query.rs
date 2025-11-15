use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
struct ChatRequestMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Clone, Debug)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatRequestMessage>,
    temperature: f32,
    max_tokens: usize,
}

#[derive(Debug, Clone)]
struct ChatRequestFactory {
    model: String,
    temperature: f32,
    max_tokens: usize,
    system_prompt: String,
    question: String,
}

impl ChatRequestFactory {
    fn new(
        model: String,
        temperature: f32,
        max_tokens: usize,
        system_prompt: String,
        question: String,
    ) -> Self {
        Self {
            model,
            temperature,
            max_tokens,
            system_prompt,
            question,
        }
    }

    fn create_system_message(&self) -> ChatRequestMessage {
        ChatRequestMessage {
            role: "system".to_string(),
            content: format!("{} Question: {}", self.system_prompt, self.question),
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
        ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
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
}

impl AI {
    pub fn new(
        model: impl Into<String>,
        url: impl Into<String>,
        temperature: f32,
        max_tokens: usize,
        system_prompt: impl Into<String>,
        question: impl Into<String>,
    ) -> Self {
        let chat_request_factory = ChatRequestFactory::new(
            model.into(),
            temperature,
            max_tokens,
            system_prompt.into(),
            question.into(),
        );
        let client = reqwest::Client::new();
        let url = url.into();
        Self {
            chat_request_factory,
            client,
            url,
        }
    }

    pub async fn query(&self, code: impl AsRef<str>) -> anyhow::Result<f32> {
        let chat_request = self.chat_request_factory.create_json(code.as_ref())?;
        let request = self
            .client
            .post(reqwest::Url::parse(&self.url)?)
            .body(chat_request)
            .build()?;
        let response = self.client.execute(request).await?;
        let response: serde_json::Value = serde_json::from_str(&response.text().await?)?;
        let response = response.get("choices").ok_or(anyhow::anyhow!("No choices in response"))?;
        let response = response.get(0).ok_or(anyhow::anyhow!("No choice in response"))?;
        let response = response.get("message").ok_or(anyhow::anyhow!("No message in response"))?;
        let response = response.get("content").ok_or(anyhow::anyhow!("No content in response"))?;
        let response = response.as_str().ok_or(anyhow::anyhow!("No string content in response"))?;
        let response : f32 = response.parse()?;

        Ok(response)
    }
}
