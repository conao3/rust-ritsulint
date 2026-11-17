# AGENTS.md

## Layout

- `lean/Ritsulint/Core.lean`: the rules that read a provision (kanji numerals, read-as clauses, references). `lean/Ritsulint/Api.lean`: the functions Rust calls. `lean/Ritsulint/Test.lean`: kernel-checked statements about `Core` on real provisions.
- `src/generated/`: Rust generated from `Ritsulint.Api` by leanfran. Regenerate it with `make generate` and commit it with the Lean change.
- `src/law.rs`: e-Gov law XML to articles and paragraphs. `src/read_as.rs`: the `read-as-quote` rule. `src/main.rs`: the CLI.
- `site/`: the page at https://ritsulint.conao3.com/ that lists the warnings (React 19, react-aria-components, Tailwind CSS v4, built with Vite+ and served as static assets of a Cloudflare Worker). `site/data/warnings.json` is written by `make site-data`.
- `tests/corpus-expected.txt`: law IDs that `make corpus-check` requires a high-confidence warning for. Each was confirmed by reading the provisions. `tests/corpus-rejected.txt`: law IDs that must have no high-confidence warning; each once gave a warning for a clause that names another law or quotes an item number.

## Lean and Rust

- Lean decides what a paragraph says: which clauses it has, which provisions each names, whether a clause is outside what the linter resolves. Add or change such rules in `Core.lean`, with a statement in `Test.lean` for the provision that motivated the change.
- Rust does input and output, looks provisions up in a law, compares text and computes the nearest text, the similarity and the warning ID.
- The statements in `Test.lean` are checked by the kernel (`decide`, `decide +kernel`). They are checks on given provisions, not theorems about every input.

## Verification

- `nix develop --command make check`: treefmt, clippy, `lake build` (the statements in `Test.lean`), `cargo test`.
- `nix develop --command make check-generated`: regenerates `src/generated/` and fails on a difference.
- `nix build`: the package `nix run` starts.
- `nix develop --command make corpus-check CORPUS=<dir> AS_OF=<YYYYMMDD>`: the full corpus. Run it after changing `Core.lean` or `read_as.rs`. The corpus is the unpacked e-Gov bulk download and is not in the repository.

- `nix develop --command make site-check`: format, lint, type check and build of `site/`.
- `nix develop --command make site-data CORPUS=<dir> AS_OF=<YYYYMMDD>` then `make site-deploy`: refresh the warnings and publish the site.

## Conventions

- A rule gives no warning when it cannot tell which provision a clause names. Count the clause in `External` or `Unsupported`.
- The warning ID excludes the revision ID and positions in the text, so that outputs for two revisions can be compared by ID.
- Messages and documents call a warning a mismatch in the e-Gov data. Whether the law itself is wrong is settled by the amending law in the official gazette.
- The CLI follows the AWS CLI form: `<command> <subcommand> --parameter value`, no positional arguments, PascalCase keys in JSON.
- Commit messages, comments, test names and documents are in English.
