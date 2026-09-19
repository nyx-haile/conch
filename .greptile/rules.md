# Review rules for nyx-haile/conch

Voice interview CLI (Rust + TypeScript, tests under tests/). Audio, provider keys and transcripts are handled locally: never commit keys or recorded content; the README's status claims must match what the code ships.

Findings should be few, demonstrated, and each carry file:line, a concrete failure scenario and the smallest fix. `nothing found` is a respectable review.

## Process
- Every change lands through a pull request into `core`; a PR targeting another branch should say why.
- The PR description must match the diff; flag claimed tests or verification the diff does not contain.

## Never in the repository
- Secrets: JWTs (`eyJ...`), `sk-`/`sk-ant-`/`sk-svcacct-` keys, `ghp_`/`github_pat_` tokens, `AKIA` keys, PEM blocks, `.env` contents, Supabase service-role keys.
- Private material: Personal health, medical or prescription detail; citizenship, residence, immigration and right-to-work status; equity, dilution, vesting, 83(b) and personal runway figures; named third parties' contact registers, email addresses, phone numbers, employers, ages or attributed private statements; verbatim private correspondence including routing headers; candid assessments of named individuals.
- Build artifacts (`target/`, `dist/`, `node_modules/`, `.venv/`, compiled binaries).

## Code
- New code paths need a test that exercises the failure case, not only the happy path.
- Shell scripts use `set -euo pipefail` and fail closed.
- Numbers and status claims in docs carry their derivation or are marked as estimates.
