use std::io::{BufRead, Write};

/// Converts KEY=VALUE lines back into nested YAML, splitting keys on "__"
/// to recover the original nesting. A group of children whose keys are
/// exactly "0", "1", ..., "n-1" (in any order in the input) is written back
/// as a YAML sequence instead of a mapping, which round-trips what
/// yaml_to_env produces for block sequences.
///
/// Unlike yaml_to_env::convert, this can't emit output as it reads: two
/// lines that belong under the same parent key might be separated by
/// unrelated lines (a hand-edited or resorted env file, or one merged from
/// multiple sources), and there's no way to know a group is "done" until
/// the input ends. So this builds a tree of the key groups seen so far -
/// sized to the number of distinct keys, not the byte size of the input -
/// and writes the whole thing out once at EOF. That's a real departure
/// from the bounded-memory streaming the yaml-to-env direction gives you;
/// see the README.
pub fn convert<R: BufRead, W: Write>(input: R, mut output: W) -> std::io::Result<()> {
    let mut root: Vec<(String, Node)> = Vec::new();

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
        insert(&mut root, &path, value.to_string());
    }

    write_node(&root, 0, &mut output)
}

enum Node {
    Leaf(String),
    Branch(Vec<(String, Node)>),
}

/// Inserts `value` at `path` into `children`, in place, creating any
/// missing branches along the way. A key that was previously a leaf and is
/// now addressed with a longer path (or vice versa) is overwritten, the
/// same way a later assignment to the same env var would win.
fn insert(children: &mut Vec<(String, Node)>, path: &[String], value: String) {
    let (head, rest) = match path.split_first() {
        Some(pair) => pair,
        None => return,
    };

    if rest.is_empty() {
        match children.iter_mut().find(|(k, _)| k == head) {
            Some((_, node)) => *node = Node::Leaf(value),
            None => children.push((head.clone(), Node::Leaf(value))),
        }
        return;
    }

    match children.iter_mut().find(|(k, _)| k == head) {
        Some((_, Node::Branch(sub))) => insert(sub, rest, value),
        Some((_, node)) => {
            let mut sub = Vec::new();
            insert(&mut sub, rest, value);
            *node = Node::Branch(sub);
        }
        None => {
            let mut sub = Vec::new();
            insert(&mut sub, rest, value);
            children.push((head.clone(), Node::Branch(sub)));
        }
    }
}

/// True when `children`'s keys are exactly the decimal strings "0".."n-1",
/// in any order - i.e. this group came from a sequence, not a mapping that
/// happens to use small integers as keys.
fn is_sequence(children: &[(String, Node)]) -> bool {
    if children.is_empty() {
        return false;
    }
    let mut seen = vec![false; children.len()];
    for (key, _) in children {
        match key.parse::<usize>() {
            Ok(n) if n < children.len() && *key == n.to_string() => {
                if seen[n] {
                    return false;
                }
                seen[n] = true;
            }
            _ => return false,
        }
    }
    seen.into_iter().all(|s| s)
}

fn write_node<W: Write>(children: &[(String, Node)], depth: usize, output: &mut W) -> std::io::Result<()> {
    if is_sequence(children) {
        return write_sequence(children, depth, output);
    }

    let indent = "  ".repeat(depth);
    for (key, node) in children {
        match node {
            Node::Leaf(value) => writeln!(output, "{}{}: {}", indent, key, value)?,
            Node::Branch(sub) => {
                writeln!(output, "{}{}:", indent, key)?;
                write_node(sub, depth + 1, output)?;
            }
        }
    }
    Ok(())
}

fn write_sequence<W: Write>(children: &[(String, Node)], depth: usize, output: &mut W) -> std::io::Result<()> {
    let indent = "  ".repeat(depth);
    for i in 0..children.len() {
        let key = i.to_string();
        let (_, node) = children.iter().find(|(k, _)| *k == key).expect("is_sequence verified every index is present");
        match node {
            Node::Leaf(value) => writeln!(output, "{}- {}", indent, value)?,
            Node::Branch(sub) => write_sequence_item(sub, depth, output)?,
        }
    }
    Ok(())
}

/// Writes a mapping as one sequence item: the first key rides on the "- "
/// line, and the rest line up underneath it at the same column, e.g.
///
/// ```text
/// - name: x
///   port: 1
/// ```
///
/// This is rendered by writing the item's mapping as if it were one level
/// deeper (so its indent already lines up past "- "), then splicing the
/// leading indent of the first line into a dash.
fn write_sequence_item<W: Write>(sub: &[(String, Node)], depth: usize, output: &mut W) -> std::io::Result<()> {
    let mut rendered = Vec::new();
    write_node(sub, depth + 1, &mut rendered)?;
    let rendered = String::from_utf8(rendered)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let child_indent = "  ".repeat(depth + 1);
    let mut lines = rendered.lines();
    if let Some(first) = lines.next() {
        let stripped = first.strip_prefix(child_indent.as_str()).unwrap_or(first);
        writeln!(output, "{}- {}", "  ".repeat(depth), stripped)?;
    }
    for line in lines {
        writeln!(output, "{}", line)?;
    }
    Ok(())
}
