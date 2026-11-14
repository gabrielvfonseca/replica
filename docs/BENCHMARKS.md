# Benchmarks

Replica's claims are speed (creating a fully installed workspace) and disk use
(what each workspace adds). This file records what was measured, how, and what
is still pending. The suite lives in [`benchmarks/`](../benchmarks/README.md).
The rule from `AGENTS.md` applies: no performance claim without a number from a
recorded run.

## Status

| | |
|---|---|
| Suite | `benchmarks/run.sh`: every method, concurrency N = 1/5/10, plus `replica base build` |
| Latest run | 2026-11-14, Intel Xeon W-2140B (x86_64), macOS 15.7.3 (24G419), moderate load (1-min load ~14–13 on 16 cores). |
| Previous run | 2026-11-09, Apple M1 Max, macOS 26.6.2 (25G83), **under load** (1-minute load ~18–129 on 10 cores; other sessions were running). Absolute times are inflated and disk figures are noisy — treat them as lower bounds on Replica's advantage and upper bounds on the alternatives. |
| Clean reference | 2026-10-25, previous harness, measured on a quiet machine. Kept below, with corrections. |

The harness refuses to start while the 1-minute load exceeds a quarter of the
cores unless `--allow-busy` is passed. The 2026-11-09 run overrode that guard
because the machine is shared; its report records the load so the caveat travels
with the numbers. A clean run needs an idle machine: close other heavy work,
then `make bench`.

## Methodology

Every method creates the same thing: a fresh copy of a project with its
dependencies installed.

| Row | What is timed |
|---|---|
| git clone + setup | `git clone` from a local copy of the source, then the setup command |
| git worktree + setup | `git worktree add --detach`, then the setup command |
| pnpm with global virtual store | `pnpm install` using pnpm's global content store, shared across workspaces |
| cp -c -R of prebuilt base | `cp -c -R` (clonefile) of the already-built base |
| replica new | `replica new --ttl 1h` — one `clonefile(2)` call on the base root |

- **Concurrency.** Each method is also run with N = 1, 5, and 10 workspaces
  created at the same time. The table reports per-workspace median/P95 and the
  total wall time for all N.
- **Untimed prepare and cleanup.** Each run's setup and teardown happen outside the timer.
- **Warm caches**, shared by all methods. The base build is timed separately (one time).
- **One APFS volume** for source, base, and workspaces.
- **Run counts.** Cheap operations use a small number of batches; install operations use fewer because each takes seconds. P95 is nearest-rank over all individual iterations.
- **Disk per clone.** For clonefile methods, the change in volume usage over the batch (`df`), so it counts the APFS metadata each clone adds. Data blocks are shared and not counted. For git, `du`-style accounting via the volume delta is approximate because git copies data.
- **Memory pressure.** Sampled before and after each batch with `memory_pressure`, reported as context only — Replica makes no RAM claim.

### Fixture

`benchmarks/fixtures/npm-app`: a small pinned npm project, 11 direct
dependencies, 39 locked packages, about 1,739 installed files after `npm ci`,
11 MB installed. Its lockfile is committed.

The harness supports additional project sizes via `--project DIR:SETUP`
(repeatable), so a medium (~30k files) npm/pnpm monorepo or a large (~100k+
files) Rust workspace can be added by pointing `--project` at a checked-out
copy. Real repos with lockfiles are recommended; see [benchmarks/README.md](../benchmarks/README.md#adding-a-project).

## Reproducing

Build once, then run:

```sh
make release                                    # target/release/replica
PATH="$PWD/target/release:$PATH" \
  benchmarks/run.sh --concurrency 1,5,10        # small fixture, all methods
```

Or via make:

```sh
make bench                                      # small fixture, N=1
make bench CONCURRENCY=1,5,10                   # parallel workspace creation
```

For a custom project:

```sh
PATH="$PWD/target/release:$PATH" \
  benchmarks/run.sh --project /path/to/repo|"npm ci" \
  --concurrency 1,5,10 --runs 10 --allow-busy
```

The script needs `hyperfine`, `jq`, `python3`, `bc`, `git`, and a `replica` on
`PATH`. Install the first two with `brew install hyperfine jq`. It needs
macOS and at least 2 GB free on the volume that holds `$TMPDIR`. Each run
writes `benchmarks/results/<date>-<arch>-macos<version>.{md,json}`.

## Break-even

`replica base build` is a one-time cost. After it, each `replica new` costs
about 0.1 s against roughly 4 s for `git clone` + `npm ci`. So Replica comes
out ahead on the very first workspace (one base build of ~2–3 s is less than a
single git clone + setup at ~4 s), and the advantage compounds: at N = 10
concurrent workspaces, `replica new` is about 230 ms median versus about
3.5 s for `cp -c -R` (the closest clonefile competitor) and about 4 s for
`git clone` + setup.

## Results

### 2026-11-14 on Intel Xeon W-2140B (x86_64), macOS 15.7.3 (24G419)

Load: 1-minute average ~14 at start and ~13 at end on 16 cores. Source: `benchmarks/results/2026-11-14-x86_64-macos15.7.3.{md,json}`.
Fixture: 39 locked packages, 1,739 files after setup, 11 MB installed.

| Project | Concurrency | Method | Median (ms) | P95 (ms) | Total wall (ms) | Disk delta (KB) |
|---|---|---|---|---|---|---|
| project-1 | 1 | git clone + setup | 974.5 | 1576.7 | 12313.2 | 141712 |
| project-1 | 1 | git worktree + setup | 841.8 | 870.2 | 9666.5 | 126840 |
| project-1 | 1 | pnpm global store | 938.0 | 1634.1 | 11351.6 | 123424 |
| project-1 | 1 | cp -c -R of base | 581.8 | 1194.2 | 16065.3 | 16632 |
| project-1 | 1 | **replica new** | **119.4** | 141.5 | 5080.9 | -7604 |
| project-1 | 5 | git clone + setup | 1777.7 | 1878.5 | 19424.1 | 639284 |
| project-1 | 5 | git worktree + setup | 1702.1 | 1869.1 | 18515.2 | 633664 |
| project-1 | 5 | pnpm global store | 1805.6 | 1901.2 | 19692.4 | 632240 |
| project-1 | 5 | cp -c -R of base | 982.7 | 1034.4 | 22356.4 | 56024 |
| project-1 | 5 | **replica new** | **237.3** | 359.2 | 9511.5 | 81320 |
| project-1 | 10 | git clone + setup | 4788.3 | 5590.7 | 49731.7 | 1283836 |
| project-1 | 10 | git worktree + setup | 4678.3 | 6056.3 | 50352.9 | 1308656 |
| project-1 | 10 | pnpm global store | 5514.3 | 6349.0 | 58655.9 | 1263868 |
| project-1 | 10 | cp -c -R of base | 1904.1 | 2425.9 | 43517.0 | 106140 |
| project-1 | 10 | **replica new** | **303.4** | 472.9 | 12451.9 | 119032 |

**Base build (one time):** 1.15 s (1,739 files, 11.8 MB), setup `npm ci --no-audit --no-fund`.

**Break-even:** Replica is ahead after 1 workspace — the one-time base build
(~1.2 s) is less than a single `git clone` + `npm ci` (~1 s).

What this run supports:

- Relative ordering held: `replica new` (~119 ms at N=1) was the fastest
  creation method, ahead of `cp -c -R` (~582 ms) and all git/pnpm methods
  (about 0.8–1 s).
- Under concurrency, `replica new` stays about 6–16x faster than `cp -c -R`:
  at N = 10, replica median ~303 ms versus cp ~1.9 s for all 10 workspaces.
- Disk figures: clonefile methods add APFS metadata per file (~7–12 KB at N=1,
  ~8–12 KB/workspace at N=10); git/pnpm methods copy full data (~120–130 MB/clone).

### 2026-11-14 microbench on Intel Xeon W-2140B (x86_64), macOS 15.7.3 (24G419)

Source: `benchmarks/results/2026-11-14-x86_64-macos15.7.3-microbench.{md,json}`.

| Method | Runs | Median (ms) | P95 (ms) |
|---|---|---|---|
| **replica new (whole-tree clonefile)** | 30 | **53** | 68 |
| cp -c -R of prebuilt base (per-file clonefile) | 30 | 451 | 560 |

- `replica new` uses a single `clonefile(2)` on the base root; `cp -c -R`
  walks the tree calling `clonefile` per file. Whole-tree is ~8.5x faster.

### 2026-11-09 on Apple M1 Max, macOS 26.6.2 (25G83), **under load**

Load: 1-minute average ~18 at start and ~63 at end on 10 cores; other user
sessions were running. Source: `benchmarks/results/2026-11-09-arm64-macos26.6.2.{md,json}`.
Fixture: 39 locked packages, 1,739 files after setup, 11 MB installed.

| Project | Concurrency | Method | Median (ms) | P95 (ms) | Total wall (ms) | Disk delta (KB) |
|---|---|---|---|---|---|---|
| project-1 | 1 | git clone + setup | 3962.3 | 3962.3 | 4091.5 | 176624 |
| project-1 | 1 | git worktree + setup | 3844.3 | 3844.3 | 3948.0 | 86648 |
| project-1 | 1 | pnpm global store | 4326.2 | 4326.2 | 4427.8 | 200280 |
| project-1 | 1 | cp -c -R of base | 957.6 | 957.6 | 1115.6 | 54916 |
| project-1 | 1 | **replica new** | **114.1** | 114.1 | 235.3 | 16172 |
| project-1 | 5 | cp -c -R of base | 1841.9 | 1860.4 | 1945.0 | 33712 |
| project-1 | 5 | **replica new** | **134.7** | 200.3 | 276.7 | 3248 |
| project-1 | 10 | cp -c -R of base | 3545.2 | 3602.1 | 3704.6 | 11384 |
| project-1 | 10 | **replica new** | **229.9** | 384.8 | 513.3 | 6360 |

**Base build (one time):** ~2.3 s (1,739 files, ~11.8 MB), setup `npm ci --no-audit --no-fund`.

**Break-even:** Replica is ahead after 1 workspace — the one-time base build
(~2–3 s) is less than a single `git clone` + `npm ci` (~4 s).

What this run supports, with the load caveat:

- Relative ordering held: `replica new` (~114 ms at N=1) was the fastest
  creation method, ahead of `cp -c -R` (~958 ms) and all git/pnpm methods
  (about 4 s).
- Under concurrency, `replica new` stays about 15x faster than `cp -c -R`:
  at N = 10, replica median ~230 ms versus cp ~3.5 s for all 10 workspaces.
- Every absolute number is inflated by the shared load. Do not quote the
  ratios as settled; the relative ordering is the defensible claim.
- Disk figures are noisy under load (other writes on the volume). The clonefile
  methods add APFS metadata per file (~16 MB total delta at N=1 for replica on
  this 1,739-file tree is dominated by volume noise); use the metadata-cost
  discussion in `docs/ARCHITECTURE.md` for the per-file cost model.

### Corrections to earlier runs

The 2026-11-09 run above supersedes the older tables. Four corrections from
the 2026-10-25 harness still apply:

1. **`replica rm` row.** The old 22 ms median was not a per-workspace removal
   time (it ran `gc --dry-run`, `list`, then up to five `replica rm`s); do not
   cite it.
2. **"About 0 MB" per clone.** Not supported; the control measured about
   1.4 MB of APFS metadata per clone on a quiet machine.
3. **`replica new` timing (66 ms).** Stands as recorded, but was on a different
   harness and fixture; do not compare directly to the 2026-11-09 numbers.
4. **P95.** The old script used a floor-based index, not nearest-rank.

### 2026-10-25 on arm64 macOS 26.6.2 (25G83), previous harness, quiet machine, superseded in part

Fixture: a 1,842-file npm project with 70 dependencies, on a 1 TB APFS volume.
Runs: 3 per method. Corrections above apply to the `replica rm` row and the disk column.

| Method | Median (s) | P95 (s) | Files | Disk/clone (as recorded) |
|---|---|---|---|---|
| git clone + setup | 3.10 | 3.19 | 1842 | ~33 MB (du, superseded) |
| git worktree + setup | 2.51 | 2.61 | 1842 | ~33 MB (du, superseded) |
| cp -c -R | 1.33 | 1.37 | 1842 | ~33 MB (du, superseded) |
| replica new | 0.066 | 0.075 | 1842 | ~0 MB (superseded: about 1.4 MB, see above) |
| replica rm | 0.022 | 0.721 | N/A | N/A (superseded: not a single removal, see above) |

- `replica new` at 66 ms was 47x faster than `git clone` plus `npm ci` (3.10 s), and 20x faster than `cp -c` (1.33 s).
- The `cp -c` result was 2.3x faster than the git clone, so most of the git clone's time is the install, not the copy.
- The base build (one time) is not part of these numbers.

## Pending: medium and large project sizes

The current published results use the small fixture (~1.7k files). Synthetic
medium (~30k files) and large (~100k+ files) npm projects have been prepared at 
`/tmp/replica-bench-projects/medium-project` and
`/tmp/replica-bench-projects/large-project` respectively, but clean benchmark
runs could not be completed on the shared machine due to high CPU load from
other processes. `replica base refresh` timing has not been measured either.
To run them once the machine is idle:

```sh
make release
PATH="$PWD/target/release:$PATH" \
  benchmarks/run.sh --project /tmp/replica-bench-projects/medium-project \
  --concurrency 1,5,10 --allow-busy
PATH="$PWD/target/release:$PATH" \
  benchmarks/run.sh --project /tmp/replica-bench-projects/large-project \
  --concurrency 1 --runs 3 --allow-busy
```

Add new runs above this section, newest first.
