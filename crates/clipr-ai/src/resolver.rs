use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionResult {
    pub key: String,
    pub source: String,
}

impl ResolutionResult {
    pub fn new(key: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            source: source.into(),
        }
    }

    pub fn is_found(&self) -> bool {
        !self.key.trim().is_empty()
    }

    pub fn not_found() -> Self {
        Self {
            key: String::new(),
            source: "Não encontrada".to_string(),
        }
    }
}

pub struct GeminiApiKeyResolver;

pub struct OpenAiApiKeyResolver;

static GEMINI_KEY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\s*(?:(?:export|typeset\s+-x|declare\s+-x)\s+)?GEMINI_API_KEY\s*=\s*(?:"([^"]*)"|'([^']*)'|([^#\s]+))"#).unwrap()
});

static OPENAI_KEY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\s*(?:(?:export|typeset\s+-x|declare\s+-x)\s+)?OPENAI_API_KEY\s*=\s*(?:"([^"]*)"|'([^']*)'|([^#\s]+))"#).unwrap()
});

impl GeminiApiKeyResolver {
    pub const ENV_VAR_NAME: &'static str = "GEMINI_API_KEY";

    pub fn resolve_with_details() -> ResolutionResult {
        resolve_key_generic(Self::ENV_VAR_NAME, &GEMINI_KEY_PATTERN)
    }

    pub fn resolve_api_key() -> Option<String> {
        let res = Self::resolve_with_details();
        if res.is_found() {
            Some(res.key)
        } else {
            None
        }
    }

    pub fn resolve() -> Option<String> {
        Self::resolve_api_key()
    }

    pub fn parse_key_from_content(content: &str) -> Option<String> {
        parse_key_from_text(content, Self::ENV_VAR_NAME, &GEMINI_KEY_PATTERN)
    }

    pub fn parse_key_from_file(path: &Path) -> Option<String> {
        parse_key_from_file_path(path, Self::ENV_VAR_NAME, &GEMINI_KEY_PATTERN)
    }
}

impl OpenAiApiKeyResolver {
    pub const ENV_VAR_NAME: &'static str = "OPENAI_API_KEY";

    pub fn resolve_with_details() -> ResolutionResult {
        resolve_key_generic(Self::ENV_VAR_NAME, &OPENAI_KEY_PATTERN)
    }

    pub fn resolve_api_key() -> Option<String> {
        let res = Self::resolve_with_details();
        if res.is_found() {
            Some(res.key)
        } else {
            None
        }
    }

    pub fn resolve() -> Option<String> {
        Self::resolve_api_key()
    }

    pub fn parse_key_from_content(content: &str) -> Option<String> {
        parse_key_from_text(content, Self::ENV_VAR_NAME, &OPENAI_KEY_PATTERN)
    }

    pub fn parse_key_from_file(path: &Path) -> Option<String> {
        parse_key_from_file_path(path, Self::ENV_VAR_NAME, &OPENAI_KEY_PATTERN)
    }
}

fn resolve_key_generic(env_var_name: &str, pattern: &Regex) -> ResolutionResult {
    // 1. Variável de ambiente direta do sistema
    if let Ok(val) = std::env::var(env_var_name) {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            info!("{env_var_name} encontrada em std::env");
            return ResolutionResult::new(trimmed, format!("Variável de ambiente ({env_var_name})"));
        }
    }

    // 2. Arquivos de configuração de shell (.zshrc, .bashrc, etc.) e .env
    let home_dir = dirs::home_dir().unwrap_or_else(|| {
        std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    });
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let candidates: Vec<(PathBuf, &str)> = vec![
        (home_dir.join(".zshrc"), "~/.zshrc"),
        (home_dir.join(".zshenv"), "~/.zshenv"),
        (home_dir.join(".zprofile"), "~/.zprofile"),
        (home_dir.join(".bashrc"), "~/.bashrc"),
        (home_dir.join(".bash_profile"), "~/.bash_profile"),
        (home_dir.join(".profile"), "~/.profile"),
        (current_dir.join(".env"), ".env (projeto)"),
        (home_dir.join(".env"), "~/.env"),
    ];

    for (path, desc) in candidates {
        if let Some(key) = parse_key_from_file_path(&path, env_var_name, pattern) {
            info!("{env_var_name} encontrada em {desc}");
            return ResolutionResult::new(key, desc);
        }
    }

    // 3. Fallback: execução de subprocesso zsh/bash
    if let Some(key) = resolve_from_shell("zsh", "-c", &format!("source ~/.zshrc 2>/dev/null; echo -n \"${env_var_name}\"")) {
        info!("{env_var_name} encontrada via shell zsh");
        return ResolutionResult::new(key, "Shell zsh (~/.zshrc)");
    }

    if let Some(key) = resolve_from_shell("bash", "-c", &format!("source ~/.bashrc 2>/dev/null; echo -n \"${env_var_name}\"")) {
        info!("{env_var_name} encontrada via shell bash");
        return ResolutionResult::new(key, "Shell bash (~/.bashrc)");
    }

    warn!("{env_var_name} não foi encontrada nas variáveis de ambiente ou arquivos de shell.");
    ResolutionResult::not_found()
}

fn parse_key_from_file_path(path: &Path, env_var_name: &str, pattern: &Regex) -> Option<String> {
    if !path.is_file() {
        return None;
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            // Tenta leitura tolerante binária decodificada como ISO-8859-1
            match std::fs::read(path) {
                Ok(bytes) => bytes.iter().map(|&b| b as char).collect(),
                Err(_) => return None,
            }
        }
    };

    parse_key_from_text(&content, env_var_name, pattern)
}

fn parse_key_from_text(content: &str, env_var_name: &str, pattern: &Regex) -> Option<String> {
    let mut last_found = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || !trimmed.contains(env_var_name) {
            continue;
        }

        if let Some(caps) = pattern.captures(trimmed) {
            let candidate = caps.get(1)
                .or_else(|| caps.get(2))
                .or_else(|| caps.get(3))
                .map(|m| m.as_str());

            if let Some(mut val) = candidate {
                val = val.trim_end_matches(';').trim();
                if !val.is_empty() && !val.starts_with('$') {
                    last_found = Some(val.to_string());
                }
            }
        }
    }
    last_found
}

fn resolve_from_shell(program: &str, arg1: &str, arg2: &str) -> Option<String> {
    let output = Command::new(program)
        .arg(arg1)
        .arg(arg2)
        .output()
        .ok()?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !text.is_empty() {
            return Some(text);
        }
    }
    None
}
