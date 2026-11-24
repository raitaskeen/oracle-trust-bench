//! Asserts what Valgrind memcheck reports in the pinned image, so the
//! blind spots of `ct-valgrind` stay measured, not assumed (GAP-CT-001,
//! `docs/notes/ct-measurements.md`).
//!
//! Needs Docker and `OTB_SANDBOX_IMAGE`, so it is ignored by default and run
//! by the CI sandbox job with `--run-ignored all`.

use std::path::{Path, PathBuf};

use otb_harness::{Harness, ImplKind, Implementation};
use otb_sandbox::{Backend, Limits};

fn harness() -> Harness {
    let image = std::env::var("OTB_SANDBOX_IMAGE").expect("set OTB_SANDBOX_IMAGE to the image ID");
    Harness {
        repo: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        backend: Backend::Docker { image },
    }
}

#[test]
#[ignore = "needs Docker and OTB_SANDBOX_IMAGE"]
fn valgrind_reports_branches_and_addresses_but_not_division_or_cmov() {
    let h = harness();
    let probe = Implementation {
        label: "valgrind probe".into(),
        kind: ImplKind::SeededFault(None),
        source: PathBuf::from("c/ct-probes/valgrind_probe.c"),
    };
    let script = |_: &str| {
        [
            "gcc -std=c11 -O2 -g \"$OTB_IN/impl/impl.c\" -o \"$OTB_WORK/p\" || exit 3",
            "echo 'OTB-SECTION sel asm'; gcc -std=c11 -O2 -S -o - \"$OTB_IN/impl/impl.c\" | awk '/^sel:/,/ret/'",
            "for m in d s b i; do echo \"OTB-SECTION mode $m\"; valgrind -q --log-fd=1 \"$OTB_WORK/p\" $m; done",
        ]
        .join("\n")
    };
    let out = h
        .run_impl_script(&probe, script, Vec::new(), Limits::default())
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.success(),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let section = |name: &str| -> String {
        text.split("OTB-SECTION ")
            .find(|s| s.starts_with(name))
            .unwrap_or_else(|| panic!("no section {name}:\n{text}"))
            .to_owned()
    };
    // The select compiles to a conditional move.
    assert!(section("sel asm").contains("cmov"), "{text}");
    // Division and conditional moves on secrets: nothing reported.
    for m in ["mode d", "mode s"] {
        let s = section(m);
        assert!(!s.contains("uninitialised"), "{m}: {s}");
    }
    // Secret-dependent branch and address: reported.
    assert!(
        section("mode b").contains("Conditional jump or move depends on uninitialised value"),
        "{text}"
    );
    assert!(
        section("mode i").contains("Use of uninitialised value of size"),
        "{text}"
    );
}
