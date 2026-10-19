use clipr_ai::{GeminiApiKeyResolver, OpenAiApiKeyResolver};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_parse_gemini_key_from_content() {
    let bash_content = r#"
# Minha config do bash
export OTHER_VAR="abc"
export GEMINI_API_KEY="AIzaSySecretGeminiKey123"
export FOO=bar
"#;
    let resolved = GeminiApiKeyResolver::parse_key_from_content(bash_content);
    assert_eq!(resolved, Some("AIzaSySecretGeminiKey123".to_string()));

    let zsh_content = r#"
typeset -x GEMINI_API_KEY='AIzaSySecretGeminiSingleQuotes';
"#;
    let resolved_zsh = GeminiApiKeyResolver::parse_key_from_content(zsh_content);
    assert_eq!(resolved_zsh, Some("AIzaSySecretGeminiSingleQuotes".to_string()));

    let bare_content = r#"
GEMINI_API_KEY=AIzaSyBareKey456
"#;
    let resolved_bare = GeminiApiKeyResolver::parse_key_from_content(bare_content);
    assert_eq!(resolved_bare, Some("AIzaSyBareKey456".to_string()));
}

#[test]
fn test_parse_openai_key_from_content() {
    let env_content = r#"
# .env local
OPENAI_API_KEY="sk-proj-1234567890abcdef"
"#;
    let resolved = OpenAiApiKeyResolver::parse_key_from_content(env_content);
    assert_eq!(resolved, Some("sk-proj-1234567890abcdef".to_string()));
}

#[test]
fn test_parse_key_from_temp_file() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "export GEMINI_API_KEY=\"AIzaFromTempFile\"").unwrap();

    let resolved = GeminiApiKeyResolver::parse_key_from_file(temp.path());
    assert_eq!(resolved, Some("AIzaFromTempFile".to_string()));
}

#[test]
fn test_ignores_comments_and_variable_references() {
    let content = r#"
# export GEMINI_API_KEY="in_comment_not_valid"
export GEMINI_API_KEY=$OTHER_VAR
"#;
    let resolved = GeminiApiKeyResolver::parse_key_from_content(content);
    assert_eq!(resolved, None);
}
