use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::pin::Pin;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::openai::*;
use crate::services::content_filter;
use crate::services::message_adapter;
use crate::services::parameter_validator::ParameterValidator;
use crate::AppState;

/// POST /v1/chat/completions
pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Rate limit
    state.rate_limiters.check("chat").map_err(AppError::RateLimited)?;

    // Verify API key
    let bearer = extract_bearer(&headers);
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    // Verify Claude auth
    if !state.auth_manager.auth_status.valid {
        return Err(AppError::ServiceUnavailable(
            "Claude Code authentication failed. Check /v1/auth/status for details.".to_string(),
        ));
    }

    // Validate request
    request.validate().map_err(AppError::BadRequest)?;

    let request_id = format!("chatcmpl-{}", &Uuid::new_v4().to_string()[..8]);

    // Extract Claude-specific headers
    let claude_headers = ParameterValidator::extract_claude_headers(&headers);

    if request.is_streaming() {
        let stream = generate_streaming_response(state, request, request_id, claude_headers);
        Ok(Sse::new(stream).into_response())
    } else {
        let response = generate_non_streaming_response(state, request, &request_id, claude_headers).await?;
        Ok(Json(response).into_response())
    }
}

async fn generate_non_streaming_response(
    state: AppState,
    request: ChatCompletionRequest,
    request_id: &str,
    claude_headers: std::collections::HashMap<String, serde_json::Value>,
) -> Result<ChatCompletionResponse, AppError> {
    // Process messages with session management
    let (all_messages, actual_session_id) = state
        .session_manager
        .process_messages(&request.messages, request.session_id.as_deref())
        .await;

    info!(
        "Chat completion: session_id={:?}, total_messages={}",
        actual_session_id,
        all_messages.len()
    );

    // Convert messages to prompt
    let (mut prompt, mut system_prompt) = message_adapter::messages_to_prompt(&all_messages);

    // Add sampling instructions
    if let Some(sampling) = request.get_sampling_instructions() {
        system_prompt = Some(match system_prompt {
            Some(sp) => format!("{sp}\n\n{sampling}"),
            None => sampling,
        });
    }

    // Filter content
    prompt = content_filter::filter_content(&prompt);
    if let Some(ref sp) = system_prompt {
        system_prompt = Some(content_filter::filter_content(sp));
    }

    // Build tool options
    let (allowed, disallowed, permission_mode) =
        build_tool_options(&request, &claude_headers);

    let model = claude_headers
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| request.model.clone());

    // Run Claude CLI
    let result = state
        .claude_cli
        .run_completion(
            &prompt,
            system_prompt.as_deref(),
            Some(&model),
            allowed.as_deref(),
            disallowed.as_deref(),
            permission_mode.as_deref(),
        )
        .await
        .map_err(AppError::Internal)?;

    if result.text.is_empty() {
        return Err(AppError::Internal("No response from Claude Code".to_string()));
    }

    // Filter the response
    let assistant_content = content_filter::filter_content(&result.text);

    // Add to session
    if let Some(ref sid) = actual_session_id {
        let msg = Message {
            role: "assistant".to_string(),
            content: assistant_content.clone(),
            name: None,
        };
        state.session_manager.add_assistant_response(sid, msg).await;
    }

    // Estimate tokens
    let prompt_tokens = message_adapter::estimate_tokens(&prompt);
    let completion_tokens = message_adapter::estimate_tokens(&assistant_content);

    Ok(ChatCompletionResponse {
        id: request_id.to_string(),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: request.model.clone(),
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: "assistant".to_string(),
                content: assistant_content,
                name: None,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: Some(Usage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }),
        system_fingerprint: None,
    })
}

fn generate_streaming_response(
    state: AppState,
    request: ChatCompletionRequest,
    request_id: String,
    claude_headers: std::collections::HashMap<String, serde_json::Value>,
) -> Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> {
    let stream = async_stream::stream! {
        // Process messages with session management
        let (all_messages, actual_session_id) = state
            .session_manager
            .process_messages(&request.messages, request.session_id.as_deref())
            .await;

        // Convert messages to prompt
        let (mut prompt, mut system_prompt) = message_adapter::messages_to_prompt(&all_messages);

        // Add sampling instructions
        if let Some(sampling) = request.get_sampling_instructions() {
            system_prompt = Some(match system_prompt {
                Some(sp) => format!("{sp}\n\n{sampling}"),
                None => sampling,
            });
        }

        // Filter content
        prompt = content_filter::filter_content(&prompt);
        if let Some(ref sp) = system_prompt {
            system_prompt = Some(content_filter::filter_content(sp));
        }

        // Build tool options
        let (allowed, disallowed, permission_mode) =
            build_tool_options(&request, &claude_headers);

        let model_name = request.model.clone();

        // Start streaming
        let rx = match state.claude_cli.run_completion_stream(
            &prompt,
            system_prompt.as_deref(),
            Some(&model_name),
            allowed.as_deref(),
            disallowed.as_deref(),
            permission_mode.as_deref(),
        ).await {
            Ok(rx) => rx,
            Err(e) => {
                error!("Failed to start streaming: {e}");
                let error_data = serde_json::json!({"error": {"message": e, "type": "streaming_error"}});
                let event = Event::default().data(serde_json::to_string(&error_data).unwrap());
                yield Ok(event);
                return;
            }
        };

        let mut rx = rx;
        let mut role_sent = false;
        let mut content_sent = false;
        let mut full_text = String::new();

        while let Some(event) = rx.recv().await {
            match event {
                crate::services::claude_cli::StreamEvent::AssistantText(text) => {
                    // Send initial role chunk
                    if !role_sent {
                        let chunk = ChatCompletionStreamResponse::new(
                            &request_id, &model_name,
                            serde_json::json!({"role": "assistant", "content": ""}),
                            None,
                        );
                        let data = serde_json::to_string(&chunk).unwrap();
                        yield Ok(Event::default().data(data));
                        role_sent = true;
                    }

                    let filtered = content_filter::filter_content(&text);
                    if !filtered.is_empty() && !filtered.chars().all(|c| c.is_whitespace()) {
                        let chunk = ChatCompletionStreamResponse::new(
                            &request_id, &model_name,
                            serde_json::json!({"content": filtered}),
                            None,
                        );
                        let data = serde_json::to_string(&chunk).unwrap();
                        yield Ok(Event::default().data(data));
                        content_sent = true;
                        full_text.push_str(&filtered);
                    }
                }
                crate::services::claude_cli::StreamEvent::Result(text) => {
                    // Skip result if we already sent assistant content (avoids duplication)
                    if content_sent {
                        continue;
                    }
                    if !role_sent {
                        let chunk = ChatCompletionStreamResponse::new(
                            &request_id, &model_name,
                            serde_json::json!({"role": "assistant", "content": ""}),
                            None,
                        );
                        let data = serde_json::to_string(&chunk).unwrap();
                        yield Ok(Event::default().data(data));
                        role_sent = true;
                    }

                    let filtered = content_filter::filter_content(&text);
                    if !filtered.is_empty() {
                        let chunk = ChatCompletionStreamResponse::new(
                            &request_id, &model_name,
                            serde_json::json!({"content": filtered}),
                            None,
                        );
                        let data = serde_json::to_string(&chunk).unwrap();
                        yield Ok(Event::default().data(data));
                        content_sent = true;
                        full_text.push_str(&filtered);
                    }
                }
                crate::services::claude_cli::StreamEvent::Error(msg) => {
                    error!("Stream error: {msg}");
                    let error_data = serde_json::json!({"error": {"message": msg, "type": "streaming_error"}});
                    yield Ok(Event::default().data(serde_json::to_string(&error_data).unwrap()));
                }
                crate::services::claude_cli::StreamEvent::Done => break,
                _ => {}
            }
        }

        // Ensure role was sent
        if !role_sent {
            let chunk = ChatCompletionStreamResponse::new(
                &request_id, &model_name,
                serde_json::json!({"role": "assistant", "content": ""}),
                None,
            );
            yield Ok(Event::default().data(serde_json::to_string(&chunk).unwrap()));
        }

        // Fallback if no content
        if !content_sent {
            let chunk = ChatCompletionStreamResponse::new(
                &request_id, &model_name,
                serde_json::json!({"content": "I'm unable to provide a response at the moment."}),
                None,
            );
            yield Ok(Event::default().data(serde_json::to_string(&chunk).unwrap()));
        }

        // Store in session
        if let Some(ref sid) = actual_session_id {
            if !full_text.is_empty() {
                let msg = Message {
                    role: "assistant".to_string(),
                    content: full_text.clone(),
                    name: None,
                };
                state.session_manager.add_assistant_response(sid, msg).await;
            }
        }

        // Usage chunk
        if let Some(ref opts) = request.stream_options {
            if opts.include_usage {
                let prompt_tokens = message_adapter::estimate_tokens(&prompt);
                let completion_tokens = message_adapter::estimate_tokens(&full_text);
                let mut chunk = ChatCompletionStreamResponse::new(
                    &request_id, &model_name,
                    serde_json::json!({}),
                    Some("stop".to_string()),
                );
                chunk.usage = Some(Usage {
                    prompt_tokens,
                    completion_tokens,
                    total_tokens: prompt_tokens + completion_tokens,
                });
                yield Ok(Event::default().data(serde_json::to_string(&chunk).unwrap()));
            }
        }

        // Final stop chunk
        let final_chunk = ChatCompletionStreamResponse::new(
            &request_id, &model_name,
            serde_json::json!({}),
            Some("stop".to_string()),
        );
        yield Ok(Event::default().data(serde_json::to_string(&final_chunk).unwrap()));

        // [DONE] sentinel
        yield Ok(Event::default().data("[DONE]"));
    };

    Box::pin(stream)
}

fn build_tool_options(
    request: &ChatCompletionRequest,
    claude_headers: &std::collections::HashMap<String, serde_json::Value>,
) -> (Option<Vec<String>>, Option<Vec<String>>, Option<String>) {
    let mut allowed: Option<Vec<String>> = None;
    let mut disallowed: Option<Vec<String>> = None;
    let mut permission_mode: Option<String> = None;

    if !request.tools_enabled() {
        // Disable all tools via --tools ""
        disallowed = Some(crate::constants::CLAUDE_TOOLS.iter().map(|s| s.to_string()).collect());
        info!("Tools disabled (default behavior for OpenAI compatibility)");
    } else {
        // Enable default safe tools
        allowed = Some(crate::constants::DEFAULT_ALLOWED_TOOLS.iter().map(|s| s.to_string()).collect());
        permission_mode = Some("bypassPermissions".to_string());
        info!("Tools enabled by user request");
    }

    // Override from headers
    if let Some(v) = claude_headers.get("allowed_tools").and_then(|v| v.as_array()) {
        allowed = Some(v.iter().filter_map(|s| s.as_str().map(String::from)).collect());
    }
    if let Some(v) = claude_headers.get("disallowed_tools").and_then(|v| v.as_array()) {
        disallowed = Some(v.iter().filter_map(|s| s.as_str().map(String::from)).collect());
    }
    if let Some(v) = claude_headers.get("permission_mode").and_then(|v| v.as_str()) {
        permission_mode = Some(v.to_string());
    }

    (allowed, disallowed, permission_mode)
}

fn extract_bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}
