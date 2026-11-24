#!/usr/bin/env python3
"""Re-run budget for 'The Oracle-Trust Gap' (OpenAI Researcher Access Program).
Edit PRICES on submission day; token volumes per call are the commitment.
Prices: USD per 1M tokens, Standard tier, from developers.openai.com/api/docs/models (fetched 2026-11-24).
No cached-input, Batch or background discounts are applied (conservative). Output includes reasoning tokens."""
PRICES = {  # model: (input, output)
    "gpt-6-luna":    (0.10, 0.50),
    "gpt-6.2-sol":   (2.00, 10.00),
    "gpt-6.2-astra": (8.00, 40.00),
}
TIERS = list(PRICES)
# (stage, model, calls, input_tokens_per_call, output_tokens_per_call)
rows = []
for m in TIERS:
    rows.append(("Arm 1: implement 60 tasks x 3 prompts x 5 samples", m, 900, 3500, 6000))
for m in TIERS:
    rows.append(("Arm 2: self-written test suite per implementation", m, 900, 5000, 5000))
# Cross-tier audits only (no model audits its own code)
rows.append(("Arm 2: audit of luna + astra code by sol (all 1,800)", "gpt-6.2-sol", 1800, 6000, 3000))
rows.append(("Arm 2: audit of sol code by astra (stratified 650)", "gpt-6.2-astra", 650, 6000, 3000))
for m in TIERS:
    rows.append(("Arm 3: characterization tests, 40 routines x 5", m, 200, 4000, 4000))
    rows.append(("Arm 3: C-to-Rust migration, 40 routines x 5", m, 200, 4000, 5000))
PILOT, RESERVE = 0.05, 0.05

def cost(m, calls, i, o):
    pi, po = PRICES[m]
    return calls * (i * pi + o * po) / 1e6

if __name__ == "__main__":
    tot_c = tot_i = tot_o = sub = 0.0
    print(f"{'Stage':52} {'Model':14} {'Calls':>6} {'In M':>7} {'Out M':>7} {'USD':>9}")
    for s, m, c, i, o in rows:
        usd = cost(m, c, i, o); sub += usd; tot_c += c; tot_i += c*i/1e6; tot_o += c*o/1e6
        print(f"{s:52} {m:14} {c:6d} {c*i/1e6:7.2f} {c*o/1e6:7.2f} {usd:9.2f}")
    print(f"{'Subtotal':52} {'':14} {int(tot_c):6d} {tot_i:7.2f} {tot_o:7.2f} {sub:9.2f}")
    p, r = sub*PILOT, sub*RESERVE
    print(f"{'Pilot and prompt development (5%)':52} {'':14} {'':6} {'':7} {'':7} {p:9.2f}")
    print(f"{'Rerun reserve (5%)':52} {'':14} {'':6} {'':7} {'':7} {r:9.2f}")
    total = sub + p + r
    print(f"{'TOTAL':52} {'':14} {'':6} {'':7} {'':7} {total:9.2f}")
    by = {}
    for s, m, c, i, o in rows: by[m] = by.get(m, 0) + cost(m, c, i, o)
    print("By model:", {k: round(v, 2) for k, v in by.items()})
    assert 950 <= total <= 1000, f"total {total:.2f} outside 950-1000: adjust volumes"
