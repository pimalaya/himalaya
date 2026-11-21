//! The documentation ceilings of the guidelines, checked over src/: a `//!`
//! paragraph of four lines at most (inline-001), and a `///` block of one
//! summary plus three lines after a blank (inline-002), clap items aside
//! since their docs are the `--help` (inline-006).

use std::{
    fs,
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

/// Every Rust file under `dir`, in path order.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];

    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }

    files.sort();
    files
}

/// The runs of consecutive lines starting with `prefix`, each as its first
/// line number and its lines with the prefix stripped.
fn blocks<'a>(text: &'a str, prefix: &str) -> Vec<(usize, Vec<&'a str>)> {
    let mut blocks: Vec<(usize, Vec<&str>)> = Vec::new();
    let mut open = false;

    for (number, line) in text.lines().enumerate() {
        let Some(doc) = line.trim_start().strip_prefix(prefix) else {
            open = false;
            continue;
        };

        if !open {
            blocks.push((number + 1, Vec::new()));
            open = true;
        }

        blocks.last_mut().unwrap().1.push(doc.trim());
    }

    blocks
}

/// The paragraphs of a doc block, code fences, lists and tables left out.
fn paragraphs<'a>(lines: &[&'a str]) -> Vec<Vec<&'a str>> {
    let mut paragraphs = vec![Vec::new()];
    let mut fenced = false;

    for line in lines {
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }

        if fenced || line.starts_with("- ") || line.starts_with("| ") {
            continue;
        }

        if line.is_empty() {
            paragraphs.push(Vec::new());
        } else {
            paragraphs.last_mut().unwrap().push(*line);
        }
    }

    paragraphs.retain(|paragraph| !paragraph.is_empty());
    paragraphs
}

/// The line ranges of the items clap derives, from their doc to their
/// closing brace, whose docs clap renders as the `--help`.
fn clap_items(text: &str) -> Vec<RangeInclusive<usize>> {
    const DERIVES: [&str; 4] = ["Parser", "Args", "Subcommand", "ValueEnum"];

    let lines: Vec<&str> = text.lines().collect();
    let mut items = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let line = line.trim_start();
        let derives_clap =
            line.starts_with("#[derive(") && DERIVES.iter().any(|derive| line.contains(derive));

        if !derives_clap {
            continue;
        }

        let mut start = index;
        while start > 0 {
            let above = lines[start - 1].trim_start();
            if above.starts_with("///") || above.starts_with("#[") {
                start -= 1;
            } else {
                break;
            }
        }

        let Some(open) = (index..lines.len()).find(|&i| lines[i].ends_with('{')) else {
            continue;
        };

        let indent = lines[open].len() - lines[open].trim_start().len();
        let close = (open + 1..lines.len())
            .find(|&i| {
                let trimmed = lines[i].trim_start();
                trimmed.starts_with('}') && lines[i].len() - trimmed.len() == indent
            })
            .unwrap_or(lines.len() - 1);

        items.push(start + 1..=close + 1);
    }

    items
}

#[test]
fn a_module_header_paragraph_holds_four_lines_at_most() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut long = Vec::new();

    for path in sources(&root.join("src")) {
        let text = fs::read_to_string(&path).unwrap();
        for (line, block) in blocks(&text, "//!") {
            for paragraph in paragraphs(&block) {
                if paragraph.len() > 4 {
                    long.push(format!("{}:{line}: {}", path.display(), paragraph[0]));
                }
            }
        }
    }

    assert!(
        long.is_empty(),
        "{} paragraphs:\n{}",
        long.len(),
        long.join("\n")
    );
}

#[test]
fn an_item_doc_holds_a_summary_and_three_lines_at_most() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut long = Vec::new();

    for path in sources(&root.join("src")) {
        let text = fs::read_to_string(&path).unwrap();
        let clap = clap_items(&text);

        for (line, block) in blocks(&text, "///") {
            if clap.iter().any(|item| item.contains(&line)) {
                continue;
            }

            let paragraphs = paragraphs(&block);
            let summary = paragraphs.first().map_or(0, Vec::len);
            let rest: usize = paragraphs.iter().skip(1).map(Vec::len).sum();

            if summary > 2 || rest > 3 {
                long.push(format!("{}:{line}: {}", path.display(), block[0]));
            }
        }
    }

    assert!(long.is_empty(), "{} docs:\n{}", long.len(), long.join("\n"));
}
