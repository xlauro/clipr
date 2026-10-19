use clipr_domain::{ClipCandidate, FramingMode, OrientationMode, ProcessingStatus, VideoExportLayout, VideoSource};
use clipr_media::ffmpeg::{FfmpegExecutor, AUDIO_FADE_DURATION_SECONDS, TEMPORAL_PADDING_SECONDS};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_resolve_ffmpeg_binary() {
    let executor = FfmpegExecutor::new();
    let binary = executor.get_ffmpeg_binary();
    assert!(!binary.is_empty(), "FFmpeg binary path should not be empty");
    assert!(binary.contains("ffmpeg"), "Command should point to ffmpeg");
}

#[test]
fn test_resolve_video_encoder() {
    let executor = FfmpegExecutor::new();
    let encoder = executor.get_video_encoder();
    assert!(!encoder.is_empty());
    let supported = ["libx264", "libopenh264", "h264_nvenc", "h264_vaapi", "h264_qsv", "h264_videotoolbox"];
    assert!(
        supported.contains(&encoder),
        "Encoder '{}' should be one of supported encoders",
        encoder
    );
}

#[test]
fn test_respect_custom_video_encoder() {
    let executor = FfmpegExecutor::with_binary_and_encoder("ffmpeg", "libopenh264");
    assert_eq!(executor.get_video_encoder(), "libopenh264");

    let executor2 = FfmpegExecutor::with_binary_and_encoder("ffmpeg", "libx264");
    assert_eq!(executor2.get_video_encoder(), "libx264");
}

#[test]
fn test_build_command_with_libx264_settings() {
    let executor = FfmpegExecutor::with_binary_and_encoder("ffmpeg", "libx264");
    let cmd = executor.build_ffmpeg_command(
        Path::new("input.mp4"),
        Path::new("output.mp4"),
        10.0,
        30.0,
        FramingMode::AutoCrop,
        None,
        4,
        true,
    );

    assert!(cmd.contains(&"-c:v".to_string()));
    let cv_idx = cmd.iter().position(|r| r == "-c:v").unwrap();
    assert_eq!(cmd[cv_idx + 1], "libx264");
    assert!(cmd.contains(&"-preset".to_string()));
    assert!(cmd.contains(&"fast".to_string()));
    assert!(cmd.contains(&"-crf".to_string()));
    assert!(cmd.contains(&"20".to_string()));
    assert!(cmd.contains(&"-pix_fmt".to_string()));
    assert!(cmd.contains(&"yuv420p".to_string()));
}

#[test]
fn test_build_command_with_libopenh264_settings() {
    let executor = FfmpegExecutor::with_binary_and_encoder("ffmpeg", "libopenh264");
    let cmd = executor.build_ffmpeg_command(
        Path::new("input.mp4"),
        Path::new("output.mp4"),
        0.0,
        15.0,
        FramingMode::Horizontal169,
        None,
        0,
        false,
    );

    assert!(cmd.contains(&"-c:v".to_string()));
    let cv_idx = cmd.iter().position(|r| r == "-c:v").unwrap();
    assert_eq!(cmd[cv_idx + 1], "libopenh264");
    assert!(cmd.contains(&"-b:v".to_string()));
    assert!(cmd.contains(&"4M".to_string()));
    assert!(cmd.contains(&"-pix_fmt".to_string()));
    assert!(cmd.contains(&"yuv420p".to_string()));
    assert!(!cmd.contains(&"-crf".to_string()), "libopenh264 should not receive -crf");
}

#[test]
fn test_include_ass_filter_when_file_exists() {
    let temp_dir = tempdir().unwrap();
    let ass_file = temp_dir.path().join("sub_test.ass");
    fs::write(&ass_file, "[Script Info]\n").unwrap();

    let executor = FfmpegExecutor::with_binary_and_encoder("ffmpeg", "libopenh264");
    let cmd = executor.build_ffmpeg_command(
        Path::new("input.mp4"),
        Path::new("output.mp4"),
        0.0,
        10.0,
        FramingMode::AutoCrop,
        Some(&ass_file),
        2,
        false,
    );

    let vf_idx = cmd.iter().position(|r| r == "-vf").unwrap();
    let vf_filter = &cmd[vf_idx + 1];
    assert!(vf_filter.contains("ass='"), "Filter must reference .ass file: {}", vf_filter);
}

#[test]
fn test_video_filter_graph_for_auto_crop() {
    let executor = FfmpegExecutor::new();
    let filter = executor.build_video_filter_graph(FramingMode::AutoCrop, None);
    assert_eq!(filter, "crop=ih*9/16:ih:(iw-ow)/2:0,scale=1080:1920");
}

#[test]
fn test_video_filter_graph_for_split_screen() {
    let executor = FfmpegExecutor::new();
    let filter = executor.build_video_filter_graph(FramingMode::SplitScreen, None);
    assert!(filter.contains("split=2[left][right]"));
    assert!(filter.contains("crop=iw/2:ih:0:0,scale=1080:960[top]"));
    assert!(filter.contains("crop=iw/2:ih:iw/2:0,scale=1080:960[bottom]"));
    assert!(filter.contains("[top][bottom]vstack=inputs=2"));
}

#[test]
fn test_video_filter_graph_for_blurred_bg() {
    let executor = FfmpegExecutor::new();
    let filter = executor.build_video_filter_graph(FramingMode::BlurredBg, None);
    assert!(filter.contains("split[v1][v2]"));
    assert!(filter.contains("scale=1080:1920:force_original_aspect_ratio=increase,crop=1080:1920,boxblur=20:5[bg]"));
    assert!(filter.contains("scale=1080:1920:force_original_aspect_ratio=decrease[fg]"));
    assert!(filter.contains("[bg][fg]overlay=(W-w)/2:(H-h)/2"));
}

#[test]
fn test_video_filter_graph_for_horizontal_16_9() {
    let executor = FfmpegExecutor::new();
    let filter = executor.build_video_filter_graph(FramingMode::Horizontal169, None);
    assert_eq!(
        filter,
        "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2"
    );
}

#[test]
fn test_audio_filter_graph_with_fades_and_pacing() {
    let executor = FfmpegExecutor::new();

    // Without silence removal: only micro-fades
    let af_standard = executor.build_audio_filter_graph(20.0, false);
    assert_eq!(
        af_standard,
        "afade=t=in:ss=0:d=0.05,afade=t=out:st=19.950:d=0.05"
    );
    assert!(!af_standard.contains("silenceremove"));

    // With optimized pacing: silenceremove + micro-fades
    let af_optimized = executor.build_audio_filter_graph(15.5, true);
    assert!(af_optimized.contains("silenceremove=stop_periods=-1:stop_duration=0.5:stop_threshold=-40dB"));
    assert!(af_optimized.contains("afade=t=in:ss=0:d=0.05"));
    assert!(af_optimized.contains("afade=t=out:st=15.450:d=0.05"));

    // Short duration edge case: non-negative fade-out start
    let af_short = executor.build_audio_filter_graph(0.02, false);
    assert!(af_short.contains("afade=t=out:st=0.000:d=0.05"));
}

#[test]
fn test_constants_values() {
    assert!((TEMPORAL_PADDING_SECONDS - 0.2).abs() < 0.001);
    assert!((AUDIO_FADE_DURATION_SECONDS - 0.05).abs() < 0.001);
}

#[test]
fn test_parse_progress_seconds_hours_greater_than_99() {
    let line1 = "frame= 1000 fps=60 time=100:00:00.00 bitrate= 100kbits/s";
    let parsed1 = FfmpegExecutor::parse_progress_seconds(line1);
    assert!(parsed1.is_some());
    assert!((parsed1.unwrap() - 360000.0).abs() < 0.01);

    let line2 = "frame= 5500 fps=30 time=125:30:15.50 bitrate= 250kbits/s";
    let parsed2 = FfmpegExecutor::parse_progress_seconds(line2);
    assert!(parsed2.is_some());
    assert!((parsed2.unwrap() - 451815.5).abs() < 0.01);

    let line3 = "frame= 9999 fps=60 time=999:59:59.99 bitrate= 500kbits/s";
    let parsed3 = FfmpegExecutor::parse_progress_seconds(line3);
    assert!(parsed3.is_some());
    assert!((parsed3.unwrap() - 3599999.99).abs() < 0.01);
}

#[test]
fn test_parse_progress_seconds_anomalous_ms() {
    let cases = [
        ("time=00:01:23.4", 83.4),
        ("time=00:00:05.1", 5.1),
        ("time=00:10:20.123456", 620.123456),
        ("time=00:05:30", 330.0),
        ("time=01:00:00.000001", 3600.000001),
    ];

    for (line, expected) in cases {
        let parsed = FfmpegExecutor::parse_progress_seconds(line);
        assert!(parsed.is_some(), "Expected progress for '{}'", line);
        assert!((parsed.unwrap() - expected).abs() < 0.0001);
    }
}

#[test]
fn test_parse_progress_seconds_invalid_lines() {
    let invalid = [
        "frame=  120 fps= 60 q=28.0 size=    1024kB",
        "[libx264 @ 0x55d78a9c] using cpu capabilities: MMX2 SSE2Fast SSSE3 SSE4.2 AVX",
        "time=@@:$$:%%",
        "time=--:--:--",
        "time=ab:cd:ef",
        "time=12:xx:34",
        "",
        "   ",
    ];

    for line in invalid {
        assert!(
            FfmpegExecutor::parse_progress_seconds(line).is_none(),
            "Line should not return progress: '{}'",
            line
        );
    }
}

#[test]
fn test_parse_progress_seconds_with_surrounding_noise() {
    let log = "[FFmpeg] >>> [pid:12345] {status: OK} size=450kB time=00:02:15.80 bitrate= 128kbits/s speed=1.98x!";
    let seconds = FfmpegExecutor::parse_progress_seconds(log);
    assert!(seconds.is_some());
    assert!((seconds.unwrap() - 135.8).abs() < 0.001);
}

#[tokio::test]
async fn test_capture_error_on_non_existent_input() {
    let temp_dir = tempdir().unwrap();
    let executor = FfmpegExecutor::new();
    let non_existent = temp_dir.path().join("missing.mp4");
    let layout = VideoExportLayout::resolve(temp_dir.path(), "Test Title", "v_123");

    let source = VideoSource::from_url("http://example.com/watch?v=v_123");
    let clip = ClipCandidate::new(
        Some(1),
        Some(1),
        "Clip 1",
        "Hook",
        0.0,
        10.0,
        90,
        "Viral reason",
        "Caption",
        vec!["#viral".to_string()],
        OrientationMode::Vertical916,
        false,
        false,
        None,
        None,
        ProcessingStatus::Pending,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );

    let res = executor.render_clip(
        &source,
        &clip,
        &non_existent,
        &layout,
        &[],
        1,
        false,
        false,
    ).await;

    assert!(res.is_err());
    let err = res.err().unwrap();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("exit code") || err_msg.contains("No such file or directory"),
        "Error message should contain FFmpeg stderr: {}",
        err_msg
    );
}
