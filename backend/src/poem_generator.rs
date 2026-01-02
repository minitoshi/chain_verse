use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

pub struct PoemGenerator {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl PoemGenerator {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    /// Generate a poem from a list of keywords with retry logic
    pub async fn generate_poem(&self, keywords: &[String]) -> Result<String> {
        self.generate_poem_with_retry(keywords, 3).await
    }

    /// Generate a poem with configurable retry attempts
    async fn generate_poem_with_retry(&self, keywords: &[String], max_retries: u32) -> Result<String> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            if attempt > 0 {
                let delay_secs = 2u64.pow(attempt); // Exponential backoff: 2, 4, 8 seconds
                println!("⏳ Retry attempt {} after {} seconds...", attempt + 1, delay_secs);
                tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
            }

            match self.try_generate_poem(keywords).await {
                Ok(poem) => return Ok(poem),
                Err(e) => {
                    println!("⚠️  Attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed after {} attempts", max_retries)))
    }

    /// Single attempt to generate a poem
    async fn try_generate_poem(&self, keywords: &[String]) -> Result<String> {
        let prompt = self.create_prompt(keywords);

        let request = OpenRouterRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let response = self
            .client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("OpenRouter API error: {}", error_text);
        }

        let response_data: OpenRouterResponse = response.json().await?;

        let poem = response_data
            .choices
            .first()
            .context("No choices in response")?
            .message
            .content
            .clone();

        Ok(poem)
    }

    /// Create a prompt for poem generation
    fn create_prompt(&self, keywords: &[String]) -> String {
        let keywords_str = keywords.join(", ");

        format!(
            r#"You are a poetic AI that creates beautiful, evocative poems.

Using ONLY the following keywords derived from the Solana blockchain, create a cohesive poem of 20-30 lines.

Keywords: {}

Instructions:
- Use all or most of these keywords naturally in the poem
- Create a coherent narrative or emotional arc
- The poem can be any mood - happy, sad, dark, light, mysterious, etc.
- Let the words guide the tone naturally
- Use vivid imagery and metaphor
- Make it flow well and feel complete
- Do NOT add a title
- Do NOT explain or comment on the poem
- ONLY output the poem itself

Write the poem now:"#,
            keywords_str
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_prompt() {
        let generator = PoemGenerator::new(
            "test_key".to_string(),
            "test_model".to_string(),
        );

        let keywords = vec!["moon".to_string(), "silence".to_string(), "journey".to_string()];
        let prompt = generator.create_prompt(&keywords);

        assert!(prompt.contains("moon"));
        assert!(prompt.contains("silence"));
        assert!(prompt.contains("journey"));
        assert!(prompt.contains("20-30 lines"));
    }
}
