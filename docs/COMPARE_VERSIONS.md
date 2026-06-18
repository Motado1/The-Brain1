# A/B compare — `main` vs the other chat's branch

Two branches built overlapping features. Boot each, run the checklist, then tell me which you prefer
per row of the scorecard. I'll consolidate the winners onto `main`.

> First compile of each version takes ~1–3 min (Rust recompiles on branch switch). Same `brain.db`.

---

## VERSION A — `main` (this chat: verified organic look + pulse rhythm)

```powershell
cd C:\Users\mccad\The-Brain1
git fetch origin
git checkout -B view-main origin/main
cd engine
cargo run -p nbe_app --release -- --db brain.db
```

**Verify (look for):**
- [ ] **Connectors** taper like tree branches — thick where they root into a soma, easing thinner
      outward, **no thread-thin middle**.
- [ ] **Connectors attach to the membrane shell** and flow out from it — they do NOT stab into the
      glowing core.
- [ ] **Soma** = granular/bumpy body; a **glowing core enclosed inside a translucent membrane**.
- [ ] **Colour** = deep red-orange.
- [ ] **Pulse** = a small tight bead of light glides along a connection at a calm speed; on arrival
      it's absorbed, **pauses ~2s, then a reply heads back** (no rapid ping-pong).
- [ ] **Dendrite surge** = when a neuron fires, light runs out through its dendrite branches root→tip.
- [ ] *(No search, no fly-to, no zoom-out detail culling in this version.)*

---

## VERSION B — `session-frontier-continue-jap19w` (other chat: UI + search + LOD)

```powershell
cd C:\Users\mccad\The-Brain1
git fetch origin
git checkout -B view-ui origin/claude/session-frontier-continue-jap19w
cd engine
cargo run -p nbe_app --release -- --db brain.db
```

**Verify (look for):**
- [ ] Press **Ctrl+P** (or Cmd+K) → a **fuzzy search overlay** opens; type a client/note name →
      it **flies you there with a smooth cinematic glide**.
- [ ] **Sidebar UI** — client list w/ renewal dates, research notes as fly-to buttons, "Add Research".
- [ ] **Glass theme** — translucent dark panels.
- [ ] **Zoom way out**: distant neurons stay as **stable specks** (don't flicker/vanish); **zoom in**:
      dendrite detail tiers back in.
- [ ] Their **connectors/soma** look (fatter, deeper-rooted — the older pass, before your taper +
      membrane tuning).
- [ ] Their **pulses** (smooth, continuous, ease-out at the node) and **dendrite surge**.

---

## Scorecard — tell me your pick per row

| # | Feature | A (`main`) | B (other) | Your pick |
|---|---------|-----------|-----------|-----------|
| 1 | Connector shape | branch taper, attach to membrane | fatter, deep-rooted | |
| 2 | Soma look | membrane + enclosed core (tuned) | older pass | |
| 3 | Colour / red | deep red-orange | their tint | |
| 4 | Pulse feel | tight bead + 2s cooldown rhythm | smooth ease-out | |
| 5 | Dendrite surge | uv root→tip | their travel+fade | |
| 6 | UI / search / fly-to | — (not built) | **only B** — keep? | |
| 7 | LOD zoom (specks + tiers) | — (not built) | **only B** — keep? | |
| 8 | Motes / atmosphere | main's | per-network colour | |

Rows 1–5 + 8 are "which look/feel do you like." Rows 6–7 are "do you want this feature kept" (only B
has it). Once you give me the picks, I rebuild a single clean `main` with the winners.

---

## Back to main when you're done
```powershell
git checkout -B view-main origin/main
```
