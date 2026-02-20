// ── Strip XML-like tags from model output ────────────────────────────────────

/// Remove XML-like tags (e.g. `<invoke>`, `</answer>`, `<parameter name="x">`) from model responses.
pub fn strip_model_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            let start = i;
            let next = if i + 1 < chars.len() { chars[i + 1] } else { '\0' };
            // Only treat as a tag if next char is a letter or '/' (closing tag)
            if next.is_ascii_alphabetic() || next == '/' {
                let mut j = i + 1;
                while j < chars.len() && chars[j] != '>' && chars[j] != '<' {
                    j += 1;
                }
                if j < chars.len() && chars[j] == '>' {
                    // Valid tag — skip it
                    i = j + 1;
                } else {
                    // Not a valid tag, emit '<' literally
                    out.push(chars[start]);
                    i = start + 1;
                }
            } else {
                // Not a tag (e.g. `x < y`), emit literally
                out.push(chars[i]);
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    // Collapse runs of blank lines (max 1 consecutive blank line)
    let mut result = String::new();
    let mut blank_count = 0u32;
    for line in out.lines() {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(line);
            result.push('\n');
        }
    }
    result.trim_end().to_string()
}

