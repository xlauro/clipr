use clipr_domain::{ThumbnailAspectRatio, VideoExportLayout};
use clipr_media::thumbnail::{ThumbnailGenerator, ThumbnailResult, TARGET_HEIGHT, TARGET_VERTICAL_HEIGHT, TARGET_VERTICAL_WIDTH, TARGET_WIDTH};
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_resolve_ffmpeg_binary() {
    let generator = ThumbnailGenerator::new();
    let binary = generator.get_ffmpeg_binary();
    assert!(!binary.is_empty());
    assert!(binary.contains("ffmpeg"));
}

#[test]
fn test_resolve_aspect_ratio_from_label() {
    assert_eq!(
        ThumbnailAspectRatio::from_label("16:9 Horizontal (YouTube)"),
        ThumbnailAspectRatio::Horizontal169
    );
    assert_eq!(
        ThumbnailAspectRatio::from_label("9:16 Vertical (TikTok / Reels / Shorts)"),
        ThumbnailAspectRatio::Vertical916
    );
    assert_eq!(
        ThumbnailAspectRatio::from_label("Vertical"),
        ThumbnailAspectRatio::Vertical916
    );
    assert_eq!(
        ThumbnailAspectRatio::from_label(""),
        ThumbnailAspectRatio::Horizontal169
    );
}

#[test]
fn test_dimensions_constants() {
    assert_eq!(TARGET_WIDTH, 1280);
    assert_eq!(TARGET_HEIGHT, 720);
    assert_eq!(TARGET_VERTICAL_WIDTH, 1080);
    assert_eq!(TARGET_VERTICAL_HEIGHT, 1920);

    assert_eq!(ThumbnailAspectRatio::Horizontal169.width(), 1280);
    assert_eq!(ThumbnailAspectRatio::Horizontal169.height(), 720);
    assert_eq!(ThumbnailAspectRatio::Vertical916.width(), 1080);
    assert_eq!(ThumbnailAspectRatio::Vertical916.height(), 1920);
}

#[test]
fn test_build_extract_frame_command_with_scale_crop_for_vertical_9_16() {
    let generator = ThumbnailGenerator::with_binary("/usr/bin/ffmpeg");
    let video = Path::new("/tmp/video.mp4");
    let out = Path::new("/tmp/out_v916.jpg");

    let cmd = generator.build_extract_frame_command(
        video,
        45.5,
        out,
        ThumbnailAspectRatio::Vertical916,
    );

    assert!(cmd.contains(&"-vf".to_string()));
    let vf_idx = cmd.iter().position(|r| r == "-vf").unwrap();
    let vf_filter = &cmd[vf_idx + 1];
    assert!(vf_filter.contains("scale=1080:1920:force_original_aspect_ratio=increase"));
    assert!(vf_filter.contains("crop=1080:1920"));
    assert!(cmd.contains(&"-ss".to_string()));
    assert!(cmd.contains(&"45.500".to_string()));
}

#[test]
fn test_build_extract_frame_command_standard_for_horizontal_16_9() {
    let generator = ThumbnailGenerator::with_binary("/usr/bin/ffmpeg");
    let video = Path::new("/tmp/video.mp4");
    let out = Path::new("/tmp/out_16x9.jpg");

    let cmd = generator.build_extract_frame_command(
        video,
        12.0,
        out,
        ThumbnailAspectRatio::Horizontal169,
    );

    assert!(!cmd.contains(&"-vf".to_string()), "Clean 16:9 extract does not force -vf");
    assert!(cmd.contains(&"-vframes".to_string()));
    assert!(cmd.contains(&"1".to_string()));
    assert!(cmd.contains(&"-q:v".to_string()));
    assert!(cmd.contains(&"2".to_string()));
}

#[test]
fn test_wrap_headline_correctly() {
    // 1. Short title (2 words) -> 1 line
    let short_lines = ThumbnailGenerator::wrap_headline("CORTE ÉPICO", 22, 3);
    assert_eq!(short_lines.len(), 1);
    assert_eq!(short_lines[0], "CORTE ÉPICO");

    // 2. Medium title (4 words) -> Wraps into at least 2 lines with max 3 words per line
    let medium_lines = ThumbnailGenerator::wrap_headline("COMO VIRALIZAR NO YOUTUBE", 22, 3);
    assert!(medium_lines.len() >= 2);
    for line in &medium_lines {
        let words: Vec<&str> = line.split_whitespace().collect();
        assert!(words.len() <= 3, "Each line must contain at most 3 words");
    }

    // 3. Very long title (> 12 words) -> Capped to 4 lines with ellipsis
    let long_title = "COMO GANHAR DINHEIRO NA INTERNET UTILIZANDO INTELIGENCIA ARTIFICIAL E FERRAMENTAS MODERNAS DE EDICAO EM 2026";
    let long_lines = ThumbnailGenerator::wrap_headline(long_title, 22, 3);
    assert!(long_lines.len() <= 4, "Must not exceed 4 lines to preserve layout");
    assert!(long_lines.last().unwrap().ends_with("..."), "Last line must end with ellipsis");
}

#[test]
fn test_wrap_giant_headline_over_300_characters() {
    let giant_headline = "ESTA É UMA HEADLINE EXTREMAMENTE LONGA COM MAIS DE TREZENTOS CARACTERES CRIADA ESPECIFICAMENTE PARA TESTAR O SISTEMA DE WORD WRAP DINÂMICO DO CLIPPR E GARANTIR QUE NENHUM TEXTO ULTRAPASSE AS MARGENS DE SEGURANÇA DO YOUTUBE NEM PROVOQUE OVERFLOW DE BUFFER OU EXCEÇÃO GRÁFICA EM RESOLUÇÕES HD 1280X720";
    assert!(giant_headline.len() > 300);

    let wrapped = ThumbnailGenerator::wrap_headline(giant_headline, 22, 3);
    assert_eq!(wrapped.len(), 4, "Giant headlines must be capped to 4 lines max");
    assert!(wrapped.last().unwrap().ends_with("..."));
}

#[test]
fn test_handle_empty_or_blank_headlines() {
    assert!(ThumbnailGenerator::wrap_headline("", 22, 3).is_empty());
    assert!(ThumbnailGenerator::wrap_headline("   ", 22, 3).is_empty());
    assert!(ThumbnailGenerator::wrap_headline("\t\n\r", 22, 3).is_empty());
}

#[test]
fn test_compose_thumbnail_command_layout() {
    let generator = ThumbnailGenerator::with_binary("/usr/bin/ffmpeg");
    let raw = Path::new("/tmp/raw.jpg");
    let out = Path::new("/tmp/out.jpg");

    // Horizontal 16:9
    let cmd_h = generator.compose_thumbnail_command(
        raw,
        out,
        "O SEGREDO REVELADO",
        ThumbnailAspectRatio::Horizontal169,
    );
    assert!(cmd_h.contains(&"-vf".to_string()));
    let vf_h = cmd_h[cmd_h.iter().position(|r| r == "-vf").unwrap() + 1].clone();
    assert!(vf_h.contains("scale=1280:720"));
    assert!(vf_h.contains("borderw=6:bordercolor=black"));
    assert!(vf_h.contains("x=70"));

    // Vertical 9:16
    let cmd_v = generator.compose_thumbnail_command(
        raw,
        out,
        "COMO VIRALIZAR NOS SHORTS",
        ThumbnailAspectRatio::Vertical916,
    );
    let vf_v = cmd_v[cmd_v.iter().position(|r| r == "-vf").unwrap() + 1].clone();
    assert!(vf_v.contains("scale=1080:1920"));
    assert!(vf_v.contains("x=(w-text_w)/2"));
}

#[test]
fn test_resolve_thumbnail_inside_thumbnails_dir_with_video_export_layout() {
    let temp_dir = tempdir().unwrap();
    let layout = VideoExportLayout::resolve(temp_dir.path(), "Vídeo de Teste", "v_abc123");
    let thumb = layout.resolve_thumbnail("thumb_clip_10_thumb.jpg");
    let raw = layout.resolve_thumbnail("thumb_clip_10_raw.jpg");

    assert!(thumb.starts_with(&layout.thumbnails_dir));
    assert!(raw.starts_with(&layout.thumbnails_dir));
    assert_eq!(
        thumb.parent().unwrap().file_name().unwrap().to_str().unwrap(),
        "thumbnails"
    );
    assert_eq!(
        thumb.file_name().unwrap().to_str().unwrap(),
        "thumb_clip_10_thumb.jpg"
    );
}

#[test]
fn test_thumbnail_result_struct() {
    let res = ThumbnailResult {
        final_thumbnail_path: Path::new("/tmp/thumb.jpg").to_path_buf(),
        raw_frame_path: Path::new("/tmp/raw.jpg").to_path_buf(),
    };
    assert_eq!(res.final_thumbnail_path, Path::new("/tmp/thumb.jpg"));
    assert_eq!(res.raw_frame_path, Path::new("/tmp/raw.jpg"));
}
