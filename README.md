# ritsulint

A linter for Japanese statutes. It reads the law XML that [e-Gov](https://laws.e-gov.go.jp/) publishes and reports places where the text of a law no longer agrees with itself.

Japanese law refers to its own provisions by position (`第七条第一項`) and by copying their words. A read-as clause such as `第七条第一項中「各号の一に該当するとき」とあるのは「…」と読み替える` quotes the words of article 7. When an amendment rewrites article 7 and leaves the clause as it was, the clause quotes words that are no longer there. ritsulint finds such clauses.

The warnings describe the consolidated text in the e-Gov data. Whether a warning is an error of the law itself is settled by the amending law as promulgated in the official gazette.

## Rules

| Rule | Reports |
|---|---|
| `read-as-quote` | A read-as clause whose quoted words are missing from the provision of the same law the clause names |
| `read-as-quote-external` | A read-as clause whose quoted words are missing from the provision of another law the clause names (`corpus check-corpus` only) |
| `relative-ref` | A reference by position (`前条第二項`, `次項`, `前三号`) to a paragraph or an item the law does not have |
| `missing-unit` | A reference that lacks its unit (`第九十八第二項` for `第九十八条第二項`) |
| `numbering` | A paragraph or item number that occurs twice in its article or paragraph, or an item number that skips one |
| `toc-range` | A range of the table of contents (`（第三条―第七条）`) that leaves out an article of the chapter or section it stands for |
| `referenced-term` | `前項に規定するX` or `第N項に規定するX` where the paragraph named has no `X` (for `X` of five characters or more) |
| `brackets` | Quotation marks or parentheses that do not pair within a paragraph |
| `spelling` | A katakana word written with both a small and a large kana (`インターネット`, `インターネツト`) in one law |
| `unused-definition` | A term defined with `以下「X」という。` that the law writes nowhere else |
| `near-term` | A rare word one character away from a defined term |
| `definition-scope` | A term defined with `以下この条において「X」という。` and written in another article that does not define it |

For the two read-as rules, `Status` tells how the quotation fails:

| Status | Meaning |
|---|---|
| `absent` | The article exists and does not contain the quoted words |
| `other-paragraph` | The quoted words are in another paragraph of the article |
| `no-paragraph` | The article has no paragraph with that number |
| `no-article` | The law has no article with that number |

For the other rules `Status` is `no-paragraph` or `no-item` (`relative-ref`), `missing-unit`, `duplicate-paragraph`, `duplicate-item` or `item-gap` (`numbering`), and `toc-range` or `toc-malformed` (a range written without `第`). `Quoted` is the reference or the number, `Context` the words around it, and `Actual` what the law has there.

`referenced-term`, `brackets`, `spelling` and `unused-definition` are `medium`: a paragraph may refer to a term by other words, a law writes the title of another law in the spelling of that law, and a definition may be used only in a form the XML holds as an image. `near-term` and `definition-scope` are `low`: most of what they report is another term of the law or an ordinary word. `brackets` and `referenced-term` do not read a paragraph that has a read-as clause, since a quotation may hold half of a pair.

`read-as-quote-external` names the other law by the title, the law number in parentheses or an abbreviation the law defines with a law number (`（昭和二十二年法律第五十号。以下「法」という。）`) written right before the article, and follows `同法` and `同条` after such a clause. A clause that reaches the other law through the words earlier in the sentence is not looked up. Its confidence is `low` when the title of the law marks a transitional or special-measures law, which quotes the other law as it was, and `medium` otherwise.

`relative-ref` does not read a paragraph that has a read-as clause, a reference inside a quotation, a table, or a reference right after `準用する`. `前項` in the first paragraph and `前号` in the first item are `low`: they are usually sentences of a table or a form, or words copied from another provision.

For `read-as-quote`, `Confidence` tells how sure the linter is of the provision the clause names:

- `high`: the clause names the article by number (`第七条第一項中`) and the law has that article.
- `medium`: the clause names the provision relative to another (`同項中`, `前条中`) or takes it over from the clause before it, where a wrong resolution of the earlier reference carries over; or the law has no article with the number, which usually means the clause refers to another law.
- `low`: words naming another law stand before the reference and the nearest text of the target is unlike the quotation; or the text before the clause names the same article number as an article of another law (`犯罪捜査規範第十六条から第十九条までの規定を準用する。この場合において、第十六条中…`), which also lowers the clauses after it in the paragraph; or several clauses of one paragraph name articles the law does not have. Such clauses usually refer to another law.

A clause that names a provision of another law, a provision that another read-as has already changed (`…において読み替えて準用する第…条中`), a range (`第…条から第…条まで`) or `これらの規定` is counted in `External` or `Unsupported` and gives no warning. A paragraph that gives its read-as as a table (`読み替えられる字句`, `…欄に掲げる字句`) is counted in `TableParagraphs` and skipped. Supplementary provisions are not read.

## Usage

```sh
nix run github:conao3/rust-ritsulint -- law check-law --xml-file path/to/law.xml
nix run github:conao3/rust-ritsulint -- corpus check-corpus --xml-dir path/to/xml --as-of 20261117
nix run github:conao3/rust-ritsulint -- history list-origins --xml-dir path/to/revisions --as-of 20261117
```

`history list-origins` reads every revision of the laws in a directory (the e-Gov API serves each as `/api/2/law_file/xml/<revision id>`; the list is `/api/2/law_revisions/<law id>`) and prints, for each warning of the latest revision, the revision it is given since. `Kind` is `target-changed` when the revision before had the same read-as clause with its quoted words present in the same provisions, `clause-new` when it lacked the clause, `renumbered`, `was-unchecked`, `introduced` for the other rules, and `oldest` or `enacted` when the warning is given since the oldest revision.

```sh
ritsulint law check-law --xml-file law.xml --output table
```

The rules read the articles of the main provisions. A law whose main provisions are paragraphs without articles is counted in `LawsWithoutArticles` and not checked, and supplementary provisions and appended tables are not read.

`corpus check-corpus` reads the directory the e-Gov [bulk download](https://laws.e-gov.go.jp/bulkdownload/) unpacks (`<law id>_<enforcement date>_<amendment id>/<same name>.xml`) and checks, of each law, the latest revision enforced on or before `--as-of`.

- `--output json|text|table` selects the format (default `json`). Keys are in PascalCase.
- `--fail-on-warnings` makes the exit code 1 when there is a warning.
- `--debug` adds `Clauses` to the JSON: every read-as clause with its source, the words before 中, the resolved provisions, the quoted words, the replacement and the result (`present`, `absent`, `other-paragraph`, `no-paragraph`, `no-article`, `external`, `unsupported`).
- Exit codes: 0 success, 1 warnings found with `--fail-on-warnings`, 252 wrong command line, 254 unreadable input.

Each warning has an `Id` computed from the law, the rule, the paragraph it stands in, the provisions it resolves to and the quoted words. The revision is not part of it, so the warnings an amendment introduced are the IDs present in the output for the later revision and absent from the earlier one. The e-Gov API serves the XML of each revision (`https://laws.e-gov.go.jp/api/2/law_file/xml/<revision id>`); the bulk download holds the current and the not yet enforced revisions only.

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

`make corpus-check` runs the linter over a local copy of the bulk download and requires a high-confidence warning for every rule and law listed in `tests/corpus-expected.txt`.

## Site

`site/` is the page at <https://ritsulint.conao3.com/> that lists the warnings for the laws in force on a date. It is a static page (React, react-aria-components, Tailwind CSS) served as the assets of a Cloudflare Worker.

```sh
nix develop --command make site-data CORPUS=path/to/xml AS_OF=20261117
nix develop --command make site-origins REVISIONS=path/to/revisions AS_OF=20261117
nix develop --command make site-check
nix develop --command make site-deploy
```

`make site-data` writes `site/data/warnings.json`, `make site-origins` writes `site/data/origins.json`, and `make site-deploy` needs `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`.

## License

Apache-2.0
