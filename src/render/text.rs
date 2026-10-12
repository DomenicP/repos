use std::path::Path;

// Unicode names for characters that can break lines or reorder displayed text.
const LINE_SEPARATOR: char = '\u{2028}';
const PARAGRAPH_SEPARATOR: char = '\u{2029}';
const LEFT_TO_RIGHT_EMBEDDING: char = '\u{202a}';
const RIGHT_TO_LEFT_EMBEDDING: char = '\u{202b}';
const POP_DIRECTIONAL_FORMATTING: char = '\u{202c}';
const LEFT_TO_RIGHT_OVERRIDE: char = '\u{202d}';
const RIGHT_TO_LEFT_OVERRIDE: char = '\u{202e}';
const LEFT_TO_RIGHT_ISOLATE: char = '\u{2066}';
const RIGHT_TO_LEFT_ISOLATE: char = '\u{2067}';
const FIRST_STRONG_ISOLATE: char = '\u{2068}';
const POP_DIRECTIONAL_ISOLATE: char = '\u{2069}';

fn disrupts_display(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            LINE_SEPARATOR
                | PARAGRAPH_SEPARATOR
                | LEFT_TO_RIGHT_EMBEDDING
                | RIGHT_TO_LEFT_EMBEDDING
                | POP_DIRECTIONAL_FORMATTING
                | LEFT_TO_RIGHT_OVERRIDE
                | RIGHT_TO_LEFT_OVERRIDE
                | LEFT_TO_RIGHT_ISOLATE
                | RIGHT_TO_LEFT_ISOLATE
                | FIRST_STRONG_ISOLATE
                | POP_DIRECTIONAL_ISOLATE
        )
}

pub(crate) fn escape(text: &str) -> String {
    if text.chars().any(disrupts_display) {
        format!("{text:?}")
    } else {
        text.to_owned()
    }
}

// Keep invalid filename bytes distinguishable instead of replacing them with
// U+FFFD. Unix filenames need not be UTF-8.
pub(crate) fn path_text(path: &Path) -> String {
    let mut bytes = path.as_os_str().as_encoded_bytes();
    if let Ok(text) = std::str::from_utf8(bytes) {
        // Quote literal escape syntax too, so it cannot be confused with a
        // filename containing the invalid bytes escaped below.
        if text.contains('"') || text.contains('\\') {
            return format!("{text:?}");
        }
        return escape(text);
    }
    let mut output = String::from("\"");
    while !bytes.is_empty() {
        match std::str::from_utf8(bytes) {
            Ok(text) => {
                for c in text.chars() {
                    output.extend(c.escape_debug());
                }
                break;
            }
            Err(error) => {
                let valid = error.valid_up_to();
                let text = std::str::from_utf8(&bytes[..valid]).expect("valid UTF-8 prefix");
                for c in text.chars() {
                    output.extend(c.escape_debug());
                }
                let end = valid + error.error_len().unwrap_or(bytes.len() - valid);
                for byte in &bytes[valid..end] {
                    output.push_str(&format!("\\x{byte:02x}"));
                }
                bytes = &bytes[end..];
            }
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_characters_cannot_break_rows() {
        assert_eq!(escape("a\n\x1bb"), "\"a\\n\\u{1b}b\"");
        assert_eq!(escape("a\u{202e}b"), "\"a\\u{202e}b\"");
    }

    #[cfg(unix)]
    #[test]
    fn invalid_filename_bytes_are_preserved() {
        use std::os::unix::ffi::OsStrExt;
        let path = Path::new(std::ffi::OsStr::from_bytes(b"repo\xff\n"));
        assert_eq!(path_text(path), "\"repo\\xff\\n\"");
        let invalid = Path::new(std::ffi::OsStr::from_bytes(b"repo\xff"));
        let literal = Path::new("\"repo\\xff\"");
        assert_ne!(path_text(invalid), path_text(literal));
    }
}
