use clipr_ai::PromptFactory;

#[test]
fn test_system_instruction_contains_core_rules() {
    let sys = PromptFactory::SYSTEM_INSTRUCTION;
    assert!(sys.contains("DIREÇÃO DE CONTEÚDO MULTIMODAL"));
    assert!(sys.contains("RUBRICA PONDERADA DE 6 CRITÉRIOS"));
    assert!(sys.contains("Hook (Peso 25%)"));
    assert!(sys.contains("Retenção (Peso 20%)"));
    assert!(sys.contains("Impacto Emocional (Peso 15%)"));
    assert!(sys.contains("Storytelling (Peso 15%)"));
    assert!(sys.contains("Standalone (Peso 15%)"));
    assert!(sys.contains("Força Visual (Peso 10%)"));
    assert!(sys.contains("POLEMICA_QUENTE"));
    assert!(sys.contains("ALTA_ENERGIA"));
    assert!(sys.contains("REGRA DO GANCHO (HOOK NOS PRIMEIROS 3 SEGUNDOS)"));
    assert!(sys.contains("SPLIT_SCREEN"));
    assert!(sys.contains("HORMOZI_YELLOW"));
}

#[test]
fn test_build_user_prompt() {
    let prompt = PromptFactory::build_user_prompt(
        "Como Ficar Rico",
        "Canal Financeiro",
        "[00:00:10.000 -> 00:00:40.000] O primeiro passo é poupar...",
    );

    assert!(prompt.contains(r#"VÍDEO: "Como Ficar Rico" por Canal Financeiro"#));
    assert!(prompt.contains("[00:00:10.000 -> 00:00:40.000] O primeiro passo é poupar..."));
    assert!(prompt.contains("CORTES CURTOS / SHORTS (30s a 90s)"));
    assert!(prompt.contains("CORTES LONGOS / DEEP DIVES"));
}

#[test]
fn test_build_multimodal_user_prompt() {
    let with_transcript = PromptFactory::build_multimodal_user_prompt(
        "Podcast IA",
        "Tech Cast",
        Some("[00:01:00.000 -> 00:02:00.000] Discussão sobre AGI..."),
    );
    assert!(with_transcript.contains("TRANSCRIÇÃO COM TIMESTAMPS DE SUPORTE:"));
    assert!(with_transcript.contains("Discussão sobre AGI"));

    let without_transcript = PromptFactory::build_multimodal_user_prompt(
        "Vídeo Visual",
        "Criador",
        None,
    );
    assert!(without_transcript.contains("TRANSCRIÇÃO: Não disponível"));
}
