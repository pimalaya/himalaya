//! # Documentation rules
//!
//! The ceilings of the guidelines over src/: a `//!` paragraph of four
//! lines (inline-001), a `///` block of a summary plus three lines
//! (inline-002), clap items aside (inline-006). Then the README's budgets
//! (readme-007, readme-008) and its header block, run for real.

#[cfg(feature = "maildir")]
use std::process::{Command, Stdio};
use std::{
    fs,
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

#[cfg(feature = "maildir")]
use serde_json::Value;
#[cfg(feature = "maildir")]
use tempfile::TempDir;

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

/// The README, read from the repository root.
fn readme() -> String {
    fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")).unwrap()
}

/// The README's `## ` sections, in order.
fn readme_sections(readme: &str) -> Vec<&str> {
    readme
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect()
}

#[test]
fn the_readme_features_are_eight_short_bullets_at_most() {
    let readme = readme();
    let features: Vec<&str> = readme
        .split("\n## Features\n")
        .nth(1)
        .expect("a Features section")
        .split("\n## ")
        .next()
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("- "))
        .collect();

    assert!(!features.is_empty() && features.len() <= 8, "{features:#?}");
    for feature in features {
        assert!(feature.chars().count() <= 120, "{feature}");
    }
}

#[test]
fn the_readme_sections_follow_the_cli_order() {
    assert_eq!(
        readme_sections(&readme()),
        [
            "Table of contents",
            "Features",
            "Installation",
            "Configuration",
            "Usage",
            "License",
            "Social",
            "Sponsoring",
        ]
    );
}

/// The two messages from Alice the README's header block finds, and one
/// from Bob it leaves out, as Maildir entries by file name.
#[cfg(feature = "maildir")]
const README_MAILDIR: [(&str, &str); 3] = [
    (
        "1:2,S",
        "From: Alice <alice@example.org>\r\n\
        To: me@example.org\r\n\
        Subject: Lunch on Friday?\r\n\
        Message-ID: <lunch@example.org>\r\n\
        Date: Wed, 18 Nov 2026 12:30:00 +0100\r\n\
        \r\n\
        Pizza?\r\n",
    ),
    (
        "2:2,",
        "From: Alice <alice@example.org>\r\n\
        To: me@example.org\r\n\
        Subject: Slides for the review\r\n\
        Message-ID: <slides@example.org>\r\n\
        Date: Thu, 19 Nov 2026 09:15:00 +0100\r\n\
        Content-Type: multipart/mixed; boundary=\"m\"\r\n\
        \r\n\
        --m\r\n\
        Content-Type: text/plain\r\n\
        \r\n\
        Attached.\r\n\
        --m\r\n\
        Content-Type: application/pdf\r\n\
        Content-Disposition: attachment; filename=\"slides.pdf\"\r\n\
        \r\n\
        PDF\r\n\
        --m--\r\n",
    ),
    (
        "3:2,S",
        "From: Bob <bob@example.org>\r\n\
        To: me@example.org\r\n\
        Subject: Weekly report\r\n\
        Message-ID: <report@example.org>\r\n\
        Date: Fri, 20 Nov 2026 08:00:00 +0100\r\n\
        \r\n\
        Numbers.\r\n",
    ),
];

/// The content of the first fenced block of `lang` in `text`, and the
/// rest of `text` after it.
#[cfg(feature = "maildir")]
fn fenced<'a>(text: &'a str, lang: &str) -> (&'a str, &'a str) {
    let open = format!("```{lang}\n");
    let start = text.find(&open).expect("a fenced block") + open.len();
    let end = start + text[start..].find("\n```").expect("a closing fence");
    (&text[start..end], &text[end + 4..])
}

#[cfg(feature = "maildir")]
#[test]
fn the_readme_header_block_prints_what_it_shows() {
    let readme = readme();
    let (command, rest) = fenced(&readme, "sh");
    let (shown, _) = fenced(rest, "json");
    let args: Vec<&str> = command
        .strip_prefix("$ himalaya ")
        .expect("a himalaya command line")
        .split_whitespace()
        .collect();

    let dir = TempDir::new().unwrap();
    let inbox = dir.path().join("root").join("inbox");
    for sub in ["cur", "new", "tmp"] {
        fs::create_dir_all(inbox.join(sub)).unwrap();
    }
    for (name, message) in README_MAILDIR {
        fs::write(inbox.join("cur").join(name), message).unwrap();
    }

    let config = dir.path().join("config.toml");
    let root = dir.path().join("root");
    let toml = format!(
        "[accounts.me]\ndefault = true\nemail = \"me@example.org\"\nmaildir.root = \"{}\"\n",
        root.display(),
    );
    fs::write(&config, toml).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .arg("-c")
        .arg(&config)
        .args(&args)
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env("TZ", "UTC")
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let printed: Value = serde_json::from_slice(&output.stdout).unwrap();
    let shown: Value = serde_json::from_str(shown).unwrap();
    assert_eq!(printed, shown);
}
