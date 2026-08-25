use std::io::{BufRead, Write};

/// Converts a stream of nested YAML mappings into KEY=VALUE lines, one per
/// scalar leaf. Only the subset of YAML actually needed for flat config
/// files is supported: block-style mappings of scalars, arbitrarily nested.
/// Sequences, flow collections, anchors, and multi-line block scalars are
/// not handled.
///
/// Reads and writes line by line so the whole document is never held in
/// memory at once. The only state kept between lines is a stack of open
/// parent keys, bounded by nesting depth rather than input size.
pub fn convert<R: BufRead, W: Write>(input: R, mut output: W) -> std::io::Result<()> {
    // (indent, key) for each mapping currently open above the line being read
    let mut stack: Vec<(usize, String)> = Vec::new();

    for line in input.lines() {
        let line = line?;
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() || trimmed.trim_start().starts_with('#') {
            continue;
        }

        let indent = trimmed.len() - trimmed.trim_start().len();
        let content = trimmed.trim_start();
        let (key, value) = split_key_value(content);

        // close out any mappings we've dedented past or moved sideways from
        while let Some(&(top_indent, _)) = stack.last() {
            if top_indent >= indent {
                stack.pop();
            } else {
                break;
            }
        }

        match value {
            None => stack.push((indent, key.to_string())),
            Some(v) => {
                let var_name = env_var_name(&stack, key);
                writeln!(output, "{}={}", var_name, v)?;
            }
        }
    }

    Ok(())
}

fn env_var_name(stack: &[(usize, String)], leaf: &str) -> String {
    let mut name = String::new();
    for (_, segment) in stack {
        name.push_str(segment);
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
