// Copyright 2026 The Sashiko Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use crate::ai::AiMessage;
use serde_json::json;

fn response_body() -> Value {
    json!({
        "message": {
            "role": "assistant",
            "content": "{\"answer\":42}",
            "tool_calls": [{
                "function": {"name": "search", "arguments": {"query": "test"}}
            }]
        },
        "total_duration": 1,
        "load_duration": 1,
        "prompt_eval_count": 3,
        "prompt_eval_duration": 1,
        "eval_count": 4,
        "eval_duration": 1
    })
}

#[test]
fn response_finish_reason_preserves_payload_and_marks_only_length_as_truncated() -> Result<()> {
    for (reason, truncated) in [
        (Some(json!("length")), true),
        (Some(json!("stop")), false),
        (Some(json!("future-reason")), false),
        (Some(Value::Null), false),
        (None, false),
    ] {
        let fields = response_body().as_object().unwrap().clone();
        let body = Value::Object(
            fields
                .into_iter()
                .chain(
                    reason
                        .clone()
                        .map(|value| ("done_reason".to_string(), value)),
                )
                .collect(),
        );
        let response = translate_ollama_response(serde_json::from_value(body)?)?;

        assert_eq!(response.truncated, truncated, "done_reason: {reason:?}");
        assert_eq!(response.content.as_deref(), Some("{\"answer\":42}"));
        let calls = response.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].id.starts_with("ollama_"));
        assert_eq!(calls[0].function_name, "search");
        assert_eq!(calls[0].arguments, json!({"query": "test"}));
        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 3);
        assert_eq!(usage.completion_tokens, 4);
        assert_eq!(usage.total_tokens, 7);
    }
    Ok(())
}

#[tokio::test]
async fn generate_content_retains_the_http_response_truncation_reason() -> Result<()> {
    use crate::ai::test_http;

    let body = json!({
        "message": {"role": "assistant", "content": "{\"answer\":42}"},
        "done": true,
        "done_reason": "length",
        "total_duration": 1,
        "load_duration": 1,
        "prompt_eval_count": 3,
        "prompt_eval_duration": 1,
        "eval_count": 4,
        "eval_duration": 1
    });
    let server = test_http::serve(test_http::complete("200 OK", "", &body.to_string())).await;
    let client = OllamaClient {
        client: test_http::test_client(),
        ..OllamaClient::new(
            server.url.clone(),
            "test-model".to_string(),
            128_000,
            2048,
            5,
            None,
        )?
    };
    let response = client
        .generate_content(AiRequest {
            system: None,
            messages: vec![AiMessage {
                role: AiRole::User,
                content: Some("Return the answer.".to_string()),
                thought: None,
                thought_signature: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: None,
            temperature: None,
            response_format: None,
            context_tag: None,
            reasoning_effort: None,
        })
        .await;
    server.finish().await;

    let response = response?;
    assert_eq!(response.content.as_deref(), Some("{\"answer\":42}"));
    assert!(response.truncated);
    Ok(())
}
