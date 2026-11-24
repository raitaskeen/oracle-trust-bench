//! Task specifications: TOML front matter followed by a Markdown body.
//!
//! ```text
//! +++
//! id = "MLKEM-T01-ntt"
//! ...
//! +++
//! Markdown body (notes, and later the prompt summary)
//! ```
//!
//! Oracle applicability is part of the spec and is therefore static: every
//! oracle kind must be listed, either with the method used or with the reason
//! it does not apply (ADR-004).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::fips::FipsDoc;
use crate::{OracleKind, TaskId};

/// A parsed task specification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    /// Stable task ID; must match the directory the spec lives in.
    pub id: TaskId,
    /// Short human-readable title.
    pub title: String,
    /// Whether the task is a component or a top-level FIPS function.
    pub level: Level,
    /// The parameter set the task is fixed to.
    pub parameter_set: String,
    /// What in the standard the task implements.
    pub fips: FipsRef,
    /// The fixed C interface the candidate must implement.
    pub c: CInterface,
    /// The input domain, in words.
    pub input_domain: String,
    /// Secret inputs and public outputs, for constant-time checks.
    pub secrets: Secrets,
    /// Applicability of every oracle kind.
    pub oracles: BTreeMap<OracleKind, Applicability>,
    /// The Markdown body after the front matter.
    #[serde(skip)]
    pub body: String,
}

/// Component or top-level function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    /// An internal building block (NTT, compression, rounding, hints).
    Component,
    /// A top-level function that ACVP can test directly.
    TopLevel,
}

/// A citation of the standard.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FipsRef {
    /// The document.
    pub document: FipsDoc,
    /// Cited algorithms by number and name.
    #[serde(default)]
    pub algorithms: Vec<AlgorithmRef>,
    /// Cited equation numbers, e.g. `4.7`.
    #[serde(default)]
    pub equations: Vec<String>,
    /// Cited section numbers, e.g. `7.3`.
    #[serde(default)]
    pub sections: Vec<String>,
}

/// One cited algorithm.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlgorithmRef {
    /// Algorithm number in the document.
    pub number: u32,
    /// Algorithm name exactly as in `fips.rs`.
    pub name: String,
}

/// The C interface of a task.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CInterface {
    /// The function name.
    pub function: String,
    /// The full C prototype, ending with `;`.
    pub signature: String,
    /// Return-value contract, if the function returns a value.
    #[serde(default)]
    pub returns: Option<String>,
}

/// Secret data, for the constant-time oracles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Secrets {
    /// Parameters whose values are secret, with the reason.
    pub inputs: Vec<String>,
    /// Outputs that are public and may be declassified after the call.
    #[serde(default)]
    pub public_outputs: Vec<String>,
}

/// Whether one oracle applies, and how or why not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Applicability {
    /// True if the oracle applies to this task.
    pub applies: bool,
    /// How the oracle is applied. Required when `applies` is true.
    #[serde(default)]
    pub method: Option<String>,
    /// Why the oracle does not apply. Required when `applies` is false.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Errors from loading or validating a spec.
#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    /// Reading the file failed.
    #[error("{path}: {source}")]
    Io {
        /// The file.
        path: String,
        /// The I/O error.
        source: std::io::Error,
    },
    /// The file does not start with a `+++` front matter block.
    #[error("missing TOML front matter delimited by +++ lines")]
    NoFrontMatter,
    /// The front matter is not valid TOML for a spec.
    #[error("front matter: {0}")]
    Toml(#[from] toml::de::Error),
    /// The spec parsed but breaks a rule.
    #[error("{0}")]
    Invalid(String),
}

impl TaskSpec {
    /// Parses a spec from its text.
    pub fn parse(text: &str) -> Result<Self, SpecError> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let rest = text
            .strip_prefix("+++\n")
            .or_else(|| text.strip_prefix("+++\r\n"))
            .ok_or(SpecError::NoFrontMatter)?;
        let end = rest.find("\n+++").ok_or(SpecError::NoFrontMatter)?;
        let front = &rest[..=end];
        let after = &rest[end + 4..];
        let body = after
            .strip_prefix("\r\n")
            .or_else(|| after.strip_prefix('\n'))
            .unwrap_or(after);
        let mut spec: TaskSpec = toml::from_str(front)?;
        body.clone_into(&mut spec.body);
        Ok(spec)
    }

    /// Loads a spec and checks that its ID matches its location
    /// `<root>/<mlkem|mldsa>/T<NN>-<slug>/spec.md`.
    pub fn load(path: &Path) -> Result<Self, SpecError> {
        let text = std::fs::read_to_string(path).map_err(|source| SpecError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let spec = Self::parse(&text)?;
        let dir = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|s| s.to_str());
        let family = path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .and_then(|s| s.to_str());
        let expected = match (family, dir) {
            (Some(f), Some(d)) => format!("{}-{}", f.to_ascii_uppercase(), d),
            _ => String::new(),
        };
        if spec.id.as_str() != expected {
            return Err(SpecError::Invalid(format!(
                "id {} does not match its location (expected {expected})",
                spec.id
            )));
        }
        Ok(spec)
    }

    /// Checks every rule a spec must satisfy. Returns all problems found.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut p = Vec::new();
        let id = self.id.as_str();
        let is_kem = id.starts_with("MLKEM-");

        let want_doc = if is_kem {
            FipsDoc::Fips203
        } else {
            FipsDoc::Fips204
        };
        if self.fips.document != want_doc {
            p.push(format!("fips.document must be {}", want_doc.name()));
        }
        let want_prefix = if is_kem { "ML-KEM-" } else { "ML-DSA-" };
        if !self.parameter_set.starts_with(want_prefix) {
            p.push(format!("parameter_set must start with {want_prefix}"));
        }
        if self.fips.algorithms.is_empty() && self.fips.equations.is_empty() {
            p.push("fips must cite at least one algorithm or equation".into());
        }
        for a in &self.fips.algorithms {
            match self.fips.document.algorithm_name(a.number) {
                Some(name) if name == a.name => {}
                Some(name) => p.push(format!(
                    "{} Algorithm {} is {name:?}, not {:?}",
                    self.fips.document.name(),
                    a.number,
                    a.name
                )),
                None => p.push(format!(
                    "{} has no Algorithm {}",
                    self.fips.document.name(),
                    a.number
                )),
            }
        }

        let sig = self.c.signature.trim();
        if !sig.ends_with(';') || !sig.contains(&format!("{}(", self.c.function)) {
            p.push("c.signature must be a prototype of c.function ending with ';'".into());
        }
        if !sig.starts_with("void ") && self.c.returns.is_none() {
            p.push("c.returns is required for a non-void function".into());
        }
        if self.title.trim().is_empty() || self.input_domain.trim().is_empty() {
            p.push("title and input_domain must be non-empty".into());
        }

        for kind in OracleKind::ALL {
            let Some(a) = self.oracles.get(&kind) else {
                p.push(format!("oracles.{} is missing", kind.name()));
                continue;
            };
            let nonempty = |s: &Option<String>| s.as_deref().is_some_and(|t| !t.trim().is_empty());
            match (a.applies, nonempty(&a.method), nonempty(&a.reason)) {
                (true, true, false) | (false, false, true) => {}
                (true, ..) => p.push(format!(
                    "oracles.{}: applies = true needs a method and no reason",
                    kind.name()
                )),
                (false, ..) => p.push(format!(
                    "oracles.{}: applies = false needs a reason and no method",
                    kind.name()
                )),
            }
        }

        let applies = |k: OracleKind| self.oracles.get(&k).is_some_and(|a| a.applies);
        if applies(OracleKind::Acvp) && self.level != Level::TopLevel {
            p.push("acvp applies only to top-level functions (GAP-ACVP-001)".into());
        }
        for k in [OracleKind::MalformedInput, OracleKind::ImplicitRejection] {
            if applies(k) && !(is_kem && self.level == Level::TopLevel) {
                p.push(format!(
                    "{} applies only to top-level ML-KEM functions",
                    k.name()
                ));
            }
        }
        if applies(OracleKind::Differential) {
            p.push("differential applies only to Arm 3, not to pilot tasks".into());
        }
        let ct = [
            OracleKind::CtValgrind,
            OracleKind::CtStaticDivision,
            OracleKind::CtDudect,
        ];
        if ct.iter().any(|k| applies(*k)) && self.secrets.inputs.is_empty() {
            p.push("constant-time oracles need at least one secret input".into());
        }
        p
    }
}

/// Loads every `spec.md` under `root/<family>/<task>/`, sorted by task ID.
pub fn load_all(root: &Path) -> Result<Vec<TaskSpec>, SpecError> {
    let io = |path: &Path, source| SpecError::Io {
        path: path.display().to_string(),
        source,
    };
    let mut specs = Vec::new();
    let mut families: Vec<_> = std::fs::read_dir(root)
        .map_err(|e| io(root, e))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    families.sort();
    for family in families {
        let mut tasks: Vec<_> = std::fs::read_dir(&family)
            .map_err(|e| io(&family, e))?
            .flatten()
            .map(|e| e.path().join("spec.md"))
            .filter(|p| p.is_file())
            .collect();
        tasks.sort();
        for path in tasks {
            specs.push(TaskSpec::load(&path)?);
        }
    }
    specs.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(specs)
}

/// Renders the task-by-oracle matrix as Markdown: a summary table, then the
/// method or reason for every cell. Output is deterministic.
#[must_use]
pub fn render_matrix_markdown(specs: &[TaskSpec]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    out.push_str("| Task |");
    for k in OracleKind::ALL {
        let _ = write!(out, " {} |", k.name());
    }
    out.push_str("\n|---|");
    for _ in OracleKind::ALL {
        out.push_str(":-:|");
    }
    out.push('\n');
    for s in specs {
        let _ = write!(out, "| {} |", s.id);
        for k in OracleKind::ALL {
            let cell = match s.oracles.get(&k) {
                Some(a) if a.applies => "applies",
                Some(_) => "N/A",
                None => "missing",
            };
            let _ = write!(out, " {cell} |");
        }
        out.push('\n');
    }
    for s in specs {
        let _ = write!(out, "\n## {}\n\n", s.id);
        for k in OracleKind::ALL {
            if let Some(a) = s.oracles.get(&k) {
                let (label, text) = if a.applies {
                    ("applies", a.method.as_deref().unwrap_or(""))
                } else {
                    ("N/A", a.reason.as_deref().unwrap_or(""))
                };
                let _ = writeln!(out, "- **{}** ({label}): {text}", k.name());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"+++
id = "MLKEM-T07-compress-d"
title = "ML-KEM Compress_d"
level = "component"
parameter_set = "ML-KEM-768"
input_domain = "x in [0, q), d in {1, 4, 5, 10, 11}"

[fips]
document = "FIPS 203"
equations = ["4.7"]
sections = ["4.2.1"]

[c]
function = "compress_d"
signature = "uint16_t compress_d(uint16_t x, unsigned d);"
returns = "Compress_d(x)"

[secrets]
inputs = ["x"]

[oracles.reference-intermediate]
applies = true
method = "exhaustive"
[oracles.acvp]
applies = false
reason = "component"
[oracles.malformed-input]
applies = false
reason = "component"
[oracles.implicit-rejection]
applies = false
reason = "component"
[oracles.ct-valgrind]
applies = true
method = "x poisoned"
[oracles.ct-static-division]
applies = true
method = "emitted code"
[oracles.ct-dudect]
applies = false
reason = "not in subsample"
[oracles.differential]
applies = false
reason = "Arm 3 only"
+++
Body text.
"#;

    #[test]
    fn good_spec_has_no_problems() {
        let s = TaskSpec::parse(GOOD).unwrap();
        assert_eq!(s.body, "Body text.\n");
        assert_eq!(s.problems(), Vec::<String>::new());
    }

    #[test]
    fn wrong_algorithm_name_is_caught() {
        let text = GOOD.replace(
            "equations = [\"4.7\"]",
            "algorithms = [{ number = 18, name = \"ML-KEM.Decaps\" }]",
        );
        let p = TaskSpec::parse(&text).unwrap().problems();
        assert!(
            p.iter().any(|m| m.contains("ML-KEM.Decaps_internal")),
            "{p:?}"
        );
    }

    #[test]
    fn missing_oracle_and_bad_applicability_are_caught() {
        let text = GOOD
            .replace(
                "[oracles.differential]\napplies = false\nreason = \"Arm 3 only\"\n",
                "",
            )
            .replace("method = \"exhaustive\"", "reason = \"oops\"");
        let p = TaskSpec::parse(&text).unwrap().problems();
        assert!(
            p.iter()
                .any(|m| m.contains("oracles.differential is missing")),
            "{p:?}"
        );
        assert!(
            p.iter().any(|m| m.contains("reference-intermediate")),
            "{p:?}"
        );
    }

    #[test]
    fn acvp_on_component_is_caught() {
        let text = GOOD.replace(
            "[oracles.acvp]\napplies = false\nreason = \"component\"",
            "[oracles.acvp]\napplies = true\nmethod = \"vectors\"",
        );
        let p = TaskSpec::parse(&text).unwrap().problems();
        assert!(p.iter().any(|m| m.contains("acvp applies only")), "{p:?}");
    }

    #[test]
    fn unknown_fields_and_missing_front_matter_are_errors() {
        assert!(TaskSpec::parse("# no front matter").is_err());
        let text = GOOD.replace("level = \"component\"", "level = \"component\"\nextra = 1");
        assert!(TaskSpec::parse(&text).is_err());
    }
}
