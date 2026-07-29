# codepack

**Turn a source folder into a clean, safe snapshot you can hand to anyone.**

Point it at a project and it produces an archive plus a set of reports fit to give to a
colleague, a new joiner, a non-technical stakeholder, or a language model. The point is
not compression — it is **not leaking your secrets** when a project leaves your machine:
API keys, tokens, passwords, `.env` files.

Everything runs **locally**. Nothing is uploaded, ever.

---

## Contents

- [What you get](#what-you-get)
- [Install](#install)
- [Quick start](#quick-start)
- [Commands](#commands)
- [Archive formats](#archive-formats)
- [Sterile copy](#sterile-copy)
- [Pre-commit use](#pre-commit-use)
- [Guarantees](#guarantees)
- [Documentation](#documentation)
- [Developing](#developing)

---

## What you get

Two ways to use it, both over the same engine — neither is a wrapper around the other.

**Desktop app** (Windows 10/11). An export wizard, a preview tree where you can override
what goes in and what stays out, a results panel, run history, folder watching, light and
dark themes, English and Russian interfaces switchable without a restart, and a tray icon.

**Command line** — the `codepack` binary. Nine commands, a stable exit-code contract, and
`--json` on everything.

A finished export contains the selected source files, a directory structure report, git
history and optionally a patch, a full text dump, roughly thirty analysis reports, an
`AI_CONTEXT` folder written for a language model, a security scan (including SARIF), and
a manifest describing all of it.

> **Screenshots.** Not included yet. They were attempted for this release and abandoned:
> capturing a window on Windows grabs whatever is actually on top of that screen region,
> which risks putting unrelated private content into a public repository. Add them by
> running the app and capturing its window deliberately.

## Install

Build the installer from the repository:

```bash
cargo xtask package
```

The NSIS `.exe` lands in `target/release/bundle/nsis/`.

The build is **not code-signed yet**, so Windows SmartScreen will warn about an unknown
publisher. Signing, notarisation, checksums and auto-update are planned but not done.

For the command line only:

```bash
cargo build --release -p codepack-cli
```

## Quick start

See what an export would include, without writing anything:

```bash
codepack preview .
```

Then produce the bundle:

```bash
codepack export . --preset "Claude Code" --out ../out
```

Six presets ship: `Claude Code`, `ChatGPT`, `Code Review`, `Security Audit`, `Онбординг`
(onboarding), and `PR Review` — the last narrowing the export to uncommitted changes, for
discussing one pull request rather than a whole project.

A token budget can be a number or a **model name**, resolved through a built-in table you
can extend with your own file — no rebuild needed:

```bash
codepack export . --budget Claude
codepack export . --budget 200k
```

If a file is missing from the bundle and you cannot see why:

```bash
codepack explain src/main.rs
```

It answers with one of four verdicts — included, excluded (naming the rule), not in the
diff selection, or not planned at all (naming the skipped directory) — and all four are a
success, because "it was excluded, here is why" is the explanation working.

## Commands

| Command | What it does |
|---|---|
| `export` | The full pipeline: plan → copy → structure → git → text dump → analytics → manifest → archive |
| `preview` | What an export would include. Writes **nothing** |
| `scan` | Find secrets and risky code. `--staged` is the pre-commit mode |
| `verify` | Re-scan a bundle that already exists — the only check its recipient can run |
| `explain <file>` | Why one file did or did not make it into the export |
| `sanitize` | Sterile copy: code with comments stripped and reformatted, optionally archived |
| `history` | Previous runs |
| `doctor` | Environment diagnostics |
| `completions <shell>` | A shell completion script |

**Exit codes are a contract** you can build a pipeline on: `0` success, `1` error,
`2` bad arguments, `3` critical secrets found. A real failure always outranks "secrets
found" — a run that broke never reports `3`, because that would tell your pipeline the
scan result can be trusted when it cannot.

**`--json`** works on every command. It carries a schema version, goes only to stdout, and
leaves progress and errors on stderr, so `codepack export --json | jq` does not break on
the first log line.

## Archive formats

ZIP by default, everywhere. 7z is available when you want smaller archives. RAR is offered
in the interface but **not implemented** — there is no permissively licensed RAR encoder to
depend on, so it is reserved and refused with a message rather than silently producing a
ZIP with the wrong extension.

```bash
codepack export . --archive-format 7z
codepack sanitize --source . --archive ../clean.zip
```

For `sanitize`, the file extension picks the container; `--archive-format` overrides it.

## Sterile copy

A standalone action, not a step of the export: it builds a copy of the project with
comments removed by real parsers (tree-sitter, never regular expressions) and the code
reformatted wherever a suitable formatter is found on your `PATH`.

```bash
codepack sanitize --source . --archive ../project-sterile.zip
```

`--out` is only needed if you also want the folder. With just `--archive`, the copy is made
in a temporary directory that cleans itself up, and one file is the whole result. You can
ask for both:

```bash
codepack sanitize --source . --out ../sterile --archive ../project-sterile.zip
```

The archive contains exactly the files this run produced, plus
`STERILE_COPY_REPORT.json`/`.md` — so whoever receives it gets the account of what was
stripped, skipped and redacted alongside the code it describes.

## Pre-commit use

```bash
codepack scan --staged
```

This reads content **from the git index**, not from your working tree — a commit is built
from the index, and the two diverge the moment a staged file is edited again.

Findings you have reviewed and accepted go in a `.codepack-allow` file beside the project.
The fingerprint is computed from the rule, the file, and the **already-redacted** message —
the secret itself is never an input. Suppressed findings are still counted and printed, so
they never disappear silently.

## Guarantees

These are held by tests, not by promises in this file:

- **Privacy is absolute.** No crate reaches the network except the explicit, user-initiated
  AI integration. A quality-gate step reads every manifest and fails the build otherwise.
- **The source is immutable.** An export never writes inside the folder it reads.
- **Secrets never reach output.** Not a report, a log, the database, or an archive — never
  in the clear.
- **Byte figures are preserved.** Token counts were added alongside, not instead.
- **Symlinks are never followed**, and extraction is path-traversal safe.

The full registry, with the reasoning behind each one, is in
[docs/architecture/invariants.md](docs/architecture/invariants.md).

## Documentation

| Document | What it answers |
|---|---|
| [docs/architecture/overview.md](docs/architecture/overview.md) | What is actually built, and how the pieces fit |
| [docs/architecture/invariants.md](docs/architecture/invariants.md) | What must never break, and why |

## Developing

```bash
python dev_tools_scripts_runner.py list   # the catalogue of routine jobs
cargo xtask gate                          # the full quality gate
cargo xtask fmt                           # format Rust and the frontend
pnpm desktop:dev                          # run the app with hot reload
```

`cargo xtask gate` is the check that must pass: formatting, clippy with warnings denied,
tests, dependency audit, frontend checks, the dev-script suite, agent-rule sync, and
network isolation.

Target platform is **Windows 10/11**. macOS and Linux remain a goal; the disabled code is
marked rather than deleted.
