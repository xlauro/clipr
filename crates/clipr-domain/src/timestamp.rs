use serde::{Deserialize, Serialize};

/// Representa uma palavra individual com timestamp de início e fim.
/// Utilizado para legendagem dinâmica de alta retenção (karaokê estilo Hormozi/Shorts/Reels)
/// e alinhamento semântico com dicionários visuais (ex: emojis virais).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordTimestamp {
    pub word: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub confidence: f64,
}

impl WordTimestamp {
    pub fn new(word: impl Into<String>, start_seconds: f64, end_seconds: f64) -> Self {
        Self {
            word: word.into(),
            start_seconds,
            end_seconds,
            confidence: 1.0,
        }
    }

    pub fn with_confidence(
        word: impl Into<String>,
        start_seconds: f64,
        end_seconds: f64,
        confidence: f64,
    ) -> Self {
        Self {
            word: word.into(),
            start_seconds,
            end_seconds,
            confidence,
        }
    }

    /// Retorna a duração em segundos da palavra.
    pub fn duration(&self) -> f64 {
        (self.end_seconds - self.start_seconds).max(0.0)
    }

    /// Retorna a palavra limpa de pontuações e espaços extras para matching e busca.
    pub fn clean_word(&self) -> String {
        self.word
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Retorna o intervalo formatado no padrão [HH:MM:SS.mmm -> HH:MM:SS.mmm].
    pub fn formatted_time_range(&self) -> String {
        format!(
            "[{} -> {}]",
            format_seconds(self.start_seconds),
            format_seconds(self.end_seconds)
        )
    }
}

fn format_seconds(seconds: f64) -> String {
    let clamped = seconds.max(0.0);
    let total_sec = clamped as u64;
    let mut hours = total_sec / 3600;
    let mut minutes = (total_sec % 3600) / 60;
    let mut secs = total_sec % 60;
    let mut millis = ((clamped - total_sec as f64) * 1000.0).round() as u64;

    if millis >= 1000 {
        secs += 1;
        millis -= 1000;
    }
    if secs >= 60 {
        minutes += 1;
        secs -= 60;
    }
    if minutes >= 60 {
        hours += 1;
        minutes -= 60;
    }
    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, secs, millis)
}
