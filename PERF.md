# Performance ledger

Numbers come from `librus.log`, which every run writes next to the executable.
`[perf]` lines are wall-clock milliseconds since process start (`src/perf.rs`).

Measure with the same conditions: same binary profile, warm session (a valid
token in `session.json`), same network. Network variance on the Librus API is
large (the slowest metadata call measured 540–940 ms across runs), so treat
sub-100 ms deltas as noise.

## Baseline (debug build, expired token)

| Milestone | Time |
|---|---|
| window created | 18 ms |
| entering event loop | 37 ms |
| metadata ready | 1913 ms |
| all data ready | 3967 ms |
| HTTP requests per refresh | 18 |

Startup (window on screen) was never the problem: 37 ms. Everything the user
waits for is the refresh.

## Attempts

| Idea | Baseline → Result | Verdict | Why |
|---|---|---|---|
| Seven metadata lookups in parallel (`tokio::join!` instead of seven `await`s) | metadata 1913 → 818 ms | kept | Pure serialisation; each request was independent. |
| Persist the refreshed token to `session.json` | restart after 6 h: −1000 ms before the first request | kept | `ensure_token` refreshed in memory only, so every start paid a full OAuth round trip. |
| `[profile.dev] opt-level=1` + deps at 3 | fetch tail −200 ms | kept | Unoptimized `serde`/`scraper` parsing dominated the tail. Dev builds stay quick for our own crate. |
| `tokio::spawn` the nine fetches instead of `join!` | all data 2767 → 2566 ms | kept | `join!` polls every future on one task, so their CPU work ran one after another. |
| Release `opt-level="z"` → 3 | not measured | not done | Needs a full LTO build to measure; network now dominates the refresh. Revisit if parsing shows up in a profile. |
| Request-level response cache (dedupe the two duplicate `Timetables`/`HomeWorks` calls) | not measured | not done | Two concurrent tasks both miss the cache, so it would need in-flight coalescing to buy anything. |

## Lock convoy (the big one)

`fetch_attendances_data` and `fetch_jakdojade_commute` held `state.subjects`
(and friends) **across network requests**. Four fetches finished within 2 ms of
each other — the signature of everything queueing behind one holder.

Fixed by cloning the maps out of the lock (they hold a few dozen entries):

| Fetch | Before | After |
|---|---|---|
| grades | 1966 ms | 100 ms |
| attendance | 2050 ms | 242 ms |
| events | 2048 ms | 292 ms |
| timetable | 2052 ms | 616 ms |
| **all data ready** | **3967 ms** | **2693 ms** |
| **dashboard populated** (metadata + dashboard) | **2123 ms** | **941 ms** |

## Current shape

The remaining time is the Librus API itself: `Grades/Categories` (~600 ms for a
64 KB payload) sets the metadata floor, and the commute lookup (Jakdożdż + two
timetable requests) is the last thing to land at ~1.9 s. Neither is our code.

Watch `[perf]` in `librus.log` for regressions: `all data ready` should stay
under ~3 s, and any new `[perf] slow request` line names the endpoint to blame.
