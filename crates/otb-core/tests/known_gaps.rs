//! Every gap ID cited anywhere in the repository must be defined in
//! `KNOWN_GAPS.md`, and each ID must be defined exactly once.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Directories scanned for gap citations, relative to the workspace root.
const SCANNED: &[&str] = &[
    "crates",
    "c",
    "tasks",
    "protocol",
    "docs",
    "analysis",
    "README.md",
];

/// File extensions scanned.
const EXTENSIONS: &[&str] = &["rs", "c", "h", "md", "toml", "py", "yml", "yaml"];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Finds every `GAP-<UPPER>-<3 digits>` token in `text`.
fn gap_ids(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while let Some(off) = text[i..].find("GAP-") {
        let start = i + off;
        let mut j = start + 4;
        let letters_start = j;
        while j < bytes.len() && bytes[j].is_ascii_uppercase() {
            j += 1;
        }
        let ok_letters = j > letters_start;
        let ok_dash = j < bytes.len() && bytes[j] == b'-';
        let digits_start = j + 1;
        let digits_end = digits_start + 3;
        let ok_digits = ok_letters
            && ok_dash
            && digits_end <= bytes.len()
            && bytes[digits_start..digits_end]
                .iter()
                .all(u8::is_ascii_digit)
            && bytes.get(digits_end).is_none_or(|b| !b.is_ascii_digit());
        if ok_digits {
            found.push(text[start..digits_end].to_owned());
        }
        i = start + 4;
    }
    found
}

fn walk(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        out.push(path.to_owned());
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.file_name().is_some_and(|n| n == "target") {
            continue;
        }
        walk(&p, out);
    }
}

#[test]
fn gap_ids_parser() {
    assert_eq!(
        gap_ids("see GAP-CT-001, GAP-ACVP-001; not GAP-ct-001, GAP-CT-01, GAP-CT-0011"),
        vec!["GAP-CT-001", "GAP-ACVP-001"]
    );
}

#[test]
fn every_cited_gap_is_defined_once() {
    let root = workspace_root();
    let ledger = fs::read_to_string(root.join("KNOWN_GAPS.md")).expect("KNOWN_GAPS.md");

    let mut defined: BTreeMap<String, usize> = BTreeMap::new();
    for line in ledger.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            let ids = gap_ids(rest);
            assert_eq!(ids.len(), 1, "heading must define exactly one gap: {line}");
            assert!(
                rest.starts_with(&ids[0]),
                "heading must start with its ID: {line}"
            );
            *defined.entry(ids[0].clone()).or_default() += 1;
        }
    }
    assert!(!defined.is_empty(), "KNOWN_GAPS.md defines no gaps");
    for (id, n) in &defined {
        assert_eq!(*n, 1, "{id} is defined {n} times");
    }

    let mut files = Vec::new();
    for rel in SCANNED {
        walk(&root.join(rel), &mut files);
    }
    let mut missing = Vec::new();
    for f in files {
        let ext_ok = f
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONS.contains(&e));
        if !ext_ok {
            continue;
        }
        let Ok(text) = fs::read_to_string(&f) else {
            continue;
        };
        for id in gap_ids(&text) {
            if !defined.contains_key(&id) {
                missing.push(format!("{} cites undefined {id}", f.display()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "undefined gap IDs:\n{}",
        missing.join("\n")
    );
}
