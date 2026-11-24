//! Mutants of the verified references, used to judge model-written test
//! suites (ADR-006).
//!
//! Each task has a mutation base, `c/mutation-base/<task>.c`: a reference
//! that passes every functional oracle of the task. Every mutant changes
//! exactly one token inside the region between `/* otb-mutate-begin */` and
//! `/* otb-mutate-end */`, using classic first-order operators:
//!
//! - relational: `<` `<=` `>` `>=` swapped with their neighbour, `==` <-> `!=`;
//! - arithmetic and bitwise (binary uses only): `+` <-> `-`, `<<` <-> `>>`,
//!   `&` -> `|`, `|` -> `&`, `^` -> `|`, `&&` <-> `||`;
//! - compound assignment: `+=` <-> `-=`, `|=` <-> `&=`, `<<=` <-> `>>=`;
//! - integer literals: n -> n + 1, and n -> n - 1 when n > 0.
//!
//! Generation is deterministic (source order, then operator order). Which
//! mutants an oracle kills is recorded once in a committed manifest,
//! `c/mutation-base/<task>.mutants.json`; mutants no functional oracle
//! kills (equivalent on the oracle's inputs) are kept in the manifest but
//! do not count when a suite is scored (GAP-MUT-001).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Marks the start of the region mutants are drawn from.
pub const BEGIN: &str = "/* otb-mutate-begin */";
/// Marks its end.
pub const END: &str = "/* otb-mutate-end */";

/// One mutant: a single token replacement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutant {
    /// Stable ID within the task, `M001`, `M002`, ...
    pub id: String,
    /// 1-based line of the replaced token in the base file.
    pub line: u32,
    /// 1-based column (byte) of the replaced token.
    pub col: u32,
    /// The original token.
    pub from: String,
    /// Its replacement.
    pub to: String,
    #[serde(skip)]
    offset: usize,
}

impl Mutant {
    /// The mutant's source: the base with one token replaced.
    #[must_use]
    pub fn apply(&self, base: &str) -> String {
        let mut s = String::with_capacity(base.len() + 4);
        s.push_str(&base[..self.offset]);
        s.push_str(&self.to);
        s.push_str(&base[self.offset + self.from.len()..]);
        s
    }
}

/// SHA-256 of a base file, hex, as recorded in the manifest.
#[must_use]
pub fn base_digest(base: &str) -> String {
    Sha256::digest(base.as_bytes())
        .iter()
        .fold(String::new(), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// Errors in a mutation base.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum MutantError {
    /// The region markers are missing, repeated or out of order.
    #[error("mutation base must contain exactly one {BEGIN} followed by one {END}")]
    Markers,
}

const PUNCT: [&str; 37] = [
    "<<=", ">>=", "...", "->", "++", "--", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "+=",
    "-=", "*=", "/=", "%=", "&=", "|=", "^=", "##", "<", ">", "+", "-", "*", "/", "%", "&", "|",
    "^", "!", "~", "=", "?",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Ident,
    Number,
    Punct,
    Other,
}

struct Token<'a> {
    text: &'a str,
    kind: Kind,
    offset: usize,
}

fn tokenize(src: &str, start: usize, end: usize) -> Vec<Token<'_>> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = start;
    while i < end {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if src[i..].starts_with("//") {
            while i < end && b[i] != b'\n' {
                i += 1;
            }
        } else if src[i..].starts_with("/*") {
            i = src[i + 2..].find("*/").map_or(end, |p| i + 2 + p + 2);
        } else if c == b'"' || c == b'\'' {
            let mut j = i + 1;
            while j < end && b[j] != c {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            out.push(Token {
                text: &src[i..(j + 1).min(end)],
                kind: Kind::Other,
                offset: i,
            });
            i = j + 1;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let j = i + src[i..end]
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .unwrap_or(end - i);
            out.push(Token {
                text: &src[i..j],
                kind: Kind::Ident,
                offset: i,
            });
            i = j;
        } else if c.is_ascii_digit() {
            let j = i + src[i..end]
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'))
                .unwrap_or(end - i);
            out.push(Token {
                text: &src[i..j],
                kind: Kind::Number,
                offset: i,
            });
            i = j;
        } else if let Some(p) = PUNCT.iter().find(|p| src[i..end].starts_with(**p)) {
            out.push(Token {
                text: p,
                kind: Kind::Punct,
                offset: i,
            });
            i += p.len();
        } else {
            out.push(Token {
                text: &src[i..=i],
                kind: Kind::Other,
                offset: i,
            });
            i += 1;
        }
    }
    out
}

/// True if a token ends an operand, so a following `+`, `-` or `&` is a
/// binary operator rather than a sign or an address-of.
fn ends_operand(t: &Token<'_>) -> bool {
    match t.kind {
        Kind::Number => true,
        Kind::Ident => !matches!(t.text, "return" | "case" | "sizeof" | "else" | "do"),
        _ => matches!(t.text, ")" | "]"),
    }
}

/// An integer literal's value and the text to rebuild it with: (value,
/// prefix, digits are uppercase hex, suffix). Floating literals and
/// octal-looking literals other than 0 are not mutated.
fn parse_int(text: &str) -> Option<(u64, bool, bool, &str)> {
    let suffix_at = text.find(['u', 'U', 'l', 'L']).unwrap_or(text.len());
    let (body, suffix) = text.split_at(suffix_at);
    if !suffix.chars().all(|c| matches!(c, 'u' | 'U' | 'l' | 'L')) {
        return None;
    }
    if let Some(hex) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        let v = u64::from_str_radix(hex, 16).ok()?;
        return Some((v, true, hex.chars().any(|c| c.is_ascii_uppercase()), suffix));
    }
    if body.len() > 1 && body.starts_with('0') {
        return None;
    }
    Some((body.parse().ok()?, false, false, suffix))
}

fn format_int(v: u64, hex: bool, upper: bool, suffix: &str) -> String {
    match (hex, upper) {
        (true, true) => format!("0x{v:X}{suffix}"),
        (true, false) => format!("0x{v:x}{suffix}"),
        (false, _) => format!("{v}{suffix}"),
    }
}

/// Generates every mutant of `base`, in deterministic order.
///
/// # Errors
///
/// The region markers are missing or malformed.
pub fn generate(base: &str) -> Result<Vec<Mutant>, MutantError> {
    let (Some(b), Some(e)) = (base.find(BEGIN), base.find(END)) else {
        return Err(MutantError::Markers);
    };
    if e < b || base.matches(BEGIN).count() != 1 || base.matches(END).count() != 1 {
        return Err(MutantError::Markers);
    }
    let toks = tokenize(base, b + BEGIN.len(), e);
    let mut out: Vec<Mutant> = Vec::new();
    for (k, t) in toks.iter().enumerate() {
        let binary = k > 0 && ends_operand(&toks[k - 1]);
        let replacements: Vec<String> = match (t.kind, t.text) {
            (Kind::Punct, "<") => vec!["<=".into()],
            (Kind::Punct, "<=") => vec!["<".into()],
            (Kind::Punct, ">") => vec![">=".into()],
            (Kind::Punct, ">=") => vec![">".into()],
            (Kind::Punct, "==") => vec!["!=".into()],
            (Kind::Punct, "!=") => vec!["==".into()],
            (Kind::Punct, "<<") => vec![">>".into()],
            (Kind::Punct, ">>") => vec!["<<".into()],
            (Kind::Punct, "&&") => vec!["||".into()],
            (Kind::Punct, "||") => vec!["&&".into()],
            (Kind::Punct, "+=") => vec!["-=".into()],
            (Kind::Punct, "-=") => vec!["+=".into()],
            (Kind::Punct, "|=") => vec!["&=".into()],
            (Kind::Punct, "&=") => vec!["|=".into()],
            (Kind::Punct, "<<=") => vec![">>=".into()],
            (Kind::Punct, ">>=") => vec!["<<=".into()],
            (Kind::Punct, "+") if binary => vec!["-".into()],
            (Kind::Punct, "-") if binary => vec!["+".into()],
            (Kind::Punct, "&") if binary => vec!["|".into()],
            (Kind::Punct, "|") => vec!["&".into()],
            (Kind::Punct, "^") => vec!["|".into()],
            (Kind::Number, text) => match parse_int(text) {
                Some((v, hex, upper, suffix)) => {
                    let mut r = vec![format_int(v + 1, hex, upper, suffix)];
                    if v > 0 {
                        r.push(format_int(v - 1, hex, upper, suffix));
                    }
                    r
                }
                None => vec![],
            },
            _ => vec![],
        };
        for to in replacements {
            let line = base[..t.offset].matches('\n').count() + 1;
            let col = t.offset - base[..t.offset].rfind('\n').map_or(0, |p| p + 1) + 1;
            out.push(Mutant {
                id: format!("M{:03}", out.len() + 1),
                line: u32::try_from(line).expect("small"),
                col: u32::try_from(col).expect("small"),
                from: t.text.to_owned(),
                to,
                offset: t.offset,
            });
        }
    }
    Ok(out)
}

/// What the functional oracles concluded about one mutant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Classified {
    /// The mutant.
    pub mutant: Mutant,
    /// True if at least one functional oracle fails the mutant. Only such
    /// mutants count when a suite is scored.
    pub killed_by_oracle: bool,
    /// The first oracle that failed it and its failure class, e.g.
    /// `reference-intermediate wrong-output`.
    pub evidence: Option<String>,
}

/// The committed record of a task's mutant set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Task ID.
    pub task: String,
    /// The base file, relative to the repository root.
    pub base: String,
    /// SHA-256 of the base file (with LF line endings).
    pub base_sha256: String,
    /// Every generated mutant, in generation order.
    pub mutants: Vec<Classified>,
}

impl Manifest {
    /// Path of a task's base file.
    #[must_use]
    pub fn base_path(task: &str) -> String {
        format!("c/mutation-base/{task}.c")
    }

    /// Path of a task's manifest.
    #[must_use]
    pub fn path(task: &str) -> String {
        format!("c/mutation-base/{task}.mutants.json")
    }

    /// The mutants that count when a suite is scored.
    pub fn killable(&self) -> impl Iterator<Item = &Mutant> {
        self.mutants
            .iter()
            .filter(|c| c.killed_by_oracle)
            .map(|c| &c.mutant)
    }

    /// Checks that the manifest describes `base` exactly as `generate`
    /// would, and fills in the token offsets so mutants can be applied.
    ///
    /// # Errors
    ///
    /// A description of the first mismatch.
    pub fn bind(&mut self, base: &str) -> Result<(), String> {
        if base_digest(base) != self.base_sha256 {
            return Err(format!(
                "{}: base file changed since the manifest was made; rerun `otb mutants classify`",
                self.task
            ));
        }
        let fresh = generate(base).map_err(|e| e.to_string())?;
        if fresh.len() != self.mutants.len() {
            return Err(format!(
                "{}: manifest lists {} mutants, generator gives {}",
                self.task,
                self.mutants.len(),
                fresh.len()
            ));
        }
        for (m, f) in self.mutants.iter_mut().zip(fresh) {
            if m.mutant.id != f.id
                || m.mutant.line != f.line
                || m.mutant.col != f.col
                || m.mutant.from != f.from
                || m.mutant.to != f.to
            {
                return Err(format!(
                    "{}: mutant {} differs from the generator",
                    self.task, f.id
                ));
            }
            m.mutant.offset = f.offset;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "#include <stdint.h>\n/* otb-mutate-begin */\n\
uint16_t f(uint16_t x, unsigned d) {\n  uint32_t a = (uint32_t)x << d;\n\
  if (a >= 0x1Fu && -d != 0) return -1;\n  a += &x == 0 ? 3 : 0;\n  return (uint16_t)(a & 7u); /* < not here */\n}\n\
/* otb-mutate-end */\nint g(void) { return 1 < 2; }\n";

    #[test]
    fn generates_first_order_mutants_inside_the_region_only() {
        let m = generate(BASE).unwrap();
        let edits: Vec<(String, String)> =
            m.iter().map(|m| (m.from.clone(), m.to.clone())).collect();
        let want: Vec<(&str, &str)> = vec![
            ("<<", ">>"),
            (">=", ">"),
            ("0x1Fu", "0x20u"),
            ("0x1Fu", "0x1Eu"),
            ("&&", "||"),
            ("!=", "=="),
            ("0", "1"),
            ("1", "2"),
            ("1", "0"),
            ("+=", "-="),
            ("==", "!="),
            ("0", "1"),
            ("3", "4"),
            ("3", "2"),
            ("0", "1"),
            ("&", "|"),
            ("7u", "8u"),
            ("7u", "6u"),
        ];
        let got: Vec<(&str, &str)> = edits
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        assert_eq!(got, want);
        // IDs are sequential, and nothing outside the markers is mutated.
        assert_eq!(m[0].id, "M001");
        assert_eq!(m.last().unwrap().id, format!("M{:03}", m.len()));
        assert!(m.iter().all(|x| x.line >= 3 && x.line <= 7));
    }

    #[test]
    fn unary_minus_and_address_of_are_not_mutated() {
        // `-d` and `-1` follow `&&` and `return`; `&x` follows `+=`.
        let m = generate(BASE).unwrap();
        assert!(!m.iter().any(|x| x.from == "-"));
        assert_eq!(m.iter().filter(|x| x.from == "&").count(), 1);
    }

    #[test]
    fn apply_replaces_exactly_one_token() {
        let m = generate(BASE).unwrap();
        let first = m[0].apply(BASE);
        assert!(first.contains("(uint32_t)x >> d"));
        assert_eq!(first.len(), BASE.len());
        let hex = m[2].apply(BASE);
        assert!(hex.contains("a >= 0x20u"));
    }

    #[test]
    fn markers_are_required() {
        assert_eq!(generate("int x;"), Err(MutantError::Markers));
        let twice = format!("{BEGIN}{END}{BEGIN}{END}");
        assert_eq!(generate(&twice), Err(MutantError::Markers));
        let reversed = format!("{END} int x; {BEGIN}");
        assert_eq!(generate(&reversed), Err(MutantError::Markers));
    }

    #[test]
    fn manifest_binds_only_to_its_own_base() {
        let mutants = generate(BASE)
            .unwrap()
            .into_iter()
            .map(|m| Classified {
                mutant: m,
                killed_by_oracle: true,
                evidence: None,
            })
            .collect();
        let mut man = Manifest {
            task: "T".into(),
            base: "b.c".into(),
            base_sha256: base_digest(BASE),
            mutants,
        };
        let json = serde_json::to_string(&man).unwrap();
        let mut back: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.bind(BASE), Ok(()));
        assert!(back.mutants[0].mutant.apply(BASE).contains(">> d"));
        assert!(man.bind(&BASE.replace("7u", "9u")).is_err());
    }

    #[test]
    fn repository_bases_have_markers_and_mutants() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for task in crate::reference::TASKS {
            let base = std::fs::read_to_string(root.join(Manifest::base_path(task))).unwrap();
            let m = generate(&base).unwrap();
            assert!(m.len() >= 5, "{task}: {} mutants", m.len());
        }
    }
}
