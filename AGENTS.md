# AGENTS.md

## Layout

- `lean/Ritsulint/Core.lean`: the rules that read a provision (kanji numerals, read-as clauses, references). `lean/Ritsulint/Api.lean`: the functions Rust calls. `lean/Ritsulint/Test.lean`: kernel-checked statements about `Core` on real provisions.
- `lean/Ritsulint/Properties.lean`: theorems about the functions of `Core` that the crate runs, for every input. `audit/`: a Lake package that runs lean-assay on those functions against the theorems.
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
- `Properties.lean` proves, for every input: `parseKansuji` and `kansujiOr0` read every number from 1 to 9,999 back from the numeral `toKansuji` writes (`parseKansuji_toKansuji`, `kansujiOr0_toKansuji`); `unquote` keeps a text without `「` as it is and reads a quotation as one `〓` (`unquote_plain`, `unquote_quotation`); `stripParens` keeps a text without `（` as it is and drops a parenthesized remark with its parentheses (`stripParens_plain`, `stripParens_remark`). `toKansuji` is a writer defined in that file from the digits of a number, apart from the reader.
- A theorem counts when it rules out a broken implementation. `audit/Audit/Core.lean` runs `#assay` of [lean-assay](https://github.com/conao3/lean-assay) on `kansujiStep`, `parseKansuji`, `kansujiOr0`, `unquoteAux`, `unquote`, `stripParensAux` and `stripParens`: it breaks each function in small ways and runs the theorems on the broken versions. `lake build --wfail` in `audit/` fails when no broken version of a function is refuted, when a broken version satisfies every theorem but changes the output, or when no input satisfies a theorem's hypotheses. Of 128 broken versions 83 are refuted, 11 give the same output on every input tried and 34 do not typecheck. A theorem about a function of `Core` goes into `Properties.lean` with an `#assay … using …` line for it.
- `analyze` and `relativeRefs` are outside the audit: no theorem states a property of them, and their outputs are checked on the provisions of `Test.lean`. The definitions of `ReadAs.lean` are stated in Lean only; the crate compares the quoted words with a provision in Rust (`src/read_as.rs`).

## Verification

- `nix develop --command make check`: treefmt, clippy, `lake build` (the statements in `Test.lean` and the theorems), `lake build --wfail` in `audit/` (the audit of the theorems with lean-assay; Lake fetches lean-assay and plausible at the revisions of `audit/lake-manifest.json`), `cargo test`.
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
