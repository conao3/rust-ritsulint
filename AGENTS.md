# AGENTS.md

## Layout

- `lean/Ritsulint/Core.lean`: the rules that read a provision (kanji numerals, read-as clauses, references). `lean/Ritsulint/Api.lean`: the functions Rust calls. `lean/Ritsulint/Test.lean`: kernel-checked statements about `Core` on real provisions.
- `lean/Ritsulint/ReadAs.lean`: what a read-as clause does to a text, and the theorem that a clause whose quoted words are absent changes nothing. `lean/Ritsulint/Findings.lean`: one theorem for each high-confidence `read-as-quote` warning, written by `make findings` (`src/findings.rs`) and checked by `lake build`.
- `src/generated/`: Rust generated from `Ritsulint.Api` by leanfran. Regenerate it with `make generate` and commit it with the Lean change.
- `src/law.rs`: e-Gov law XML to articles, paragraphs, items and the table of contents. `src/main.rs`: the CLI.
- Rules: `src/read_as.rs` (`read-as-quote`), `src/external.rs` (`read-as-quote-external`, needs the whole corpus), `src/relative_ref.rs` (`relative-ref`, `missing-unit`), `src/structure.rs` (`numbering`, `toc-range`), `src/text.rs` (`referenced-term`, `brackets`, `spelling`), `src/terms.rs` (`unused-definition`, `near-term`, `definition-scope`). `src/history.rs`: since which revision a warning is given.
- `site/`: the page at https://ritsulint.conao3.com/ that lists the warnings and keeps the state of its controls in the query string (`c`, `rule`, `kind`, `q`) (React 19, react-aria-components, Tailwind CSS v4, built with Vite+ and served as static assets of a Cloudflare Worker). `site/data/warnings.json` is written by `make site-data`.
- `tests/corpus-expected.txt`: `<rule> <law ID>` lines that `make corpus-check` requires a high-confidence warning for. The `read-as-quote` and `relative-ref` lines were confirmed by reading the provisions; the `missing-unit`, `numbering` and `toc-range` lines by reading the list of warnings. `tests/corpus-rejected.txt`: lines that must have no high-confidence warning; each once gave a warning for a clause that names another law, quotes an item number, or stands in a table or a form.

## Lean and Rust

- Lean decides what a paragraph says: which clauses it has, which provisions each names, whether a clause is outside what the linter resolves. Add or change such rules in `Core.lean`, with a statement in `Test.lean` for the provision that motivated the change.
- `external.rs`, `text.rs` and `terms.rs` find their clauses, definitions and words with regular expressions and scans in Rust, and take numerals and `unquote` from Lean.
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
