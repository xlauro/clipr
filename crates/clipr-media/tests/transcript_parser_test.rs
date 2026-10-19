use clipr_domain::TranscriptSegment;
use clipr_media::transcript::TranscriptParser;

#[test]
fn test_parse_vtt_content_correctly() {
    let vtt = r#"WEBVTT
Kind: captions
Language: pt

00:00:01.200 --> 00:00:04.500
<c.colorE5E5E5>Olá pessoal, hoje vou revelar</c>

00:00:04.500 --> 00:00:08.150
o maior segredo da inteligência artificial.

00:00:08.150 --> 00:00:12.000
Prestem muita atenção nisso!
"#;

    let segments = TranscriptParser::parse_vtt(vtt);
    assert_eq!(segments.len(), 3);

    let first = &segments[0];
    assert!((first.start_seconds - 1.2).abs() < 0.001);
    assert!((first.end_seconds - 4.5).abs() < 0.001);
    assert_eq!(first.text, "Olá pessoal, hoje vou revelar");

    let second = &segments[1];
    assert!((second.start_seconds - 4.5).abs() < 0.001);
    assert!((second.end_seconds - 8.15).abs() < 0.001);
    assert_eq!(second.text, "o maior segredo da inteligência artificial.");

    let formatted = TranscriptParser::format_for_ai(&segments);
    assert!(formatted.contains("[00:00:01.200 -> 00:00:04.500]"));
    assert!(formatted.contains("Prestem muita atenção nisso!"));
}

#[test]
fn test_extract_plain_text_correctly() {
    let vtt = r#"WEBVTT

00:00:01.000 --> 00:00:03.000
Primeira frase importante.

00:00:03.500 --> 00:00:06.000
Segunda frase com mais detalhes.
"#;

    let plain_text = TranscriptParser::extract_plain_text_from_vtt(vtt);
    assert_eq!(
        plain_text,
        "Primeira frase importante.\nSegunda frase com mais detalhes."
    );

    let segments = vec![
        TranscriptSegment::new(1.0, 2.0, "Linha 1"),
        TranscriptSegment::new(2.5, 4.0, "Linha 2"),
    ];
    assert_eq!(
        TranscriptParser::extract_plain_text(&segments),
        "Linha 1\nLinha 2"
    );
    assert_eq!(TranscriptParser::extract_plain_text(&[]), "");
}

#[test]
fn test_ignore_headers_and_notes() {
    let vtt = r#"WEBVTT
Kind: captions
Language: pt-BR
NOTE Este arquivo foi gerado automaticamente

00:00:05,000 --> 00:00:10,000
Primeira fala real.
"#;

    let segments = TranscriptParser::parse_vtt(vtt);
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].text, "Primeira fala real.");
    assert!((segments[0].start_seconds - 5.0).abs() < 0.001);
    assert!((segments[0].end_seconds - 10.0).abs() < 0.001);
}

#[test]
fn test_handle_timestamps_without_hours() {
    let vtt = r#"WEBVTT

01:30.500 --> 01:35.000
Fala em um minuto e trinta segundos.
"#;

    let segments = TranscriptParser::parse_vtt(vtt);
    assert_eq!(segments.len(), 1);
    assert!((segments[0].start_seconds - 90.5).abs() < 0.001);
    assert!((segments[0].end_seconds - 95.0).abs() < 0.001);
}

#[test]
fn test_clean_xml_tags_and_entities() {
    let vtt = r##"WEBVTT

00:00:01.000 --> 00:00:03.000
<font color="#ffff00">Texto&nbsp;<b>em negrito</b></font> &amp; <00:00:02.000>palavra.
"##;

    let segments = TranscriptParser::parse_vtt(vtt);
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].text, "Texto em negrito & palavra.");
}

#[test]
fn test_blank_input_returns_empty() {
    assert!(TranscriptParser::parse_vtt("").is_empty());
    assert!(TranscriptParser::parse_vtt("   ").is_empty());
}

#[test]
fn test_parse_vtt_with_windows_and_unix_line_breaks() {
    let unix_vtt = "WEBVTT\n\n00:00:01.000 --> 00:00:03.000\nPrimeira fala Unix.\n\n00:00:04.000 --> 00:00:07.000\nSegunda fala Unix.\n";
    let windows_vtt = "WEBVTT\r\n\r\n00:00:01.000 --> 00:00:03.000\r\nPrimeira fala Unix.\r\n\r\n00:00:04.000 --> 00:00:07.000\r\nSegunda fala Unix.\r\n";
    let mixed_vtt = "WEBVTT\r\n\n00:00:01.000 --> 00:00:03.000\nPrimeira fala Unix.\r\n\r\n00:00:04.000 --> 00:00:07.000\nSegunda fala Unix.\r\n";

    let unix_segments = TranscriptParser::parse_vtt(unix_vtt);
    let windows_segments = TranscriptParser::parse_vtt(windows_vtt);
    let mixed_segments = TranscriptParser::parse_vtt(mixed_vtt);

    assert_eq!(unix_segments.len(), 2);
    assert_eq!(unix_segments.len(), windows_segments.len());
    assert_eq!(unix_segments.len(), mixed_segments.len());

    for i in 0..unix_segments.len() {
        assert_eq!(unix_segments[i].text, windows_segments[i].text);
        assert!((unix_segments[i].start_seconds - windows_segments[i].start_seconds).abs() < 0.001);
        assert!((unix_segments[i].end_seconds - windows_segments[i].end_seconds).abs() < 0.001);

        assert_eq!(unix_segments[i].text, mixed_segments[i].text);
        assert!((unix_segments[i].start_seconds - mixed_segments[i].start_seconds).abs() < 0.001);
        assert!((unix_segments[i].end_seconds - mixed_segments[i].end_seconds).abs() < 0.001);
    }
}

#[test]
fn test_clean_nested_and_malformed_html_and_style_tags() {
    let vtt = r#"WEBVTT

00:00:00.500 --> 00:00:04.000
<b><i><c.color.yellow>Fala crucial com cores</c></i></b> e <c.colorFFFF00.bg_black>destaque neon</c>!

00:00:05.000 --> 00:00:08.500
<b><i>Tag aberta sem fechar e <font face="Arial" size="14">fonte com atributos</font>.
"#;

    let segments = TranscriptParser::parse_vtt(vtt);
    assert_eq!(segments.len(), 2);
    assert_eq!(
        segments[0].text,
        "Fala crucial com cores e destaque neon!"
    );
    assert_eq!(
        segments[1].text,
        "Tag aberta sem fechar e fonte com atributos."
    );
}

#[test]
fn test_decode_unbalanced_and_varied_html_entities() {
    let vtt = r#"WEBVTT

00:00:01.000 --> 00:00:04.000
Tom&nbsp;&amp;&nbsp;Jerry &gt; Outros &lt;animações&gt; &quot;Top 1&quot; &#39;clássico&#39;
"#;

    let segments = TranscriptParser::parse_vtt(vtt);
    assert_eq!(segments.len(), 1);
    let text = &segments[0].text;
    assert!(text.contains("Tom & Jerry"));
    assert!(text.contains(">"));
    assert!(text.contains("\"Top 1\""));
    assert!(text.contains("'clássico'"));
    assert!(!text.contains("&nbsp;"));
    assert!(!text.contains("&amp;"));
    assert!(!text.contains("&quot;"));
}

#[test]
fn test_sort_out_of_order_timestamps() {
    let out_of_order_vtt = r#"WEBVTT

00:00:20.000 --> 00:00:25.000
Terceira fala que apareceu primeiro no arquivo.

00:00:01.000 --> 00:00:05.000
Primeira fala real do início do vídeo.

00:00:10.000 --> 00:00:15.000
Segunda fala intermediária.
"#;

    let segments = TranscriptParser::parse_vtt(out_of_order_vtt);
    assert_eq!(segments.len(), 3);

    assert_eq!(segments[0].text, "Primeira fala real do início do vídeo.");
    assert!((segments[0].start_seconds - 1.0).abs() < 0.001);
    assert!((segments[0].end_seconds - 5.0).abs() < 0.001);

    assert_eq!(segments[1].text, "Segunda fala intermediária.");
    assert!((segments[1].start_seconds - 10.0).abs() < 0.001);
    assert!((segments[1].end_seconds - 15.0).abs() < 0.001);

    assert_eq!(
        segments[2].text,
        "Terceira fala que apareceu primeiro no arquivo."
    );
    assert!((segments[2].start_seconds - 20.0).abs() < 0.001);
    assert!((segments[2].end_seconds - 25.0).abs() < 0.001);
}

#[test]
fn test_merge_overlapping_segments() {
    let overlapping_vtt = r#"WEBVTT

00:00:02.000 --> 00:00:05.000
Fala repetida

00:00:03.000 --> 00:00:08.000
Fala repetida

00:00:07.000 --> 00:00:10.000
Fala da Pessoa B enquanto A ainda concluía
"#;

    let segments = TranscriptParser::parse_vtt(overlapping_vtt);
    assert_eq!(segments.len(), 2);

    assert_eq!(segments[0].text, "Fala repetida");
    assert!((segments[0].start_seconds - 2.0).abs() < 0.001);
    assert!((segments[0].end_seconds - 8.0).abs() < 0.001);

    assert_eq!(
        segments[1].text,
        "Fala da Pessoa B enquanto A ainda concluía"
    );
    assert!((segments[1].start_seconds - 7.0).abs() < 0.001);
    assert!((segments[1].end_seconds - 10.0).abs() < 0.001);
}

#[test]
fn test_ignore_repeated_blank_lines_and_positioning_metadata() {
    let noisy_vtt = r#"WEBVTT
Kind: captions
Language: pt-BR

NOTE Comentário 1 explicativo



1
00:00:01.000 --> 00:00:03.000 line:0% position:50% align:middle
Fala com metadados de posicionamento.




NOTE Outro comentário no meio do conteúdo


2
00:00:04.000 --> 00:00:06.000
Fala final sem ruído.



"#;

    let segments = TranscriptParser::parse_vtt(noisy_vtt);
    assert_eq!(segments.len(), 2);

    assert_eq!(segments[0].text, "Fala com metadados de posicionamento.");
    assert!((segments[0].start_seconds - 1.0).abs() < 0.001);
    assert!((segments[0].end_seconds - 3.0).abs() < 0.001);

    assert_eq!(segments[1].text, "Fala final sem ruído.");
    assert!((segments[1].start_seconds - 4.0).abs() < 0.001);
    assert!((segments[1].end_seconds - 6.0).abs() < 0.001);
}

#[test]
fn test_parse_srt_format_with_comma_and_sequence_numbers() {
    let srt = r#"1
00:00:01,500 --> 00:00:04,200
Primeira linha em formato SRT.

2
00:00:04,200 --> 00:00:07,800
Segunda linha em formato SRT.
"#;

    let segments = TranscriptParser::parse_vtt(srt);
    assert_eq!(segments.len(), 2);

    assert_eq!(segments[0].text, "Primeira linha em formato SRT.");
    assert!((segments[0].start_seconds - 1.5).abs() < 0.001);
    assert!((segments[0].end_seconds - 4.2).abs() < 0.001);

    assert_eq!(segments[1].text, "Segunda linha em formato SRT.");
    assert!((segments[1].start_seconds - 4.2).abs() < 0.001);
    assert!((segments[1].end_seconds - 7.8).abs() < 0.001);
}

#[test]
fn test_merge_progressive_scrolling_captions() {
    let scrolling_vtt = r#"WEBVTT
Kind: captions
Language: pt

00:00:00.080 --> 00:00:01.350
Isso aqui que tá

00:00:01.350 --> 00:00:01.360
Isso aqui que tá

00:00:01.360 --> 00:00:03.310
Isso aqui que tá acontecendo é uma loucura.

00:00:03.310 --> 00:00:06.000
Um único desenvolvedor tá tentando desafiar empresas.
"#;

    let segments = TranscriptParser::parse_vtt(scrolling_vtt);
    assert_eq!(segments.len(), 2);

    assert_eq!(
        segments[0].text,
        "Isso aqui que tá acontecendo é uma loucura."
    );
    assert!((segments[0].start_seconds - 0.08).abs() < 0.001);
    assert!((segments[0].end_seconds - 3.31).abs() < 0.001);

    assert_eq!(
        segments[1].text,
        "Um único desenvolvedor tá tentando desafiar empresas."
    );
    assert!((segments[1].start_seconds - 3.31).abs() < 0.001);
    assert!((segments[1].end_seconds - 6.0).abs() < 0.001);
}

#[test]
fn test_clean_karaoke_and_style_tags() {
    let vtt = r#"WEBVTT

00:00:00.100 --> 00:00:02.500 align:start position:0%
Isso<00:00:00.200><c> aqui</c><00:00:00.320><c> é</c> <c.colorFFFF00>incrível</c>!
"#;

    let segments = TranscriptParser::parse_vtt(vtt);
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].text, "Isso aqui é incrível!");
}

#[test]
fn test_group_multi_line_text_within_same_cue() {
    let vtt = r#"WEBVTT

00:00:02.000 --> 00:00:05.000
Primeira linha da mesma fala.
Segunda linha que complementa a ideia.
"#;

    let segments = TranscriptParser::parse_vtt(vtt);
    assert_eq!(segments.len(), 1);
    assert_eq!(
        segments[0].text,
        "Primeira linha da mesma fala. Segunda linha que complementa a ideia."
    );
}

#[test]
fn test_empty_or_comments_only_vtt() {
    let vtt = r#"WEBVTT

NOTE Comentário do autor
NOTE Outro comentário explicativo

"#;

    let segments = TranscriptParser::parse_vtt(vtt);
    assert!(segments.is_empty());
}
