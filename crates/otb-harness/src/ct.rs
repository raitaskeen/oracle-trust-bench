//! Constant-time oracles (ADR-005).
//!
//! - `ct-static-division`: the implementation file is compiled to assembly
//!   for every configuration in [`ASM_CONFIGS`] at -O2 and -O3, inside the
//!   sandbox. Any division instruction, or any reference to a division or
//!   modulo helper routine, in the emitted code is a failure (primary
//!   level; a violation in any configuration counts). The same file is also
//!   compiled to LLVM IR at -O0 and its `udiv`, `sdiv`, `urem`, `srem`,
//!   `fdiv` and `frem` instructions are counted; that count is a
//!   diagnostic only and never decides the outcome.
//! - `ct-valgrind`: the task program is built with `-DOTB_CT_VALGRIND`
//!   (the driver marks the spec's secret inputs undefined) by the pinned
//!   compiler at -O2 and -O3 and run under Valgrind memcheck. A branch or a
//!   memory address that depends on a secret is a failure.
//!
//! The scanners and the log parser are plain functions over text so they
//! can be tested without a sandbox.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::Duration;

use otb_core::{Evidence, FailureClass, OracleKind, Outcome};
use otb_sandbox::{Exit, Limits};

use crate::oracles::{OracleRun, task_flag};
use crate::selftest::class_name;
use crate::{Harness, HarnessError, Implementation};

/// Wall-clock limit for one constant-time oracle run (all compilations, or
/// both Valgrind runs). Compiling a whole reference library ten times is
/// the slowest case.
pub const CT_WALL_LIMIT: Duration = Duration::from_secs(300);

/// One compiler and target for the emitted-code check.
#[derive(Clone, Copy, Debug)]
pub struct AsmConfig {
    /// Stable name used in evidence and reports.
    pub name: &'static str,
    /// Compiler executable in the image.
    pub compiler: &'static str,
    /// Target flags.
    pub flags: &'static str,
    /// True for targets with no C library in the image: compiled against
    /// the declaration-only headers in `c/harness/ct-include` (GAP-CT-005).
    pub cross: bool,
}

/// The emitted-code configurations (ADR-005). GCC 14.2 is the compiler the
/// functional and Valgrind builds use; Clang 19.1.7 covers the other
/// targets. Cortex-M0+ (Armv6-M) is included because it has no 32 x 32 ->
/// 64-bit multiply, so division by a constant stays a division there,
/// while the other targets turn it into a multiplication at -O2 and -O3.
pub const ASM_CONFIGS: [AsmConfig; 5] = [
    AsmConfig {
        name: "gcc-x86_64",
        compiler: "gcc",
        flags: "",
        cross: false,
    },
    AsmConfig {
        name: "clang-x86_64",
        compiler: "clang",
        flags: "--target=x86_64-linux-gnu",
        cross: false,
    },
    AsmConfig {
        name: "clang-aarch64",
        compiler: "clang",
        flags: "--target=aarch64-linux-gnu",
        cross: true,
    },
    AsmConfig {
        name: "clang-armv7em-cortex-m4",
        compiler: "clang",
        flags: "--target=arm-none-eabi -mcpu=cortex-m4 -mthumb -mfloat-abi=soft",
        cross: true,
    },
    AsmConfig {
        name: "clang-armv6m-cortex-m0plus",
        compiler: "clang",
        flags: "--target=arm-none-eabi -mcpu=cortex-m0plus -mthumb -mfloat-abi=soft",
        cross: true,
    },
];

/// Optimisation levels; a violation at either counts.
pub const OPT_LEVELS: [&str; 2] = ["O2", "O3"];

/// Library routines that divide or take a remainder (compiler runtime
/// helpers for targets or widths without a division instruction, soft
/// float, and the C library's `div` family). A reference to any of them in
/// an instruction or a data word counts as a division.
pub const DIVISION_HELPERS: [&str; 33] = [
    "__aeabi_uidiv",
    "__aeabi_idiv",
    "__aeabi_uidivmod",
    "__aeabi_idivmod",
    "__aeabi_uldivmod",
    "__aeabi_ldivmod",
    "__aeabi_fdiv",
    "__aeabi_ddiv",
    "__udivsi3",
    "__divsi3",
    "__umodsi3",
    "__modsi3",
    "__udivmodsi4",
    "__divmodsi4",
    "__udivdi3",
    "__divdi3",
    "__umoddi3",
    "__moddi3",
    "__udivmoddi4",
    "__divmoddi4",
    "__udivti3",
    "__divti3",
    "__umodti3",
    "__modti3",
    "__udivmodti4",
    "__divmodti4",
    "__divsf3",
    "__divdf3",
    "__divtf3",
    "div",
    "ldiv",
    "lldiv",
    "imaxdiv",
];

/// One division found in emitted code or IR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The function it is in (`<file scope>` before the first label).
    pub function: String,
    /// The instruction mnemonic, or `call <helper>`.
    pub what: String,
}

fn is_division_mnemonic(m: &str) -> bool {
    let m = m.strip_prefix('v').unwrap_or(m);
    ["div", "idiv", "udiv", "sdiv", "fdiv"]
        .iter()
        .any(|p| m.starts_with(p))
}

fn helper_in(operands: &str) -> Option<&'static str> {
    operands
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.'))
        .find_map(|tok| DIVISION_HELPERS.iter().find(|h| **h == tok).copied())
}

fn tokens(s: &str) -> impl Iterator<Item = &str> {
    s.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.'))
        .filter(|t| !t.is_empty())
}

/// The functions of one assembly or IR file: for each, the divisions in it
/// and every symbol it refers to; plus the symbols referred to from data.
#[derive(Clone, Debug, Default)]
pub struct Listing {
    /// (function, divisions in it, symbols it refers to), in file order.
    functions: Vec<(String, Vec<String>, Vec<String>)>,
    /// Symbols referred to from data (function pointer tables, literal
    /// pools, global initialisers).
    data_refs: Vec<String>,
}

impl Listing {
    fn current(&mut self) -> &mut (String, Vec<String>, Vec<String>) {
        if self.functions.is_empty() {
            self.functions
                .push(("<file scope>".into(), Vec::new(), Vec::new()));
        }
        self.functions.last_mut().expect("non-empty")
    }

    fn start(&mut self, name: &str) {
        self.functions
            .push((name.to_owned(), Vec::new(), Vec::new()));
    }

    /// Every division in the file.
    #[must_use]
    pub fn all(&self) -> Vec<Finding> {
        self.functions
            .iter()
            .flat_map(|(f, divs, _)| {
                divs.iter().map(|w| Finding {
                    function: f.clone(),
                    what: w.clone(),
                })
            })
            .collect()
    }

    /// The divisions in code reachable from `root`, and the number outside
    /// it. Reachable means: `root`, every function whose address appears in
    /// data, and, transitively, every function a reachable function refers
    /// to by name (a call, a tail call or taking its address). If `root` is
    /// not defined in the file, every function counts as reachable.
    #[must_use]
    pub fn reachable_from(&self, root: &str) -> (Vec<Finding>, usize) {
        let defined: std::collections::BTreeSet<&str> =
            self.functions.iter().map(|(f, _, _)| f.as_str()).collect();
        if !defined.contains(root) {
            return (self.all(), 0);
        }
        let mut reach: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let mut todo: Vec<&str> = vec![root];
        todo.extend(
            self.data_refs
                .iter()
                .map(String::as_str)
                .filter(|s| defined.contains(s)),
        );
        while let Some(f) = todo.pop() {
            if !reach.insert(f) {
                continue;
            }
            for (_, _, refs) in self.functions.iter().filter(|(n, _, _)| n == f) {
                todo.extend(
                    refs.iter()
                        .map(String::as_str)
                        .filter(|s| defined.contains(s) && !reach.contains(s)),
                );
            }
        }
        let mut inside = Vec::new();
        let mut outside = 0;
        for (f, divs, _) in &self.functions {
            if reach.contains(f.as_str()) {
                inside.extend(divs.iter().map(|w| Finding {
                    function: f.clone(),
                    what: w.clone(),
                }));
            } else {
                outside += divs.len();
            }
        }
        (inside, outside)
    }
}

/// Parses GNU-syntax assembly (x86-64 AT&T, AArch64, Arm Thumb).
#[must_use]
pub fn parse_asm(asm: &str) -> Listing {
    let mut l = Listing::default();
    for line in asm.lines() {
        let Some(first) = line.split_whitespace().next() else {
            continue;
        };
        if !line.starts_with([' ', '\t']) {
            // A label at column 0. Local labels (.L...) and numeric labels
            // do not start a function.
            if let Some(name) = first.strip_suffix(':')
                && !name.starts_with('.')
                && !name.chars().all(|c| c.is_ascii_digit())
            {
                l.start(name);
            }
            continue;
        }
        if first.starts_with(['#', '@', ';']) || first.starts_with("//") {
            continue;
        }
        let operands = line.trim_start()[first.len()..].trim();
        if let Some(directive) = first.strip_prefix('.') {
            // Data words can hold function addresses and helper addresses
            // (literal pools, function pointer tables).
            if matches!(
                directive,
                "word" | "long" | "quad" | "4byte" | "8byte" | "xword"
            ) {
                l.data_refs.extend(tokens(operands).map(str::to_owned));
                if let Some(h) = helper_in(operands) {
                    l.current().1.push(format!("address of {h}"));
                }
            }
            continue;
        }
        let mnemonic = first.to_ascii_lowercase();
        let cur = l.current();
        if is_division_mnemonic(&mnemonic) {
            cur.1.push(mnemonic);
        } else if let Some(h) = helper_in(operands) {
            cur.1.push(format!("{mnemonic} {h}"));
        }
        cur.2.extend(tokens(operands).map(str::to_owned));
    }
    l
}

/// Every division in GNU-syntax assembly, whole file.
#[must_use]
pub fn scan_asm(asm: &str) -> Vec<Finding> {
    parse_asm(asm).all()
}

/// Parses LLVM IR: functions, their division and remainder instructions,
/// and the `@` symbols each refers to.
#[must_use]
pub fn parse_ir(ir: &str) -> Listing {
    let mut l = Listing::default();
    let mut in_function = false;
    for line in ir.lines() {
        if let Some(rest) = line.strip_prefix("define ") {
            if let Some(at) = rest.find('@') {
                let name: String = rest[at + 1..].chars().take_while(|c| *c != '(').collect();
                l.start(name.trim_matches('"'));
                in_function = true;
            }
            continue;
        }
        if line.starts_with('}') {
            in_function = false;
            continue;
        }
        let refs = line
            .split('@')
            .skip(1)
            .filter_map(|s| tokens(s).next())
            .map(str::to_owned);
        if !in_function {
            if line.starts_with('@') {
                // A global's initialiser: data references.
                l.data_refs.extend(refs.skip(1));
            }
            continue;
        }
        let refs: Vec<String> = refs.collect();
        let cur = l.current();
        cur.2.extend(refs);
        if let Some((_, rhs)) = line.split_once(" = ")
            && let Some(op) = rhs.split_whitespace().next()
            && matches!(op, "udiv" | "sdiv" | "urem" | "srem" | "fdiv" | "frem")
        {
            cur.1.push(op.to_owned());
        }
    }
    l
}

/// Every division or remainder instruction in LLVM IR, whole file.
#[must_use]
pub fn scan_ir(ir: &str) -> Vec<Finding> {
    parse_ir(ir).all()
}

fn fail_run(task: &str, oracle: OracleKind, cases: u64, evidence: Evidence) -> OracleRun {
    OracleRun {
        task: task.to_owned(),
        oracle,
        outcome: Outcome::Fail { evidence },
        cases,
        groups: BTreeMap::new(),
    }
}

fn whole_fail(task: &str, oracle: OracleKind, class: FailureClass, detail: &str) -> OracleRun {
    let n = u64::try_from(match oracle {
        OracleKind::CtStaticDivision => ASM_CONFIGS.len() * OPT_LEVELS.len(),
        _ => OPT_LEVELS.len(),
    })
    .expect("small");
    fail_run(
        task,
        oracle,
        n,
        Evidence {
            class,
            cases_total: n,
            cases_failed: n,
            first_failing_case: None,
            detail: detail.to_owned(),
        },
    )
}

fn check_sandbox(out: &otb_sandbox::Outcome, what: &str) -> Result<(), HarnessError> {
    if out.truncated {
        return Err(HarnessError::Input(format!("{what}: output truncated")));
    }
    match out.exit {
        Exit::Code(0) | Exit::TimedOut => Ok(()),
        _ => Err(HarnessError::Program {
            what: what.to_owned(),
            exit: out.exit,
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }),
    }
}

fn static_script(lib_include: &str) -> String {
    let src = "\"$OTB_IN/impl/impl.c\"";
    let inc = format!("-I\"$OTB_IN/harness\"{lib_include}");
    let cross = "-nostdlibinc -isystem \"$OTB_IN/harness/ct-include\"";
    let mut s = String::new();
    for c in ASM_CONFIGS {
        for o in OPT_LEVELS {
            let extra = if c.cross { cross } else { "" };
            let _ = writeln!(
                s,
                "echo 'OTB-ASM {} {o}'; {} -std=c11 -{o} {} {extra} {inc} -S -o - {src} \
                 || echo OTB-ASM-FAILED",
                c.name, c.compiler, c.flags
            );
        }
    }
    let _ = writeln!(
        s,
        "echo OTB-IR; clang -std=c11 -O0 {inc} -S -emit-llvm -o - {src} || echo OTB-IR-FAILED"
    );
    s
}

/// Runs `ct-static-division` on `imp`. Only divisions in code reachable
/// from the task's C function (`root`, from the spec) count; divisions
/// elsewhere in the file are reported but not counted (ADR-005).
///
/// # Errors
///
/// A harness failure (staging, sandbox, truncated output).
///
/// Kept as one function so the section format is read in one place.
#[allow(clippy::too_many_lines)]
pub fn static_division(
    h: &Harness,
    task: &str,
    root: &str,
    imp: &Implementation,
) -> Result<OracleRun, HarnessError> {
    const ORACLE: OracleKind = OracleKind::CtStaticDivision;
    let out = h.run_impl_script(
        imp,
        static_script,
        Vec::new(),
        Limits {
            wall: CT_WALL_LIMIT,
            ..Limits::default()
        },
    )?;
    check_sandbox(&out, "ct-static-division")?;
    if out.exit == Exit::TimedOut {
        return Ok(whole_fail(
            task,
            ORACLE,
            FailureClass::Timeout,
            "compilation exceeded the wall-clock limit",
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);

    // Split into sections: (config, opt) -> (asm text, failed); then IR.
    let mut sections: Vec<(String, String, bool)> = Vec::new();
    let mut ir = String::new();
    let mut ir_failed = false;
    let mut in_ir = false;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("OTB-ASM ") {
            sections.push((rest.to_owned(), String::new(), false));
            in_ir = false;
        } else if line == "OTB-ASM-FAILED" {
            if let Some(s) = sections.last_mut() {
                s.2 = true;
            }
        } else if line == "OTB-IR" {
            in_ir = true;
        } else if line == "OTB-IR-FAILED" {
            ir_failed = true;
        } else if in_ir {
            ir.push_str(line);
            ir.push('\n');
        } else if let Some(s) = sections.last_mut() {
            s.1.push_str(line);
            s.1.push('\n');
        }
    }
    if sections.len() != ASM_CONFIGS.len() * OPT_LEVELS.len() {
        return Err(HarnessError::Input(format!(
            "ct-static-division: expected {} assembly sections, got {}",
            ASM_CONFIGS.len() * OPT_LEVELS.len(),
            sections.len()
        )));
    }
    // The first section is GCC x86-64 at -O2, the build that is also run:
    // if it does not compile, the implementation does not compile.
    if sections[0].2 {
        return Ok(whole_fail(
            task,
            ORACLE,
            FailureClass::CompileError,
            "did not compile with the pinned toolchain",
        ));
    }

    let mut groups = BTreeMap::new();
    let mut compiled = 0u64;
    let mut failed = 0u64;
    let mut first = None;
    for (label, asm, did_fail) in &sections {
        if *did_fail {
            groups.insert(format!("emitted {label}: not compiled (GAP-CT-005)"), 0);
            continue;
        }
        compiled += 1;
        let (found, outside) = parse_asm(asm).reachable_from(root);
        if outside > 0 {
            groups.insert(
                format!("emitted {label}: divisions outside the task function's call graph (not counted)"),
                u64::try_from(outside).expect("small"),
            );
        }
        if !found.is_empty() {
            failed += 1;
            groups.insert(
                format!("emitted {label}: divisions"),
                u64::try_from(found.len()).expect("small"),
            );
            if first.is_none() {
                first = Some(format!(
                    "{label}: {} in {}",
                    found[0].what, found[0].function
                ));
            }
        }
    }
    let (ir_found, _) = parse_ir(&ir).reachable_from(root);
    let diag = if ir_failed {
        "diagnostic, not part of the verdict: LLVM IR -O0 not compiled".to_owned()
    } else if let Some(f) = ir_found.first() {
        format!(
            "diagnostic, not part of the verdict: LLVM IR -O0 division/remainder (first: {} in {})",
            f.what, f.function
        )
    } else {
        "diagnostic, not part of the verdict: LLVM IR -O0 division/remainder".to_owned()
    };
    groups.insert(diag, u64::try_from(ir_found.len()).expect("small"));

    let outcome = if failed == 0 {
        Outcome::Pass
    } else {
        Outcome::Fail {
            evidence: Evidence {
                class: FailureClass::VariableTimeInstruction,
                cases_total: compiled,
                cases_failed: failed,
                first_failing_case: first,
                detail: "division instruction or division helper in emitted code (ADR-005)".into(),
            },
        }
    };
    Ok(OracleRun {
        task: task.to_owned(),
        oracle: ORACLE,
        outcome,
        cases: compiled,
        groups,
    })
}

/// One error reported by Valgrind memcheck.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VgError {
    /// Failure class.
    pub class: FailureClass,
    /// The error's first line, without the process prefix.
    pub kind: String,
    /// The innermost stack frame (`function (file:line)`), without the
    /// address, if Valgrind printed one.
    pub location: Option<String>,
}

fn classify_vg(kind: &str) -> FailureClass {
    if kind.starts_with("Conditional jump or move depends on uninitialised value") {
        FailureClass::SecretDependentBranch
    } else if kind.starts_with("Use of uninitialised value of size") {
        FailureClass::SecretDependentAddress
    } else {
        FailureClass::Crash
    }
}

/// Strips Valgrind's `==PID== ` prefix.
fn vg_body(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("==")?;
    let end = rest.find("==")?;
    if end == 0 || !rest[..end].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let body = &rest[end + 2..];
    Some(body.strip_prefix(' ').unwrap_or(body))
}

/// Parses memcheck's `-q` output into errors, in order.
#[must_use]
pub fn parse_valgrind(log: &str) -> Vec<VgError> {
    let mut errors: Vec<VgError> = Vec::new();
    let mut open = false;
    for line in log.lines() {
        let Some(body) = vg_body(line) else {
            continue;
        };
        if body.trim().is_empty() {
            open = false;
            continue;
        }
        if body.starts_with(' ') {
            let t = body.trim_start();
            if open
                && let Some(frame) = t.strip_prefix("at ")
                && let Some(last) = errors.last_mut()
                && last.location.is_none()
            {
                let place = frame.split_once(": ").map_or(frame, |(_, p)| p);
                last.location = Some(place.to_owned());
            }
            continue;
        }
        if body.to_ascii_lowercase().starts_with("warning:") {
            open = false;
            continue;
        }
        errors.push(VgError {
            class: classify_vg(body),
            kind: body.to_owned(),
            location: None,
        });
        open = true;
    }
    errors
}

fn valgrind_script(flag: &str, lib_include: &str) -> String {
    let mut s = String::from("cat > \"$OTB_WORK/in.bin\"\n");
    for o in OPT_LEVELS {
        let _ = writeln!(
            s,
            "if cc -std=c11 -{o} -g -DOTB_CT_VALGRIND -DOTB_TASK_{flag} \
             -I\"$OTB_IN/harness\"{lib_include} \"$OTB_IN/harness/drivers.c\" \
             \"$OTB_IN/impl/impl.c\" -o \"$OTB_WORK/p{o}\"; then \
             echo 'OTB-VG-BEGIN {o}'; \
             valgrind -q --tool=memcheck --error-limit=no --log-fd=3 \"$OTB_WORK/p{o}\" \
             < \"$OTB_WORK/in.bin\" 3>&1 >/dev/null; \
             echo \"OTB-VG-END {o} $?\"; \
             else echo 'OTB-VG-NOBUILD {o}'; fi"
        );
    }
    s
}

/// Runs `ct-valgrind` on `imp` with the driver records in `input`.
///
/// # Errors
///
/// A harness failure (staging, sandbox, truncated output, missing markers).
///
/// Kept as one function so the marker format is read in one place.
#[allow(clippy::too_many_lines)]
pub fn valgrind(
    h: &Harness,
    task: &str,
    imp: &Implementation,
    input: Vec<u8>,
) -> Result<OracleRun, HarnessError> {
    const ORACLE: OracleKind = OracleKind::CtValgrind;
    let flag = task_flag(task).to_owned();
    let out = h.run_impl_script(
        imp,
        |lib| valgrind_script(&flag, lib),
        input,
        Limits {
            wall: CT_WALL_LIMIT,
            ..Limits::default()
        },
    )?;
    check_sandbox(&out, "ct-valgrind")?;
    if out.exit == Exit::TimedOut {
        return Ok(whole_fail(
            task,
            ORACLE,
            FailureClass::Timeout,
            "Valgrind run exceeded the wall-clock limit",
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    if text.lines().any(|l| l.starts_with("OTB-VG-NOBUILD")) {
        return Ok(whole_fail(
            task,
            ORACLE,
            FailureClass::CompileError,
            "did not compile with the pinned toolchain",
        ));
    }
    let mut groups = BTreeMap::new();
    let mut failed = 0u64;
    let mut first: Option<(FailureClass, String)> = None;
    let mut seen = 0;
    let mut current: Option<(String, String)> = None; // (opt, log)
    for line in text.lines() {
        if let Some(o) = line.strip_prefix("OTB-VG-BEGIN ") {
            current = Some((o.to_owned(), String::new()));
        } else if let Some(rest) = line.strip_prefix("OTB-VG-END ") {
            let Some((opt, log)) = current.take() else {
                continue;
            };
            seen += 1;
            let code = rest.split_whitespace().nth(1).unwrap_or("?");
            let mut errors = parse_valgrind(&log);
            if code != "0" {
                errors.push(VgError {
                    class: FailureClass::Crash,
                    kind: format!("program exited with code {code} under Valgrind"),
                    location: None,
                });
            }
            if errors.is_empty() {
                continue;
            }
            failed += 1;
            let label = format!("gcc-x86_64 -{opt}");
            for e in &errors {
                *groups
                    .entry(format!("{label}: {}", class_name(e.class)))
                    .or_insert(0) += 1;
            }
            if first.is_none() {
                let e = &errors[0];
                first = Some((
                    e.class,
                    format!(
                        "{label}: {}{}",
                        e.kind,
                        e.location
                            .as_ref()
                            .map_or_else(String::new, |l| format!(" at {l}"))
                    ),
                ));
            }
        } else if let Some((_, log)) = current.as_mut() {
            log.push_str(line);
            log.push('\n');
        }
    }
    if seen != OPT_LEVELS.len() {
        return Err(HarnessError::Input(format!(
            "ct-valgrind: expected {} Valgrind runs, saw {seen}\n{}",
            OPT_LEVELS.len(),
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    let total = u64::try_from(OPT_LEVELS.len()).expect("small");
    let outcome = match first {
        None => Outcome::Pass,
        Some((class, case)) => Outcome::Fail {
            evidence: Evidence {
                class,
                cases_total: total,
                cases_failed: failed,
                first_failing_case: Some(case),
                detail: "secret-dependent branch or memory address under Valgrind memcheck \
                         (ADR-005)"
                    .into(),
            },
        },
    };
    Ok(OracleRun {
        task: task.to_owned(),
        oracle: ORACLE,
        outcome,
        cases: total,
        groups,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assembly captured from the pinned image (clang 19.1.7) for
    /// `uint32_t vdiv(uint32_t a, uint32_t b) { return a / b; }` and a
    /// division by the constant 3329, trimmed to the relevant lines.
    const X86: &str = "\t.text\n\t.globl\tvdiv                            # -- Begin function vdiv\n\
vdiv:                                   # @vdiv\n\t.cfi_startproc\n\tmovl\t%edi, %eax\n\txorl\t%edx, %edx\n\tdivl\t%esi\n\tretq\n\
compress_d:\n\timulq\t$989558401, %rax, %rax\n\tretq\n";
    const M0: &str = "compress_d:\n\tpush\t{r7, lr}\n\tbl\t__aeabi_uidiv\n\tbics\tr0, r1\n\tpop\t{r7, pc}\n\
.LBB0_1:\n\tldr\tr1, .LCPI0_0\n.LCPI0_0:\n\t.long\t__aeabi_idivmod\n";
    const A64: &str =
        "vdiv:\n\tudiv\tw0, w0, w1\n\tret\nok:\n\tumull\tx8, w9, w8\n\t// udiv in a comment\n";

    #[test]
    fn finds_division_instructions_and_helpers() {
        assert_eq!(
            scan_asm(X86),
            vec![Finding {
                function: "vdiv".into(),
                what: "divl".into()
            }]
        );
        assert_eq!(
            scan_asm(M0),
            vec![
                Finding {
                    function: "compress_d".into(),
                    what: "bl __aeabi_uidiv".into()
                },
                Finding {
                    function: "compress_d".into(),
                    what: "address of __aeabi_idivmod".into()
                }
            ]
        );
        assert_eq!(scan_asm(A64).len(), 1);
        assert_eq!(scan_asm(A64)[0].what, "udiv");
    }

    #[test]
    fn multiplication_and_comments_are_not_divisions() {
        let clean = "f:\n\timull\t$5, %eax\n\tmulq\t%rcx\n\t# divl in a comment\n\tcall\tdivide_by_two@PLT\n\
\tcall\tmemcpy@PLT\n\t.ascii\t\"div\"\n";
        assert!(scan_asm(clean).is_empty(), "{:?}", scan_asm(clean));
    }

    #[test]
    fn x86_variants_and_float_division() {
        for m in [
            "idivq", "divb", "divss", "vdivsd", "fdivrp", "sdiv", "udiveq",
        ] {
            let asm = format!("f:\n\t{m}\t%x\n");
            assert_eq!(scan_asm(&asm).len(), 1, "{m}");
        }
        let calls = "f:\n\tcallq\t__udivti3@PLT\n\tjmp\tldiv\n";
        assert_eq!(scan_asm(calls).len(), 2);
    }

    /// Shape of the mlkem-native Cortex-M0+ listing: the task function
    /// calls a helper; an unrelated function divides; a function whose
    /// address sits in a data table divides too.
    const CALLS: &str = "ntt:\n\tpush\t{r7, lr}\n\tbl\tpoly_ntt\n\tpop\t{r7, pc}\n\
poly_ntt:\n\tmuls\tr0, r1\n\tbx\tlr\n\
shake256x4:\n\tbl\t__aeabi_uidiv\n\tbx\tlr\n\
via_table:\n\tudiv\tr0, r0, r1\n\tbx\tlr\n\
table:\n\t.long\tvia_table\n\
decaps:\n\tbl\tenc\n\tbx\tlr\n\
enc:\n\tb\tshake256x4\n";

    #[test]
    fn reachability_counts_only_code_the_task_can_run() {
        let l = parse_asm(CALLS);
        // ntt reaches poly_ntt (no division); via_table is reachable through
        // the data table; shake256x4 is not reachable from ntt.
        let (inside, outside) = l.reachable_from("ntt");
        assert_eq!(
            inside,
            vec![Finding {
                function: "via_table".into(),
                what: "udiv".into()
            }]
        );
        assert_eq!(outside, 1);
        // decaps reaches shake256x4 through a tail call in enc.
        let (inside, outside) = l.reachable_from("decaps");
        assert_eq!(inside.len(), 2);
        assert_eq!(outside, 0);
        // An unknown root falls back to the whole file.
        assert_eq!(l.reachable_from("absent").0.len(), 2);
    }

    #[test]
    fn ir_reachability() {
        let ir = "@tbl = global [1 x ptr] [ptr @g]\n\
define i32 @f(i32 %a) {\n  %r = call i32 @h(i32 %a)\n  ret i32 %r\n}\n\
define i32 @h(i32 %a) {\n  %q = udiv i32 %a, 3\n  ret i32 %q\n}\n\
define i32 @g(i32 %a) {\n  %q = srem i32 %a, 3\n  ret i32 %q\n}\n\
define i32 @dead(i32 %a) {\n  %q = urem i32 %a, 3\n  ret i32 %q\n}\n";
        let (inside, outside) = parse_ir(ir).reachable_from("f");
        let got: Vec<&str> = inside.iter().map(|f| f.function.as_str()).collect();
        assert_eq!(got, ["h", "g"]);
        assert_eq!(outside, 1);
    }

    #[test]
    fn ir_scan() {
        let ir = "define dso_local zeroext i16 @compress_d(i16 noundef zeroext %0) #0 {\n  \
%12 = udiv i32 %11, 3329\n  %13 = mul i32 %12, 2\n}\ndefine i32 @g(i32 %a) {\n  %r = srem i32 %a, 7\n  \
%s = udiv exact i32 %a, 4\n}\n";
        let f = scan_ir(ir);
        let got: Vec<(&str, &str)> = f
            .iter()
            .map(|x| (x.function.as_str(), x.what.as_str()))
            .collect();
        assert_eq!(got, [("compress_d", "udiv"), ("g", "srem"), ("g", "udiv")]);
    }

    /// Lines in the format Valgrind 3.24.0 printed in the pinned image
    /// (`docs/notes/ct-measurements.md`), with a warning and a memory error.
    #[test]
    fn parses_memcheck_output() {
        let log = "==23== Conditional jump or move depends on uninitialised value(s)\n\
==23==    at 0x109543: br (impl.c:6)\n==23==    by 0x1090A7: main (drivers.c:15)\n==23== \n\
==26== Warning: set address range perms: large range\n\
==26== Use of uninitialised value of size 8\n==26==    at 0x1095A3: idx (impl.c:7)\n==26== \n\
==26== Invalid read of size 4\n==26==    at 0x1095B0: idx (impl.c:8)\n\
not a valgrind line\n";
        let e = parse_valgrind(log);
        assert_eq!(e.len(), 3);
        assert_eq!(e[0].class, FailureClass::SecretDependentBranch);
        assert_eq!(e[0].location.as_deref(), Some("br (impl.c:6)"));
        assert_eq!(e[1].class, FailureClass::SecretDependentAddress);
        assert_eq!(e[1].location.as_deref(), Some("idx (impl.c:7)"));
        assert_eq!(e[2].class, FailureClass::Crash);
        assert_eq!(parse_valgrind(""), vec![]);
        assert_eq!(parse_valgrind("==x== Conditional jump\n"), vec![]);
    }

    #[test]
    fn scripts_cover_every_configuration() {
        let s = static_script("");
        assert_eq!(
            s.matches("OTB-ASM ").count(),
            ASM_CONFIGS.len() * OPT_LEVELS.len()
        );
        assert!(s.contains("-mcpu=cortex-m0plus"));
        assert_eq!(s.matches("-nostdlibinc").count(), 3 * OPT_LEVELS.len());
        let v = valgrind_script("T07", " -I\"$OTB_IN/lib\"");
        assert!(v.contains("-O2 -g -DOTB_CT_VALGRIND -DOTB_TASK_T07"));
        assert!(v.contains("-O3 -g"));
    }
}
