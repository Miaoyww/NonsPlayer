# TTML Processor

A high-performance Rust library for parsing and generating TTML files, specifically tailored for the Apple Music and [AMLL](<https://github.com/amll-dev/applemusic-like-lyrics>) formats.

## A Note on Specialization

This library is **not** a general-purpose TTML subtitle parser. It is specifically designed to handle the unique conventions, metadata structures, and extensions (e.g., `itunes:*` attributes, `<iTunesMetadata>`) found in TTML files used by Apple Music. Attempting to use it on generic TTML subtitle files may result in errors or incomplete data.

## Usage

Add `ttml_processor` to your `Cargo.toml`:
```toml
[dependencies]
ttml_processor = "0.4.0" # Replace with the latest version
```

### Parsing Example

```rust
use ttml_processor::parse_ttml;

fn main() {
    let ttml_content = r#"
    <tt xmlns="http://www.w3.org/ns/ttml" xmlns:itunes="http://music.apple.com/lyric-ttml-internal" itunes:timing="word">
      <body>
        <div>
          <p begin="5.000" end="10.000">
            <span begin="5.100" end="5.500">Hello </span>
            <span begin="5.600" end="6.000">world</span>
          </p>
        </div>
      </body>
    </tt>
    "#;

    let parsed_data = parse_ttml(ttml_content).expect("Failed to parse TTML");

    assert_eq!(parsed_data.lines.len(), 1);
    let first_line = &parsed_data.lines[0];
    assert_eq!(first_line.start_time, 5000);

    let syllables = &first_line
        .words
        .as_ref()
        .expect("Should have syllables in first line");
    assert_eq!(syllables.len(), 2);
    assert_eq!(syllables[0].text, "Hello");
    assert_eq!(syllables[0].start_time, 5100);
    assert_eq!(syllables[0].ends_with_space, Some(true));

    println!("Successfully parsed TTML: {parsed_data:?}");
}
```

### Generation Example

```rust
use ttml_processor::{
    GeneratorConfig,
    generate_ttml,
    model::{
        LyricLine,
        Syllable,
        TTMLMetadata,
        TTMLResult,
    },
};

fn main() {
    let mut line = LyricLine {
        start_time: 5000,
        end_time: 10000,
        ..Default::default()
    };
    line.push_word(Syllable {
        text: "Hello".into(),
        start_time: 5100,
        end_time: 5500,
        ends_with_space: Some(true),
        ..Default::default()
    });
    line.push_word(Syllable {
        text: "world".into(),
        start_time: 5600,
        end_time: 6000,
        ..Default::default()
    });
    line.rebuild_text();

    let result = TTMLResult {
        metadata: TTMLMetadata {
            timing_mode: Some("word".into()),
            language: Some("en".into()),
            ..Default::default()
        },
        lines: vec![line],
    };

    let config = GeneratorConfig {
        use_apple_format_rules: false,
        format: true,
        line_timing: false,
    };

    let ttml_string = generate_ttml(&result, &config).expect("Failed to generate TTML");
    println!("{ttml_string}");
}
```

### Generator Options

`GeneratorConfig` controls the shape of the generated TTML:

- `use_apple_format_rules` — write per-line translations/transliterations into `<head>` and
  follow Apple Music's background-vocal conventions. When `false`, per-line
  translations/transliterations are emitted as inline `x-translation` / `x-roman` spans instead.
  Word-by-word translations/transliterations always go into `<head>`.
- `format` — pretty-print the XML instead of emitting it as a single line.
- `line_timing` — generate line-by-line lyrics: `itunes:timing="Line"`, plain text inside each
  `<p>` with no per-syllable `<span>`, word-by-word translations/transliterations flattened to
  line text (their placement still follows `use_apple_format_rules`), and background vocals
  omitted.

> [!NOTE]
> `line_timing` is the only way to produce line-by-line TTML. Setting the `timing_mode` metadata
> to `"Line"` merely writes that value into the `itunes:timing` attribute, the per-syllable
> `<span>`s are still emitted. Callers usually decide whether to enable `line_timing` either by
> checking that every main lyric line has at most one syllable, or — when the input comes from a
> trustworthy source — by reading the parsed `TTMLMetadata.timing_mode` or another
> high-confidence timing-mode signal.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
