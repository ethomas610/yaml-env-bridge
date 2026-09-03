use std::io::{BufRead, Write};

/// Converts a stream of nested YAML mappings (and the sequences nested
/// inside them) into KEY=VALUE lines, one per scalar leaf. Only the subset
/// of YAML actually needed for flat config files is supported: block-style
/// mappings and block-style sequences of scalars or mappings, arbitrarily
/// nested. Flow collections (`{a: 1}`, `[1, 2]`), anchors/aliases, and
/// multi-line block scalars are not handled.
///
/// Sequence items are flattened using their index as the path segment, so
///
/// ```text
/// tags:
///   - a
///   - b
/// ```
///
/// becomes `TAGS__0=a` / `TAGS__1=b`, and a sequence of mappings nests the
/// index the same way a map key would: `NESTED__0__NAME=...`.
///
/// Reads and writes line by line so the whole document is never held in
/// memory at once. The only state kept between lines is a stack of open
/// parent segments, bounded by nesting depth rather than input size.
pub fn convert<R: BufRead, W: Write>(input: R, mut output: W) -> std::io::Result<()> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut root_seq_next: usize = 0;

    for line in input.lines() {
        let line = line?;
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() || trimmed.trim_start().starts_with('#') {
            continue;
        }

        let indent = trimmed.len() - trimmed.trim_start().len();
        let content = trimmed.trim_start();

        if let Some(after_dash) = content.strip_prefix('-') {
            if after_dash.is_empty() || after_dash.starts_with(char::is_whitespace) {
                handle_seq_item(&mut stack, &mut root_seq_next, indent, after_dash, &mut output)?;
                continue;
            }
        }

        // close out any mappings we've dedented past or moved sideways from
        while let Some(top) = stack.last() {
            if top.indent >= indent {
                stack.pop();
            } else {
                break;
            }
        }

        let (key, value) = split_key_value(content);
        match value {
            None => stack.push(Frame { indent, name: key.to_string(), seq_next: None }),
            Some(v) => {
                let var_name = env_var_name(&stack, key);
                writeln!(output, "{}={}", var_name, v)?;
            }
        }
    }

    Ok(())
}

struct Frame {
    indent: usize,
    name: String,
    // Some(n) once this frame has been established as a sequence, where n
    // is the index the next "- " item under it should take.
    seq_next: Option<usize>,
}

fn handle_seq_item<W: Write>(
    stack: &mut Vec<Frame>,
    root_seq_next: &mut usize,
    dash_indent: usize,
    after_dash: &str,
    output: &mut W,
) -> std::io::Result<()> {
    // close any item (or deeper mapping) that was open at this indent or past it
    while let Some(top) = stack.last() {
        if top.indent >= dash_indent {
            stack.pop();
        } else {
            break;
        }
    }

    let idx = match stack.last_mut() {
        Some(parent) => {
            let idx = parent.seq_next.unwrap_or(0);
            parent.seq_next = Some(idx + 1);
            idx
        }
        None => {
            let idx = *root_seq_next;
            *root_seq_next += 1;
            idx
        }
    };

    let inner = after_dash.trim_start();
    let inner_offset = after_dash.len() - inner.len();
    let content_start = dash_indent + 1 + inner_offset;

    if inner.is_empty() {
        // item's content is entirely on following, deeper-indented lines
        stack.push(Frame { indent: dash_indent, name: idx.to_string(), seq_next: None });
        return Ok(());
    }

    if seq_item_is_mapping(inner) {
        stack.push(Frame { indent: dash_indent, name: idx.to_string(), seq_next: None });
        let (key, value) = split_key_value(inner);
        match value {
            None => {
                stack.push(Frame { indent: content_start, name: key.to_string(), seq_next: None })
            }
            Some(v) => {
                let var_name = env_var_name(stack, key);
                writeln!(output, "{}={}", var_name, v)?;
            }
        }
    } else {
        let value = strip_comment(inner);
        let var_name = env_var_name(stack, &idx.to_string());
        writeln!(output, "{}={}", var_name, value)?;
    }

    Ok(())
}

/// A block sequence item is a nested mapping (rather than a plain scalar)
/// when its inline content has the shape `key: value` or `key:`. This is
/// the same "colon followed by space or end of line" rule YAML itself uses
/// to distinguish a mapping indicator from a colon that's just part of a
/// scalar (e.g. `- http://example.com` stays a scalar).
fn seq_item_is_mapping(inner: &str) -> bool {
    inner.ends_with(':') || inner.contains(": ")
}

fn env_var_name(stack: &[Frame], leaf: &str) -> String {
    let mut name = String::new();
    for frame in stack {
        name.push_str(&frame.name);
        name.push_str("__");
    }
    name.push_str(leaf);

    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c.to_ascii_uppercase() } else { '_' })
        .collect()
}

fn split_key_value(content: &str) -> (&str, Option<&str>) {
    match content.find(':') {
        Some(idx) => {
            let key = content[..idx].trim();
            let rest = strip_comment(content[idx + 1..].trim());
            if rest.is_empty() {
                (key, None)
            } else {
                (key, Some(rest))
            }
        }
        // a line with no colon at all isn't valid in the mappings we support
        None => (content.trim(), None),
    }
}

fn strip_comment(value: &str) -> &str {
    // don't strip inside quoted scalars; this is a shortcut, not a full
    // quote-aware scanner, so a '#' after an escaped quote can still fool it
    if value.starts_with('"') || value.starts_with('\'') {
        return value;
    }
    match value.find(" #") {
        Some(idx) => value[..idx].trim_end(),
        None => value,
    }
}
