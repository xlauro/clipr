use clipr_domain::{OrientationMode, SubtitlePreset, TranscriptSegment, WordTimestamp};
use clipr_media::subtitles::{SubtitleGenerator, WordTimestampInterpolator};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_generate_ass_subtitles_correctly() {
    let temp_dir = tempdir().unwrap();
    let target_ass = temp_dir.path().join("teste_9_16.ass");

    let segments = vec![
        TranscriptSegment::new(5.0, 10.0, "trecho antes do corte"),
        TranscriptSegment::new(10.0, 20.0, "gancho do corte viral"),
        TranscriptSegment::new(20.0, 30.0, "desfecho da história"),
        TranscriptSegment::new(35.0, 40.0, "trecho após o corte"),
    ];

    let res = SubtitleGenerator::generate_ass_subtitles(
        &segments,
        10.0,
        30.0,
        OrientationMode::Vertical916,
        SubtitlePreset::HormoziYellow,
        &target_ass,
        true,
        false,
    );
    assert!(res.is_ok());
    assert!(target_ass.exists());

    let content = fs::read_to_string(&target_ass).unwrap();

    assert!(content.contains("[Script Info]"));
    assert!(content.contains("PlayResX: 1080"));
    assert!(content.contains("PlayResY: 1920"));
    assert!(content.contains("Style: Viral,Montserrat ExtraBold,64"));
    assert!(content.contains(",40,40,620,1"));
    assert!(content.contains("&H0000FFFF"));
    assert!(content.contains(",1,8,2,2,40,40,620,1"));

    assert!(content.contains("GANCHO"));
    assert!(content.contains("DO"));
    assert!(content.contains("CORTE"));
    assert!(content.contains("DESFECHO"));
    // Highlight green (&H0000FF00)
    assert!(content.contains(r"{\c&H0000FF00&}"));

    // Outside portions must not be present
    assert!(!content.contains("TRECHO ANTES DO CORTE"));
    assert!(!content.contains("TRECHO APÓS O CORTE"));
}

#[test]
fn test_generate_ass_subtitles_for_horizontal_16_9() {
    let temp_dir = tempdir().unwrap();
    let target_ass = temp_dir.path().join("teste_16_9.ass");

    let segments = vec![TranscriptSegment::new(10.0, 20.0, "discussão em formato horizontal")];

    let res = SubtitleGenerator::generate_ass_subtitles(
        &segments,
        10.0,
        20.0,
        OrientationMode::Horizontal169,
        SubtitlePreset::HormoziYellow,
        &target_ass,
        false,
        false,
    );
    assert!(res.is_ok());
    assert!(target_ass.exists());

    let content = fs::read_to_string(&target_ass).unwrap();
    assert!(content.contains("PlayResX: 1920"));
    assert!(content.contains("PlayResY: 1080"));
    assert!(content.contains("Style: Viral,Montserrat ExtraBold,48"));
    assert!(content.contains(",40,40,100,1"));
    assert!(content.contains("DISCUSSÃO"));
    assert!(content.contains("HORIZONTAL"));
}

#[test]
fn test_support_alternative_presets_in_generator() {
    let temp_dir = tempdir().unwrap();
    let segments = vec![TranscriptSegment::new(0.0, 5.0, "estilo visual moderno")];

    // Cyber Green
    let cyber_ass = temp_dir.path().join("cyber.ass");
    SubtitleGenerator::generate_ass_subtitles(
        &segments,
        0.0,
        5.0,
        OrientationMode::Vertical916,
        SubtitlePreset::CyberGreen,
        &cyber_ass,
        false,
        false,
    )
    .unwrap();
    let cyber_content = fs::read_to_string(&cyber_ass).unwrap();
    assert!(cyber_content.contains(SubtitlePreset::CyberGreen.primary_color()));
    assert!(cyber_content.contains(SubtitlePreset::CyberGreen.highlight_color()));

    // Clean White
    let white_ass = temp_dir.path().join("white.ass");
    SubtitleGenerator::generate_ass_subtitles(
        &segments,
        0.0,
        5.0,
        OrientationMode::Vertical916,
        SubtitlePreset::CleanWhite,
        &white_ass,
        false,
        false,
    )
    .unwrap();
    let white_content = fs::read_to_string(&white_ass).unwrap();
    assert!(white_content.contains(SubtitlePreset::CleanWhite.primary_color()));
    assert!(white_content.contains(SubtitlePreset::CleanWhite.highlight_color()));
}

#[test]
fn test_provide_fonts_directory() {
    let fonts_dir = SubtitleGenerator::get_fonts_directory();
    assert!(fonts_dir.is_some(), "Fonts directory should not be None");
    let dir = fonts_dir.unwrap();
    assert!(dir.is_dir(), "Fonts directory should exist on disk");
    assert!(dir.join("Montserrat-ExtraBold.otf").is_file());
}

#[test]
fn test_format_ass_time_correctly() {
    assert_eq!(SubtitleGenerator::format_ass_time(0.0), "0:00:00.00");
    assert_eq!(SubtitleGenerator::format_ass_time(1.5), "0:00:01.50");
    assert_eq!(SubtitleGenerator::format_ass_time(65.25), "0:01:05.25");
    assert_eq!(SubtitleGenerator::format_ass_time(3600.0), "1:00:00.00");
}

#[test]
fn test_format_srt_time_correctly() {
    assert_eq!(SubtitleGenerator::format_srt_time(0.0), "00:00:00,000");
    assert_eq!(SubtitleGenerator::format_srt_time(1.234), "00:00:01,234");
    assert_eq!(SubtitleGenerator::format_srt_time(65.5), "00:01:05,500");
    assert_eq!(SubtitleGenerator::format_srt_time(3600.0), "01:00:00,000");
}

#[test]
fn test_generate_srt_from_word_timestamps() {
    let temp_dir = tempdir().unwrap();
    let srt_file = temp_dir.path().join("test_subtitles.srt");

    let words = vec![
        WordTimestamp::new("Ganhe", 0.0, 0.4),
        WordTimestamp::new("muito", 0.4, 0.8),
        WordTimestamp::new("dinheiro", 0.8, 1.5),
        WordTimestamp::new("com", 1.6, 2.0),
        WordTimestamp::new("inteligência", 2.0, 2.7),
        WordTimestamp::new("artificial", 2.7, 3.5),
    ];

    let res = SubtitleGenerator::generate_srt_subtitles_from_words(&words, &srt_file, true);
    assert!(res.is_ok());
    assert!(srt_file.exists());

    let srt_content = fs::read_to_string(&srt_file).unwrap();
    assert!(srt_content.contains("1\n"));
    assert!(srt_content.contains(" --> "));
    assert!(srt_content.contains("DINHEIRO 💰"));
    assert!(srt_content.contains("00:00:00,000 --> "));
}

#[test]
fn test_generate_srt_from_transcript_segments() {
    let temp_dir = tempdir().unwrap();
    let srt_file = temp_dir.path().join("segments.srt");

    let segments = vec![TranscriptSegment::new(0.0, 5.0, "O segredo para vencer na vida")];

    let res = SubtitleGenerator::generate_srt_subtitles(&segments, 0.0, 5.0, &srt_file, true);
    assert!(res.is_ok());
    assert!(srt_file.exists());

    let content = fs::read_to_string(&srt_file).unwrap();
    assert!(content.contains("SEGREDO 💡") || content.contains("VENCER 🏆"));
}

#[test]
fn test_generate_ass_with_pop_zoom_and_emojis() {
    let temp_dir = tempdir().unwrap();
    let ass_file = temp_dir.path().join("pop_zoom.ass");

    let words = vec![
        WordTimestamp::new("Corte", 0.0, 0.5),
        WordTimestamp::new("viral", 0.5, 1.2),
        WordTimestamp::new("insano", 1.2, 1.8),
    ];

    let res = SubtitleGenerator::generate_ass_subtitles_from_words(
        &words,
        OrientationMode::Vertical916,
        SubtitlePreset::HormoziYellow,
        &ass_file,
        true,
        true,
    );
    assert!(res.is_ok());
    assert!(ass_file.exists());

    let content = fs::read_to_string(&ass_file).unwrap();
    assert!(content.contains("[Script Info]"));
    assert!(content.contains("Dialogue: 0,"));
    assert!(content.contains(r"\t(0,80,\fscx115\fscy115)\t(80,160,\fscx100\fscy100)"));
    assert!(content.contains("🔥"));
}

#[test]
fn test_support_all_modern_subtitle_presets() {
    let temp_dir = tempdir().unwrap();
    let words = vec![
        WordTimestamp::new("Teste", 0.0, 1.0),
        WordTimestamp::new("Preset", 1.0, 2.0),
    ];

    let presets = [
        SubtitlePreset::HormoziYellow,
        SubtitlePreset::MrbeastBlueYellow,
        SubtitlePreset::CyberGreen,
        SubtitlePreset::NeonMagenta,
        SubtitlePreset::MinimalistElegant,
        SubtitlePreset::CleanWhite,
    ];

    for preset in presets {
        let file = temp_dir.path().join(format!("preset_{:?}.ass", preset));
        let res = SubtitleGenerator::generate_ass_subtitles_from_words(
            &words,
            OrientationMode::Vertical916,
            preset,
            &file,
            false,
            false,
        );
        assert!(res.is_ok());
        assert!(file.exists());
        let ass = fs::read_to_string(&file).unwrap();
        assert!(ass.contains(preset.primary_color()));
        assert!(ass.contains(preset.highlight_color()));
    }
}

#[test]
fn test_group_word_timestamps_into_two_to_three_words() {
    let words = vec![
        WordTimestamp::new("W1", 0.0, 0.5),
        WordTimestamp::new("W2", 0.5, 1.0),
        WordTimestamp::new("W3", 1.0, 1.5),
        WordTimestamp::new("W4", 1.5, 2.0),
        WordTimestamp::new("W5", 2.0, 2.5),
        WordTimestamp::new("W6", 2.5, 3.0),
        WordTimestamp::new("W7", 3.0, 3.5),
    ];

    let chunks = WordTimestampInterpolator::group_timestamps_into_chunks(&words, 2, 3);
    assert!(!chunks.is_empty());

    for chunk in &chunks {
        assert!(
            chunk.words.len() >= 2 && chunk.words.len() <= 3,
            "Each chunk should have between 2 and 3 words. Found: {}",
            chunk.words.len()
        );
    }
}
