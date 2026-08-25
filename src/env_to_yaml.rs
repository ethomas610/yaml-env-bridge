use std::io::{BufRead, Write};

/// Converts KEY=VALUE lines back into a nested YAML mapping, splitting keys
/// on "__" to recover the original nesting.
///
/// This only keeps the previously written key path in memory (bounded by
/// nesting depth), not the input as a whole, so it streams the same way
/// convert() in yaml_to_env.rs does. The tradeoff: it assumes keys sharing
/// a prefix arrive as consecutive lines, the way `yaml-env-bridge --to env`
/// produces them. Interleaved groups will emit the same mapping key twice
/// instead of being merged; see the README for why that's a real limit
/// and not just a missing check.
pub fn convert<R: BufRead, W: Write>(input: R, mut output: W) -> std::io::Result<()> {
    let mut previous_path: Vec<String> = Vec::new();

    for line in input.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let (key, value) = match trimmed.split_once('=') {
            Some(pair) => pair,
            None => continue,
        };

        let path: Vec<String> = key.split("__").map(|s| s.to_lowercase()).collect();
        let shared = path
            .iter()
            .zip(previous_path.iter())
            .take_while(|(a, b)| a == b)
            .count();

        for (depth, segment) in path.iter().enumerate() {
            if depth < shared {
                continue;
            }
            let indent = "  ".repeat(depth);
            if depth + 1 == path.len() {
                writeln!(output, "{}{}: {}", indent, segment, value)?;
            } else {
                writeln!(output, "{}{}:", indent, segment)?;
            }
        }

        previous_path = path;
    }

    Ok(())
}
