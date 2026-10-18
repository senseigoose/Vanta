# SPRT testing

Every strength-related change must beat the previous version here before it is kept.

## One-time setup (Codespace / Linux)
```bash
git clone https://github.com/Disservin/fastchess.git /tmp/fastchess
cd /tmp/fastchess && make -j2
sudo cp fastchess /usr/local/bin/      # or add /tmp/fastchess to PATH
fastchess --help | head -50            # flag names differ slightly between versions
```
If `run.sh` errors on a flag, fix it against `fastchess --help`; the engine side
(UCI, `option Hash`) is not the problem.

Opening book (balanced random 8-ply positions):
```bash
cargo run --release -p uci -- genbook 2000 8 1 > tools/openings/book8.epd
```

## Running a test
```bash
git tag v0.1 <commit-of-last-good-version>     # once per baseline
tools/sprt/run.sh v0.1 0 5                      # new working copy vs v0.1, H0=0 Elo, H1=5 Elo
```
- Use `[0, 5]` for small improvements, `[-3, 1]`-style bounds for "must not lose Elo" cleanups.
- A run ends at H1 accepted (keep), H0 accepted (revert), or the round cap (inconclusive).
- 2 cores means `CONC=2`; the machine, TC and book must stay the same between baseline and candidate.
- Record each result in `docs/results.md`: commit, baseline, bounds, outcome, Elo estimate.
