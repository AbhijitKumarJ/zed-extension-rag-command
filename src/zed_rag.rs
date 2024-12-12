// Import necessary modules from zed and serde crates
use serde::Deserialize;
use zed::{
    http_client::{HttpMethod, HttpRequest},
    serde_json::{self, json},
};
use zed_extension_api::{self as zed, http_client::RedirectPolicy, Result};

// Main extension struct
struct RagExtension;

// Response structure from the API containing choices
#[derive(Deserialize, Debug)]
struct ApiResponse {
    choices: Vec<Choice>,
}

// Structure representing a single choice from the API response
#[derive(Deserialize, Debug)]
struct Choice {
    message: Message,
}

// Structure containing the actual message content
#[derive(Deserialize, Debug)]
struct Message {
    content: String,
}

// Implementation of the Zed Extension trait for RagExtension
impl zed::Extension for RagExtension {
    // Constructor for the extension
    fn new() -> Self {
        Self
    }

    // Handler for slash commands
    fn run_slash_command(
        &self,
        command: zed::SlashCommand,
        arguments: Vec<String>,
        worktree: Option<&zed::Worktree>,
    ) -> Result<zed::SlashCommandOutput> {
        // Verify the command is "rag"
        if command.name != "rag" {
            return Err("Invalid command. Expected 'rag'.".into());
        }

        // Combine all arguments into a single query string
        let query = arguments.join(" ");
        if query.is_empty() {
            return Ok(zed::SlashCommandOutput {
                text: "Error: Query not provided. Please enter a prompt.".to_string(),
                sections: vec![],
            });
        }

        // Prepare the HTTP request to the RAG API
        let request = HttpRequest {
			method: HttpMethod::Post,
			url: "http://127.0.0.1:8000/v1/chat/completions".to_string(),
			headers: vec![("Content-Type".to_string(), "application/json".to_string())],
			body: Some(
							// Construct the JSON payload for the API request
							serde_json::to_vec(&json!({
											"model": "openai:gpt-4o",
											"messages": [
															{ "role": "system", "content": "You are a helpful consultant." },
															{ "role": "user", "content": query }
											],
											"temperature": 0.1
							}))
							.map_err(|err| format!("Failed to serialize request body: {}", err))?,
			),
			redirect_policy: RedirectPolicy::FollowAll,
		};

        // Initialize the streaming connection to the API
        let mut stream = zed::http_client::fetch_stream(&request)
            .map_err(|err| format!("HTTP request failed: {}", err))?;

        // Collect response sections
        let mut sections = Vec::new();
        loop {
            // Process each chunk from the stream
            match stream
                .next_chunk()
                .map_err(|err| format!("Stream error: {}", err))?
            {
                Some(response_chunk) => {
                    // Convert chunk bytes to string
                    let response_body = String::from_utf8(response_chunk)
                        .map_err(|err| format!("Failed to parse response chunk: {}", err))?;

                    // Parse the JSON response
                    let api_response: ApiResponse = serde_json::from_str(&response_body)
                        .map_err(|err| format!("Failed to deserialize API response: {}", err))?;

                    // Process each choice in the response
                    for choice in api_response.choices {
                        sections.push(zed::SlashCommandOutputSection {
                            range: (0..choice.message.content.len()).into(),
                            label: choice.message.content,
                        });
                    }
                }
                None => break, // Exit loop when stream is finished
            }
        }

        // Return the final formatted output
        Ok(zed::SlashCommandOutput {
            text: "RAG API Response".to_string(),
            sections,
        })
    }
}

// Register the extension with Zed
zed::register_extension!(RagExtension);
