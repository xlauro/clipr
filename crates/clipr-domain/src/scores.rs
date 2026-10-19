use serde::{Deserialize, Serialize};

/// Detalhamento das pontuações do algoritmo de viralização.
/// Pesos da rubrica:
/// - Hook: 25%
/// - Retention: 20%
/// - Emotional: 15%
/// - Storytelling: 15%
/// - Standalone: 15%
/// - Visual: 10%
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub hook: i32,
    pub retention: i32,
    pub emotional: i32,
    pub storytelling: i32,
    pub standalone: i32,
    pub visual: i32,
}

impl ScoreBreakdown {
    pub fn new(
        hook: i32,
        retention: i32,
        emotional: i32,
        storytelling: i32,
        standalone: i32,
        visual: i32,
    ) -> Self {
        Self {
            hook: hook.clamp(0, 100),
            retention: retention.clamp(0, 100),
            emotional: emotional.clamp(0, 100),
            storytelling: storytelling.clamp(0, 100),
            standalone: standalone.clamp(0, 100),
            visual: visual.clamp(0, 100),
        }
    }

    pub fn from_uniform(uniform_score: i32) -> Self {
        let clamped = uniform_score.clamp(0, 100);
        Self {
            hook: clamped,
            retention: clamped,
            emotional: clamped,
            storytelling: clamped,
            standalone: clamped,
            visual: clamped,
        }
    }

    pub fn default_breakdown() -> Self {
        Self::from_uniform(70)
    }

    pub fn calculate_weighted_virality_score(&self) -> i32 {
        let weighted = (0.25 * self.hook as f64)
            + (0.20 * self.retention as f64)
            + (0.15 * self.emotional as f64)
            + (0.15 * self.storytelling as f64)
            + (0.15 * self.standalone as f64)
            + (0.10 * self.visual as f64);
        (weighted.round() as i32).clamp(0, 100)
    }
}

impl Default for ScoreBreakdown {
    fn default() -> Self {
        Self::default_breakdown()
    }
}
