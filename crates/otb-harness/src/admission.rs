//! Candidate admission: a lexical policy check run in Rust before any
//! candidate is compiled (ADR-004).
//!
//! The check exists so that a candidate cannot tamper with the oracles that
//! run in its own process (for example by undoing Valgrind's secret marking
//! with client requests, which need inline assembly), cannot run code
//! before `main`, and cannot read files the harness did not give it. It is
//! a policy, not a proof: it works on tokens, not on the C semantics
//! (GAP-ADM-001). The sandbox remains the security boundary.
//!
//! The rules, in the order they are checked:
//!
//! 1. size at most [`MAX_SOURCE_BYTES`];
//! 2. printable ASCII, tab, CR and LF only;
//! 3. no trigraphs (`-std=c11` enables them, so `??=` would be `#`);
//! 4. after line splicing: comments and literals are skipped, then
//!    - no digraph `%:` (another spelling of `#`), no `##` (token pasting
//!      can build any identifier), no stray backslash (universal character
//!      names can spell identifiers);
//!    - preprocessing directives only from [`ALLOWED_DIRECTIVES`];
//!    - `#include` only of [`ALLOWED_HEADERS`], spelled literally;
//!    - none of [`FORBIDDEN_IDENTIFIERS`], and no identifier containing
//!      `valgrind` in any case.
//!
//! The first violation in source order is reported, so the result is
//! deterministic.

/// Largest admitted source, in bytes.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;

/// Headers a candidate may include. `stdio.h` is excluded: the driver owns
/// stdout, and output from the candidate would corrupt the records.
pub const ALLOWED_HEADERS: [&str; 7] = [
    "limits.h",
    "stdbool.h",
    "stddef.h",
    "stdint.h",
    "stdlib.h",
    "string.h",
    "tasks.h",
];

/// Preprocessing directives a candidate may use.
pub const ALLOWED_DIRECTIVES: [&str; 10] = [
    "define", "elif", "else", "endif", "error", "if", "ifdef", "ifndef", "include", "undef",
];

/// Identifiers a candidate may not use, with the policy each one breaks.
pub const FORBIDDEN_IDENTIFIERS: [(&str, &str); 11] = [
    ("asm", "inline-asm"),
    ("__asm", "inline-asm"),
    ("__asm__", "inline-asm"),
    ("_Pragma", "pragma"),
    ("__pragma", "pragma"),
    ("constructor", "attribute:constructor"),
    ("__constructor__", "attribute:constructor"),
    ("destructor", "attribute:destructor"),
    ("__destructor__", "attribute:destructor"),
    ("ifunc", "attribute:ifunc"),
    ("__ifunc__", "attribute:ifunc"),
];

/// Why a candidate was not admitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    /// The rule broken, e.g. `header:stdio.h`. This string goes into the
    /// `Rejected` verdict and must stay stable.
    pub policy: String,
    /// The 1-based physical line where the violation starts (0 for
    /// whole-file rules).
    pub line: usize,
}

fn reject(policy: impl Into<String>, line: usize) -> Result<(), Rejection> {
    Err(Rejection {
        policy: policy.into(),
        line,
    })
}

/// Checks `source` (a candidate implementation) against the admission
/// policy.
///
/// # Errors
///
/// The first rule the source breaks.
pub fn check(source: &[u8]) -> Result<(), Rejection> {
    check_as(source, false)
}

/// Checks a model-written test file (ADR-006): the candidate policy, plus
/// `otb_test.h` among the allowed headers, and no `#define` or `#undef` of
/// a name starting with `OTB_` or `otb_` (policy `harness-name`), so a suite
/// cannot replace `OTB_ASSERT` or the harness's counters.
///
/// # Errors
///
/// The first rule the source breaks.
pub fn check_test_file(source: &[u8]) -> Result<(), Rejection> {
    check_as(source, true)
}

fn check_as(source: &[u8], test_file: bool) -> Result<(), Rejection> {
    if source.len() > MAX_SOURCE_BYTES {
        return reject("size", 0);
    }
    let mut line = 1;
    for &b in source {
        if !(b == b'\t' || b == b'\n' || b == b'\r' || (0x20..0x7f).contains(&b)) {
            return reject("character-set", line);
        }
        if b == b'\n' {
            line += 1;
        }
    }
    if let Some(i) = source
        .windows(3)
        .position(|w| w[0] == b'?' && w[1] == b'?' && b"=/'()!<>-".contains(&w[2]))
    {
        // Lines before the trigraph, plus one: the number of LF-separated pieces.
        let l = source[..i].split(|&c| c == b'\n').count();
        return reject("trigraph", l);
    }
    let (text, line_of) = splice(source);
    Scanner {
        s: &text,
        line_of: &line_of,
        pos: 0,
        test_file,
    }
    .run()
}

/// Removes backslash-newline pairs (translation phase 2) and keeps, for
/// every remaining byte, its 1-based physical line.
fn splice(source: &[u8]) -> (Vec<u8>, Vec<usize>) {
    let mut text = Vec::with_capacity(source.len());
    let mut line_of = Vec::with_capacity(source.len());
    let mut line = 1;
    let mut i = 0;
    while i < source.len() {
        if source[i] == b'\\' {
            let rest = &source[i + 1..];
            let skip = if rest.starts_with(b"\r\n") {
                3
            } else if rest.starts_with(b"\n") {
                2
            } else {
                0
            };
            if skip > 0 {
                i += skip;
                line += 1;
                continue;
            }
        }
        text.push(source[i]);
        line_of.push(line);
        if source[i] == b'\n' {
            line += 1;
        }
        i += 1;
    }
    (text, line_of)
}

struct Scanner<'a> {
    s: &'a [u8],
    line_of: &'a [usize],
    pos: usize,
    /// True when checking a test file (ADR-006).
    test_file: bool,
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

impl Scanner<'_> {
    fn line(&self) -> usize {
        self.line_of
            .get(self.pos)
            .or_else(|| self.line_of.last())
            .copied()
            .unwrap_or(1)
    }

    fn peek(&self, off: usize) -> u8 {
        self.s.get(self.pos + off).copied().unwrap_or(0)
    }

    fn starts(&self, p: &[u8]) -> bool {
        self.s[self.pos..].starts_with(p)
    }

    /// Skips spaces, tabs, CRs and comments, but not newlines.
    fn skip_blank(&mut self) -> Result<(), Rejection> {
        loop {
            match self.peek(0) {
                b' ' | b'\t' | b'\r' => self.pos += 1,
                b'/' if self.peek(1) == b'/' => {
                    while self.pos < self.s.len() && self.s[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                }
                b'/' if self.peek(1) == b'*' => {
                    let line = self.line();
                    self.pos += 2;
                    loop {
                        if self.pos >= self.s.len() {
                            return reject("unterminated-comment", line);
                        }
                        if self.starts(b"*/") {
                            self.pos += 2;
                            break;
                        }
                        self.pos += 1;
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn ident(&mut self) -> &str {
        let start = self.pos;
        while self.pos < self.s.len() && is_ident(self.s[self.pos]) {
            self.pos += 1;
        }
        // The input is ASCII (checked before scanning).
        std::str::from_utf8(&self.s[start..self.pos]).unwrap_or("")
    }

    fn literal(&mut self, quote: u8) -> Result<(), Rejection> {
        let line = self.line();
        self.pos += 1;
        loop {
            match self.peek(0) {
                0 | b'\n' => return reject("unterminated-literal", line),
                b'\\' => self.pos += 2,
                c if c == quote => {
                    self.pos += 1;
                    return Ok(());
                }
                _ => self.pos += 1,
            }
        }
    }

    fn check_ident(id: &str, line: usize) -> Result<(), Rejection> {
        if let Some((_, policy)) = FORBIDDEN_IDENTIFIERS.iter().find(|(f, _)| *f == id) {
            return reject(*policy, line);
        }
        if id.to_ascii_lowercase().contains("valgrind") {
            return reject("valgrind", line);
        }
        Ok(())
    }

    fn directive(&mut self) -> Result<(), Rejection> {
        let line = self.line();
        self.pos += 1; // '#'
        self.skip_blank()?;
        match self.peek(0) {
            b'\n' | 0 => return Ok(()), // null directive
            c if is_ident_start(c) => {}
            _ => return reject("directive:other", line),
        }
        let name = self.ident().to_owned();
        if !ALLOWED_DIRECTIVES.contains(&name.as_str()) {
            return reject(format!("directive:{name}"), line);
        }
        if self.test_file && (name == "define" || name == "undef") {
            self.skip_blank()?;
            let macro_name = self.ident().to_ascii_lowercase();
            if macro_name.starts_with("otb_") {
                return reject("harness-name", line);
            }
            return Ok(());
        }
        if name != "include" {
            return Ok(());
        }
        self.skip_blank()?;
        let close = match self.peek(0) {
            b'<' => b'>',
            b'"' => b'"',
            _ => return reject("include:computed", line),
        };
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.s.len() && self.s[self.pos] != close && self.s[self.pos] != b'\n' {
            self.pos += 1;
        }
        if self.peek(0) != close {
            return reject("include:malformed", line);
        }
        let header = String::from_utf8_lossy(&self.s[start..self.pos]).into_owned();
        self.pos += 1;
        let test_header = self.test_file && header == "otb_test.h";
        if !ALLOWED_HEADERS.contains(&header.as_str()) && !test_header {
            return reject(format!("header:{header}"), line);
        }
        self.skip_blank()?;
        if !matches!(self.peek(0), b'\n' | 0) {
            return reject("include:malformed", line);
        }
        Ok(())
    }

    fn run(mut self) -> Result<(), Rejection> {
        let mut at_line_start = true;
        while self.pos < self.s.len() {
            self.skip_blank()?;
            if self.pos >= self.s.len() {
                break;
            }
            let line = self.line();
            let c = self.peek(0);
            if c == b'\n' {
                self.pos += 1;
                at_line_start = true;
                continue;
            }
            if self.starts(b"%:") {
                return reject("digraph", line);
            }
            if self.starts(b"##") {
                return reject("token-paste", line);
            }
            if c == b'#' && at_line_start {
                self.directive()?;
                at_line_start = false;
                continue;
            }
            at_line_start = false;
            match c {
                b'"' | b'\'' => self.literal(c)?,
                b'\\' => return reject("backslash", line),
                c if is_ident_start(c) => {
                    let id = self.ident().to_owned();
                    Self::check_ident(&id, line)?;
                }
                c if c.is_ascii_digit() => {
                    // pp-number: digits, letters, '_', '.', and sign after e/E/p/P.
                    self.pos += 1;
                    while self.pos < self.s.len() {
                        let d = self.s[self.pos];
                        let prev = self.s[self.pos - 1];
                        if is_ident(d)
                            || d == b'.'
                            || ((d == b'+' || d == b'-') && b"eEpP".contains(&prev))
                        {
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                }
                _ => self.pos += 1,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(src: &str) -> Option<String> {
        check(src.as_bytes()).err().map(|r| r.policy)
    }

    const GOOD: &str = r#"/* A plausible candidate: "asm" in a comment is fine. */
#include <stdint.h>
#include "tasks.h"
#define Q 3329
#define MASK(d) ((1u << (d)) - 1u)
#if defined(Q) && Q > 0
static const char *note = "asm volatile, #pragma, ## and \\ inside a string";
#endif
// constructor in a comment is fine too
uint16_t compress_d(uint16_t x, unsigned d) {
  uint32_t a = (uint32_t)x << d;
  float f = 1.5e+3f; (void)f; (void)note; char c = '\'';(void)c;
  return (uint16_t)(((a * 2u + Q) / (2u * Q)) & MASK(d));
}
"#;

    #[test]
    fn admits_an_ordinary_candidate() {
        assert_eq!(check(GOOD.as_bytes()), Ok(()));
    }

    #[test]
    fn rejects_each_rule() {
        let cases: &[(&str, &str)] = &[
            ("#include <stdio.h>\n", "header:stdio.h"),
            (
                "#include \"../lib/mlkem_native.c\"\n",
                "header:../lib/mlkem_native.c",
            ),
            (
                "#include <valgrind/memcheck.h>\n",
                "header:valgrind/memcheck.h",
            ),
            ("#define H <stdint.h>\n#include H\n", "include:computed"),
            ("#include <stdint.h> x\n", "include:malformed"),
            ("#pragma GCC optimize(\"O0\")\n", "directive:pragma"),
            ("#line 1 \"x.c\"\n", "directive:line"),
            ("# 1 \"x.c\"\n", "directive:other"),
            ("#include_next <stdint.h>\n", "directive:include_next"),
            ("void f(void){ __asm__ volatile(\"\"); }\n", "inline-asm"),
            ("void f(void){ asm(\"nop\"); }\n", "inline-asm"),
            ("_Pragma(\"GCC optimize 0\")\n", "pragma"),
            (
                "__attribute__((constructor)) static void f(void){}\n",
                "attribute:constructor",
            ),
            (
                "__attribute__((destructor)) static void f(void){}\n",
                "attribute:destructor",
            ),
            ("int VALGRIND_x;\n", "valgrind"),
            ("#define CAT(a,b) a##b\n", "token-paste"),
            ("%:pragma once\n", "digraph"),
            ("??=pragma once\n", "trigraph"),
            ("int \\u0061sm;\n", "backslash"),
            ("/* never closed\n", "unterminated-comment"),
            ("const char *s = \"never closed;\n", "unterminated-literal"),
            ("int caf\u{e9};\n", "character-set"),
            ("int x;\0\n", "character-set"),
        ];
        for (src, want) in cases {
            assert_eq!(policy(src).as_deref(), Some(*want), "{src:?}");
        }
    }

    #[test]
    fn line_splicing_cannot_hide_a_directive_or_identifier() {
        assert_eq!(
            policy("#pra\\\ngma once\n").as_deref(),
            Some("directive:pragma")
        );
        assert_eq!(
            policy("#pra\\\r\ngma once\n").as_deref(),
            Some("directive:pragma")
        );
        assert_eq!(
            policy("void f(void){ __a\\\nsm__(\"\"); }\n").as_deref(),
            Some("inline-asm")
        );
        // A comment cannot hide the start of a directive either.
        assert_eq!(
            policy("/* x */ #pragma once\n").as_deref(),
            Some("directive:pragma")
        );
    }

    #[test]
    fn reports_the_line_of_the_first_violation() {
        let r = check(b"int a;\n\n#include <stdio.h>\nasm(\"\");\n").unwrap_err();
        assert_eq!((r.policy.as_str(), r.line), ("header:stdio.h", 3));
        let r = check(b"int a;\n/* x\n y */ int b;\n??=\n").unwrap_err();
        assert_eq!((r.policy.as_str(), r.line), ("trigraph", 4));
        let r = check(b"int a; \\\nint b;\n_Pragma(\"\")\n").unwrap_err();
        assert_eq!((r.policy.as_str(), r.line), ("pragma", 3));
    }

    #[test]
    fn size_limit() {
        let big = "int a;\n".repeat(MAX_SOURCE_BYTES / 7 + 1);
        assert_eq!(policy(&big).as_deref(), Some("size"));
        let fits = " ".repeat(MAX_SOURCE_BYTES);
        assert_eq!(policy(&fits), None);
    }

    #[test]
    fn test_files_get_otb_test_h_but_cannot_redefine_harness_names() {
        let suite = "#include \"otb_test.h\"\n#include \"tasks.h\"\n#define N 3\nint main(void) { OTB_ASSERT(compress_d(0, 1) == 0); return 0; }\n";
        assert_eq!(check_test_file(suite.as_bytes()), Ok(()));
        // A candidate may not include the test header.
        assert_eq!(policy(suite).as_deref(), Some("header:otb_test.h"));
        for bad in [
            "#define OTB_ASSERT(x) ((void)0)\n",
            "#undef OTB_ASSERT\n",
            "#define otb_assert_record(a, b, c) ((void)0)\n",
        ] {
            let r = check_test_file(bad.as_bytes()).unwrap_err();
            assert_eq!(r.policy, "harness-name", "{bad}");
        }
        // The candidate rules still apply to tests.
        assert_eq!(
            check_test_file(b"#include <stdio.h>\n").unwrap_err().policy,
            "header:stdio.h"
        );
    }

    #[test]
    fn hash_inside_a_line_is_not_a_directive() {
        // Stringizing inside a macro body is allowed.
        assert_eq!(policy("#define S(x) #x\nconst char *p = S(a);\n"), None);
    }
}
