use std::io::{BufRead, Write};

/// Converts KEY=VALUE lines back into a nested YAML mapping, splitting keys
/// on "__" to recover the original nesting.
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

fn write_node<W: Write>(children: &[(String, Node)], depth: usize, output: &mut W) -> std::io::Result<()> {
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
