use clipr_domain::{SubtitlePreset, TranscriptSegment};
use clipr_media::subtitles::{InterpolatedWord, WordChunk, WordTimestampInterpolator};

#[test]
fn test_interpolate_word_timestamps_proportionally() {
    let segment = TranscriptSegment::new(10.0, 14.0, "O extraordinário resultado apareceu");
    let words = WordTimestampInterpolator::interpolate_words(&segment);

    assert_eq!(words.len(), 4);
    assert_eq!(words[0].word, "O");
    assert_eq!(words[1].word, "extraordinário");
    assert_eq!(words[2].word, "resultado");
    assert_eq!(words[3].word, "apareceu");

    assert!((words[0].start_seconds - 10.0).abs() < 0.001);
    assert!((words[3].end_seconds - 14.0).abs() < 0.001);

    for i in 0..words.len() - 1 {
        assert!(words[i].start_seconds < words[i].end_seconds);
        assert!((words[i].end_seconds - words[i + 1].start_seconds).abs() < 0.001);
    }

    // "extraordinário" must have longer duration than "O"
    assert!(words[1].duration() > words[0].duration());
}

#[test]
fn test_extra_weight_to_punctuation() {
    let segment = TranscriptSegment::new(0.0, 5.0, "casa, casa casa.");
    let words = WordTimestampInterpolator::interpolate_words(&segment);

    assert_eq!(words.len(), 3);
    // "casa." has strong punctuation (.!?), duration should be longer than simple "casa"
    assert!(words[2].duration() > words[1].duration());
    // "casa," has comma, duration should also be longer than simple "casa"
    assert!(words[0].duration() > words[1].duration());
}

#[test]
fn test_group_words_into_chunks_of_two_to_four_words() {
    let words = vec![
        InterpolatedWord::new("Hoje", 0.0, 0.5),
        InterpolatedWord::new("nós", 0.5, 0.8),
        InterpolatedWord::new("vamos", 0.8, 1.3),
        InterpolatedWord::new("revolucionar", 1.3, 2.0),
        InterpolatedWord::new("a", 2.0, 2.2),
        InterpolatedWord::new("nossa", 2.2, 2.6),
        InterpolatedWord::new("empresa", 2.6, 3.2),
    ];

    let chunks = WordTimestampInterpolator::group_into_chunks(&words, 2, 4);
    assert!(!chunks.is_empty());

    for chunk in &chunks {
        assert!(
            chunk.words.len() >= 2 && chunk.words.len() <= 4,
            "Each chunk should have between 2 and 4 words. Found: {}",
            chunk.words.len()
        );
    }

    let total_words: usize = chunks.iter().map(|c| c.words.len()).sum();
    assert_eq!(total_words, 7);
    assert!((chunks.first().unwrap().start_seconds - 0.0).abs() < 0.001);
    assert!((chunks.last().unwrap().end_seconds - 3.2).abs() < 0.001);
}

#[test]
fn test_split_chunks_on_speech_pauses() {
    let words = vec![
        InterpolatedWord::new("Primeira", 0.0, 0.5),
        InterpolatedWord::new("frase", 0.5, 1.0),
        // Pause of 1.5s
        InterpolatedWord::new("Segunda", 2.5, 3.0),
        InterpolatedWord::new("frase", 3.0, 3.5),
    ];

    let chunks = WordTimestampInterpolator::group_into_chunks(&words, 2, 4);
    assert_eq!(chunks.len(), 2, "Should split into 2 chunks due to silence between speeches");
    assert_eq!(chunks[0].words[0].word, "Primeira");
    assert_eq!(chunks[0].words[1].word, "frase");
    assert_eq!(chunks[1].words[0].word, "Segunda");
    assert_eq!(chunks[1].words[1].word, "frase");
}

#[test]
fn test_generate_karaoke_events_with_highlight_color() {
    let chunk = WordChunk::new(
        vec![
            InterpolatedWord::new("CORTE", 0.0, 0.8),
            InterpolatedWord::new("VIRAL", 0.8, 1.5),
            InterpolatedWord::new("AGORA", 1.5, 2.0),
        ],
        0.0,
        2.0,
    );

    let preset = SubtitlePreset::HormoziYellow;
    let events = WordTimestampInterpolator::generate_karaoke_events(
        &[chunk],
        preset,
        false,
        false,
    );

    assert_eq!(events.len(), 3);

    // Event 1 highlights "CORTE"
    let first = &events[0];
    assert!((first.start_seconds - 0.0).abs() < 0.001);
    assert!((first.end_seconds - 0.8).abs() < 0.001);
    assert!(first.formatted_ass_text.contains(&format!(r"{{\c{}&}}CORTE{{\c{}&}}", preset.highlight_color(), preset.primary_color())));
    assert!(first.formatted_ass_text.contains("VIRAL AGORA"));

    // Event 2 highlights "VIRAL"
    let second = &events[1];
    assert!((second.start_seconds - 0.8).abs() < 0.001);
    assert!((second.end_seconds - 1.5).abs() < 0.001);
    assert!(second.formatted_ass_text.contains(&format!(r"CORTE {{\c{}&}}VIRAL{{\c{}&}} AGORA", preset.highlight_color(), preset.primary_color())));

    // Event 3 highlights "AGORA"
    let third = &events[2];
    assert!((third.start_seconds - 1.5).abs() < 0.001);
    assert!((third.end_seconds - 2.0).abs() < 0.001);
    assert!(third.formatted_ass_text.contains(&format!(r"CORTE VIRAL {{\c{}&}}AGORA{{\c{}&}}", preset.highlight_color(), preset.primary_color())));
}

#[test]
fn test_alternative_presets_in_events() {
    let chunk = WordChunk::new(
        vec![
            InterpolatedWord::new("TESTE", 0.0, 1.0),
            InterpolatedWord::new("NEON", 1.0, 2.0),
        ],
        0.0,
        2.0,
    );

    let cyber_events = WordTimestampInterpolator::generate_karaoke_events(
        std::slice::from_ref(&chunk),
        SubtitlePreset::CyberGreen,
        false,
        false,
    );
    assert!(cyber_events[0].formatted_ass_text.contains(SubtitlePreset::CyberGreen.highlight_color()));
    assert!(cyber_events[0].formatted_ass_text.contains(SubtitlePreset::CyberGreen.primary_color()));

    let white_events = WordTimestampInterpolator::generate_karaoke_events(
        &[chunk],
        SubtitlePreset::CleanWhite,
        false,
        false,
    );
    assert!(white_events[0].formatted_ass_text.contains(SubtitlePreset::CleanWhite.highlight_color()));
    assert!(white_events[0].formatted_ass_text.contains(SubtitlePreset::CleanWhite.primary_color()));
}

#[test]
fn test_interpolate_and_generate_events_for_clip_window() {
    let segments = vec![
        TranscriptSegment::new(0.0, 10.0, "trecho antes do inicio"),
        TranscriptSegment::new(10.0, 15.0, "começo do corte viral imperdível"),
        TranscriptSegment::new(15.0, 20.0, "final do corte viral"),
        TranscriptSegment::new(25.0, 30.0, "trecho depois do corte"),
    ];

    let events = WordTimestampInterpolator::interpolate_and_generate_events(
        &segments,
        10.0,
        20.0,
        SubtitlePreset::HormoziYellow,
        false,
        false,
    );

    assert!(!events.is_empty());
    assert!((events.first().unwrap().start_seconds - 0.0).abs() < 0.001);
    assert!(events.last().unwrap().end_seconds <= 10.01);

    for event in &events {
        assert!(!event.formatted_ass_text.contains("INICIO"));
        assert!(!event.formatted_ass_text.contains("DEPOIS"));
    }
}

#[test]
fn test_handle_edge_cases_gracefully() {
    assert!(WordTimestampInterpolator::interpolate_words(&TranscriptSegment::new(0.0, 5.0, "")).is_empty());
    assert!(WordTimestampInterpolator::interpolate_words(&TranscriptSegment::new(0.0, 5.0, "   ")).is_empty());
    assert!(WordTimestampInterpolator::group_into_chunks(&[], 2, 4).is_empty());
    assert!(WordTimestampInterpolator::generate_karaoke_events(&[], SubtitlePreset::HormoziYellow, false, false).is_empty());

    // Single word
    let single = WordTimestampInterpolator::interpolate_words(&TranscriptSegment::new(2.0, 3.0, "Única"));
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].word, "Única");
    assert!((single[0].start_seconds - 2.0).abs() < 0.001);
    assert!((single[0].end_seconds - 3.0).abs() < 0.001);

    let single_chunk = WordTimestampInterpolator::group_into_chunks(&single, 2, 4);
    assert_eq!(single_chunk.len(), 1);
    assert_eq!(single_chunk[0].words.len(), 1);

    let single_event = WordTimestampInterpolator::generate_karaoke_events(&single_chunk, SubtitlePreset::HormoziYellow, false, false);
    assert_eq!(single_event.len(), 1);
    assert!(single_event[0].formatted_ass_text.contains("ÚNICA"));
}

#[test]
fn test_very_long_sentences_over_50_words() {
    let mut raw_tokens = Vec::new();
    for i in 1..=60 {
        let punct = if i % 7 == 0 { "." } else { "" };
        raw_tokens.push(format!("palavra{i}{punct}"));
    }
    let text = raw_tokens.join(" ");

    let segment = TranscriptSegment::new(0.0, 60.0, text);
    let words = WordTimestampInterpolator::interpolate_words(&segment);

    assert_eq!(words.len(), 60);
    assert!((words.first().unwrap().start_seconds - 0.0).abs() < 0.001);
    assert!((words.last().unwrap().end_seconds - 60.0).abs() < 0.001);

    for i in 0..words.len() - 1 {
        assert!(words[i].start_seconds < words[i].end_seconds);
        assert!((words[i].end_seconds - words[i + 1].start_seconds).abs() < 0.001);
    }

    let chunks = WordTimestampInterpolator::group_into_chunks(&words, 2, 4);
    assert!(!chunks.is_empty());

    let mut total_words = 0;
    for chunk in &chunks {
        assert!(
            chunk.words.len() >= 2 && chunk.words.len() <= 4,
            "Chunk size must be between 2 and 4, found: {}",
            chunk.words.len()
        );
        total_words += chunk.words.len();
    }
    assert_eq!(total_words, 60);

    let events = WordTimestampInterpolator::generate_karaoke_events(&chunks, SubtitlePreset::CyberGreen, false, false);
    assert_eq!(events.len(), 60);
}

#[test]
fn test_extreme_punctuation() {
    let segment = TranscriptSegment::new(
        5.0,
        15.0,
        "Opa!!!!!!! Sério?????? Não... Mas, afinal::: sim!? ... !?",
    );

    let words = WordTimestampInterpolator::interpolate_words(&segment);
    assert!(!words.is_empty());
    assert!((words.first().unwrap().start_seconds - 5.0).abs() < 0.001);
    assert!((words.last().unwrap().end_seconds - 15.0).abs() < 0.001);

    for w in &words {
        assert!(w.duration() > 0.0);
    }

    let chunks = WordTimestampInterpolator::group_into_chunks(&words, 2, 4);
    assert!(!chunks.is_empty());

    let events = WordTimestampInterpolator::generate_karaoke_events(&chunks, SubtitlePreset::HormoziYellow, false, false);
    assert!(!events.is_empty());
}

#[test]
fn test_emojis_and_multilingual_utf8() {
    let segment = TranscriptSegment::new(
        0.0,
        10.0,
        "🔥 Olá mundo! 日本語 🌟 مرحبا Привет 🤖 bênção & ação!",
    );

    let words = WordTimestampInterpolator::interpolate_words(&segment);
    assert!(!words.is_empty());
    assert!((words.first().unwrap().start_seconds - 0.0).abs() < 0.001);
    assert!((words.last().unwrap().end_seconds - 10.0).abs() < 0.001);

    for w in &words {
        assert!(w.duration() > 0.0);
    }
}
