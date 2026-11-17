# ritsulint

A linter for Japanese statutes. It reads the law XML that [e-Gov](https://laws.e-gov.go.jp/) publishes and reports places where the text of a law no longer agrees with itself.

Japanese law refers to its own provisions by position (`第七条第一項`) and by copying their words. A read-as clause such as `第七条第一項中「各号の一に該当するとき」とあるのは「…」と読み替える` quotes the words of article 7. When an amendment rewrites article 7 and leaves the clause as it was, the clause quotes words that are no longer there. ritsulint finds such clauses.

The warnings describe the consolidated text in the e-Gov data. Whether a warning is an error of the law itself is settled by the amending law as promulgated in the official gazette.

## Rules

| Rule | Reports |
|---|---|
| `read-as-quote` | A read-as clause whose quoted words are missing from the provision the clause names |

`Status` tells how the quotation fails:

| Status | Meaning |
|---|---|
| `absent` | The article exists and does not contain the quoted words |
| `other-paragraph` | The quoted words are in another paragraph of the article |
| `no-paragraph` | The article has no paragraph with that number |
| `no-article` | The law has no article with that number |

`Confidence` tells how sure the linter is of the provision the clause names:

- `high`: the clause names the article by number (`第七条第一項中`) and the law has that article.
- `medium`: the clause names the provision relative to another (`同項中`, `前条中`) or takes it over from the clause before it, where a wrong resolution of the earlier reference carries over; or the law has no article with the number, which usually means the clause refers to another law.
- `low`: words naming another law stand before the reference and the nearest text of the target is unlike the quotation; or the text before the clause names the same article number as an article of another law (`犯罪捜査規範第十六条から第十九条までの規定を準用する。この場合において、第十六条中…`), which also lowers the clauses after it in the paragraph; or several clauses of one paragraph name articles the law does not have. Such clauses usually refer to another law.

A clause that names a provision of another law, a provision that another read-as has already changed (`…において読み替えて準用する第…条中`), a range (`第…条から第…条まで`) or `これらの規定` is counted in `External` or `Unsupported` and gives no warning. A paragraph that gives its read-as as a table (`読み替えられる字句`, `…欄に掲げる字句`) is counted in `TableParagraphs` and skipped. Supplementary provisions are not read.

## Usage

```sh
nix run github:conao3/rust-ritsulint -- law check-law --xml-file path/to/law.xml
nix run github:conao3/rust-ritsulint -- corpus check-corpus --xml-dir path/to/xml --as-of 20261117
```

`corpus check-corpus` reads the directory the e-Gov [bulk download](https://laws.e-gov.go.jp/bulkdownload/) unpacks (`<law id>_<enforcement date>_<amendment id>/<same name>.xml`) and checks, of each law, the latest revision enforced on or before `--as-of`.

- `--output json|text|table` selects the format (default `json`). Keys are in PascalCase.
- `--fail-on-warnings` makes the exit code 1 when there is a warning.
- `--debug` adds `Clauses` to the JSON: every read-as clause with its source, the words before 中, the resolved provisions, the quoted words, the replacement and the result (`present`, `absent`, `other-paragraph`, `no-paragraph`, `no-article`, `external`, `unsupported`).
- Exit codes: 0 success, 1 warnings found with `--fail-on-warnings`, 252 wrong command line, 254 unreadable input.

Each warning has an `Id` computed from the law, the rule, the paragraph the clause stands in, the words before `中`, the provisions they resolve to and the quoted words. The revision is not part of it, so the warnings an amendment introduced are the IDs present in the output for the later revision and absent from the earlier one. The e-Gov API serves the XML of each revision (`https://laws.e-gov.go.jp/api/2/law_file/xml/<revision id>`); the bulk download holds the current and the not yet enforced revisions only.

```sh
ritsulint law check-law --xml-file 326CO0000000319_20240401_505AC0000000063.xml | jq -r '.Warnings[].Id' | sort > before
ritsulint law check-law --xml-file 326CO0000000319_20240610_505AC0000000056.xml | jq -r '.Warnings[].Id' | sort > after
comm -13 before after
```

For the Immigration Control Act this prints one ID: the amendment enforced on 2024-06-10 rewrote `但し` in article 41 as `ただし` and left the quotation in article 13 paragraph 7 as it was.

## How it is built

The rules that read a provision are written in Lean 4 (`lean/Ritsulint/Core.lean`): kanji numerals, splitting a paragraph into read-as clauses, and resolving the words before `中` to provisions. `lean/Ritsulint/Test.lean` states what these functions return on provisions as they are written, and the Lean kernel checks each statement when the package is built. [leanfran](https://github.com/conao3/lean-leanfran) compiles the Lean code to the Rust module in `src/generated/`.

Rust reads the XML, looks up the provisions, compares the text and prints the report.

## Development

```sh
nix develop --command make check
nix develop --command make check-generated
nix build
nix develop --command make corpus-check CORPUS=path/to/xml AS_OF=20261117
```

`make corpus-check` runs the linter over a local copy of the bulk download and requires a high-confidence warning for every law listed in `tests/corpus-expected.txt`.

## License

Apache-2.0
