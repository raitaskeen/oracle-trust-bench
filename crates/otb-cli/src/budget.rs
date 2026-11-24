//! `otb budget`: a port of `analysis/budget.py` with exact arithmetic.
//!
//! The Python script uses binary floats. This port keeps every amount as an
//! integer number of picodollars (10^-12 USD), so no rounding happens until a
//! value is printed. Printed values are rounded half up to cents. CI checks
//! that both programs print the same TOTAL line.
//!
//! Prices and token volumes are copied from `analysis/budget.py` (prices:
//! Standard tier, USD per 1M tokens, fetched 2026-11-24). A test fails if
//! the two disagree.

use std::fmt::Write as _;

/// Price per 1M tokens in micro-dollars: (input, output).
const PRICES: [(&str, u64, u64); 3] = [
    ("gpt-6-luna", 100_000, 500_000),
    ("gpt-6.2-sol", 2_000_000, 10_000_000),
    ("gpt-6.2-astra", 8_000_000, 40_000_000),
];

/// One budget line: (stage, model, calls, input tokens per call, output tokens per call).
type Row = (&'static str, &'static str, u64, u64, u64);

fn rows() -> Vec<Row> {
    let tiers = PRICES.map(|p| p.0);
    let mut r = Vec::new();
    for m in tiers {
        r.push((
            "Arm 1: implement 60 tasks x 3 prompts x 5 samples",
            m,
            900,
            3500,
            6000,
        ));
    }
    for m in tiers {
        r.push((
            "Arm 2: self-written test suite per implementation",
            m,
            900,
            5000,
            5000,
        ));
    }
    r.push((
        "Arm 2: audit of luna + astra code by sol (all 1,800)",
        "gpt-6.2-sol",
        1800,
        6000,
        3000,
    ));
    r.push((
        "Arm 2: audit of sol code by astra (stratified 650)",
        "gpt-6.2-astra",
        650,
        6000,
        3000,
    ));
    for m in tiers {
        r.push((
            "Arm 3: characterization tests, 40 routines x 5",
            m,
            200,
            4000,
            4000,
        ));
        r.push((
            "Arm 3: C-to-Rust migration, 40 routines x 5",
            m,
            200,
            4000,
            5000,
        ));
    }
    r
}

/// Pilot and reserve, each 5% (budget.py: `PILOT, RESERVE = 0.05, 0.05`).
const PILOT_PERCENT: u128 = 5;
const RESERVE_PERCENT: u128 = 5;

/// Cost of a row in picodollars: calls * (in * price_in + out * price_out),
/// with prices in micro-dollars per million tokens.
fn cost_pico(model: &str, calls: u64, input: u64, output: u64) -> u128 {
    let (_, pi, po) = PRICES
        .iter()
        .find(|p| p.0 == model)
        .expect("every row uses a priced model");
    u128::from(calls) * (u128::from(input) * u128::from(*pi) + u128::from(output) * u128::from(*po))
}

/// Formats picodollars as dollars rounded half up to cents.
fn dollars(pico: u128) -> String {
    let cents = (pico + 5_000_000_000) / 10_000_000_000;
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// Formats a token count as millions with two decimals, rounded half up.
fn millions(tokens: u128) -> String {
    let hundredths = (tokens + 5_000) / 10_000;
    format!("{}.{:02}", hundredths / 100, hundredths % 100)
}

/// The exact totals, in picodollars: (subtotal, pilot, reserve, total).
fn totals() -> (u128, u128, u128, u128) {
    let sub: u128 = rows()
        .iter()
        .map(|&(_, m, c, i, o)| cost_pico(m, c, i, o))
        .sum();
    assert_eq!(sub * PILOT_PERCENT % 100, 0, "pilot share is exact");
    let pilot = sub * PILOT_PERCENT / 100;
    let reserve = sub * RESERVE_PERCENT / 100;
    (sub, pilot, reserve, sub + pilot + reserve)
}

/// Renders the same table as `python3 analysis/budget.py`.
#[must_use]
pub fn render() -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:52} {:14} {:>6} {:>7} {:>7} {:>9}",
        "Stage", "Model", "Calls", "In M", "Out M", "USD"
    );
    let (mut calls, mut tin, mut tout) = (0u64, 0u128, 0u128);
    for (s, m, c, i, o) in rows() {
        let usd = cost_pico(m, c, i, o);
        calls += c;
        tin += u128::from(c * i);
        tout += u128::from(c * o);
        let _ = writeln!(
            out,
            "{s:52} {m:14} {c:>6} {:>7} {:>7} {:>9}",
            millions(u128::from(c * i)),
            millions(u128::from(c * o)),
            dollars(usd)
        );
    }
    let (sub, pilot, reserve, total) = totals();
    let _ = writeln!(
        out,
        "{:52} {:14} {calls:>6} {:>7} {:>7} {:>9}",
        "Subtotal",
        "",
        millions(tin),
        millions(tout),
        dollars(sub)
    );
    let blank = "";
    let _ = writeln!(
        out,
        "{:52} {blank:14} {blank:6} {blank:7} {blank:7} {:>9}",
        "Pilot and prompt development (5%)",
        dollars(pilot)
    );
    let _ = writeln!(
        out,
        "{:52} {blank:14} {blank:6} {blank:7} {blank:7} {:>9}",
        "Rerun reserve (5%)",
        dollars(reserve)
    );
    let _ = writeln!(
        out,
        "{:52} {blank:14} {blank:6} {blank:7} {blank:7} {:>9}",
        "TOTAL",
        dollars(total)
    );
    let by_model: Vec<String> = PRICES
        .iter()
        .map(|(m, ..)| {
            let v: u128 = rows()
                .iter()
                .filter(|r| r.1 == *m)
                .map(|&(_, mm, c, i, o)| cost_pico(mm, c, i, o))
                .sum();
            format!("{m}: {}", dollars(v))
        })
        .collect();
    let _ = writeln!(out, "By model: {}", by_model.join(", "));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PICO_PER_DOLLAR: u128 = 1_000_000_000_000;

    /// Exact values, from the application's B-full table
    /// (`protocol/APPLICATION.md`): subtotal 869.075 (shown as 869.08),
    /// pilot and reserve 43.45375 each, total 955.9825 (shown as 955.98).
    #[test]
    fn exact_totals_match_the_application() {
        let (sub, pilot, reserve, total) = totals();
        assert_eq!(sub, 869_075 * PICO_PER_DOLLAR / 1000);
        assert_eq!(pilot, 43_453_750 * PICO_PER_DOLLAR / 1_000_000);
        assert_eq!(reserve, pilot);
        assert_eq!(total, 9_559_825 * PICO_PER_DOLLAR / 10_000);
        assert_eq!(dollars(total), "955.98");
    }

    #[test]
    fn calls_and_tokens_match_the_application() {
        let r = rows();
        assert_eq!(r.iter().map(|x| x.2).sum::<u64>(), 9_050);
        let tin: u64 = r.iter().map(|x| x.2 * x.3).sum();
        let tout: u64 = r.iter().map(|x| x.2 * x.4).sum();
        assert_eq!((tin, tout), (42_450_000, 42_450_000));
    }

    #[test]
    fn by_model_matches_the_application() {
        // Application, B-1500: astra 651.20, sol 211.10, luna 6.78.
        let text = render();
        let line = text.lines().last().unwrap();
        assert_eq!(
            line,
            "By model: gpt-6-luna: 6.78, gpt-6.2-sol: 211.10, gpt-6.2-astra: 651.20"
        );
    }

    #[test]
    fn rounding_is_half_up() {
        assert_eq!(dollars(5 * PICO_PER_DOLLAR / 1000), "0.01");
        assert_eq!(dollars(4_999 * PICO_PER_DOLLAR / 1_000_000), "0.00");
    }

    /// The prices and volumes in this file must be the ones in budget.py.
    #[test]
    fn constants_match_budget_py() {
        let py = include_str!("../../../analysis/budget.py");
        for needle in [
            r#""gpt-6-luna":    (0.10, 0.50)"#,
            r#""gpt-6.2-sol":   (2.00, 10.00)"#,
            r#""gpt-6.2-astra": (8.00, 40.00)"#,
            "m, 900, 3500, 6000",
            "m, 900, 5000, 5000",
            r#""gpt-6.2-sol", 1800, 6000, 3000"#,
            r#""gpt-6.2-astra", 650, 6000, 3000"#,
            "m, 200, 4000, 4000",
            "m, 200, 4000, 5000",
            "PILOT, RESERVE = 0.05, 0.05",
        ] {
            assert!(
                py.contains(needle),
                "budget.py no longer contains {needle:?}"
            );
        }
    }
}
