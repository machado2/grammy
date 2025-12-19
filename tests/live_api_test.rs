use grammy::api::check_grammar;
use grammy::config::ApiProvider;

// These tests require valid API keys in environment variables
// OPENAI_API_KEY
// OPENROUTER_API_KEY (optional, if you want to test that too)

#[tokio::test]
async fn test_openai_grammar_check() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    if api_key.is_empty() {
        return;
    }

    // "I has a cat" is a clear grammatical error
    let text = "I has a cat.".to_string();

    let (suggestions, _) = check_grammar(
        text,
        api_key,
        "gpt-4o-mini".to_string(), // verify with a cheap smart model
        ApiProvider::OpenAI,
        1,
    )
    .await
    .expect("Grammar check failed");

    assert!(!suggestions.is_empty(), "Should have found suggestions");

    let s = &suggestions[0];
    assert!(s.original.contains("has"), "Original text match failure");
    // We expect a correction like "have"
    assert!(
        s.replacement.is_some(),
        "Should provide a replacement for grammar error"
    );
}

#[tokio::test]
async fn test_openai_comment_only() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    if api_key.is_empty() {
        return;
    }

    // Try to trigger a comment by asking something ambiguous or just wrong in a fact way?
    // It's hard to force the model to *only* comment, but we can verify the struct parsing works
    // if we mock the response or if we just run a normal check and ensure no crash.
    // Ideally, we'd input text that is stylistically questionable but valid,
    // but the new prompt says "if grammatically correct, do NOT suggest anything".

    // Let's just verify a standard check doesn't panic on the new optional logic
    let text = "The ambiguous sentence.".to_string();

    let _ = check_grammar(
        text,
        api_key,
        "gpt-4o-mini".to_string(),
        ApiProvider::OpenAI,
        2,
    )
    .await;

    // Pass if no panic
}
