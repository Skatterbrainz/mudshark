//! Small parsers for common native-command output formats.

/// Parse a line of `KEY="value"` pairs as emitted by tools like `lsblk -P`.
///
/// Quoted values may contain spaces; unquoted values run to the next space.
/// Pairs are returned in order of appearance.
pub fn key_value_pairs(line: &str) -> Vec<(String, String)> {
    let bytes = line.as_bytes();
    let mut pairs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && bytes[i] != b' ' {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            // Malformed token; skip to the next space and continue.
            while i < bytes.len() && bytes[i] != b' ' {
                i += 1;
            }
            continue;
        }
        let key = line[key_start..i].to_string();
        i += 1; // consume '='

        let value = if i < bytes.len() && bytes[i] == b'"' {
            i += 1; // opening quote
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            let v = line[start..i].to_string();
            if i < bytes.len() {
                i += 1; // closing quote
            }
            v
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b' ' {
                i += 1;
            }
            line[start..i].to_string()
        };
        pairs.push((key, value));
    }
    pairs
}
