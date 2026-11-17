LEANFRAN ?= leanfran

.PHONY: all
all: check

# Build the Lean package (proofs included) and regenerate src/generated/.
.PHONY: generate
generate:
	cd lean && lake build
	$(LEANFRAN) project generate --lake-dir lean --output-dir src/generated --modules Ritsulint.Api --output text
	rustfmt --edition 2024 src/generated/mod.rs

.PHONY: check-generated
check-generated: generate
	git diff --exit-code -- src/generated

.PHONY: fmt-check
fmt-check:
	nix fmt -- --ci

.PHONY: lint
lint:
	cargo clippy --all-targets -- -D warnings

.PHONY: test
test:
	cd lean && lake build
	cargo test

.PHONY: check
check: fmt-check lint test

# Run the release build over the e-Gov bulk download unpacked at CORPUS. Require a
# high-confidence warning for every law listed in tests/corpus-expected.txt, no high-confidence
# warning for the laws listed in tests/corpus-rejected.txt, and distinct warning IDs.
.PHONY: corpus-check
corpus-check:
	test -n "$(CORPUS)"
	cargo build --release
	./target/release/ritsulint corpus check-corpus --xml-dir "$(CORPUS)" --as-of "$(AS_OF)" > target/corpus.json
	jq -r '[.Warnings[] | select(.Confidence == "high") | .Rule + " " + .LawId] | unique | .[]' target/corpus.json | LC_ALL=C sort > target/corpus-found.txt
	LC_ALL=C sort tests/corpus-expected.txt | LC_ALL=C comm -23 - target/corpus-found.txt > target/corpus-missing.txt
	test ! -s target/corpus-missing.txt
	LC_ALL=C sort tests/corpus-rejected.txt | LC_ALL=C comm -12 - target/corpus-found.txt > target/corpus-rejected-found.txt
	test ! -s target/corpus-rejected-found.txt
	test "$$(jq '.Warnings | length' target/corpus.json)" = "$$(jq '[.Warnings[].Id] | unique | length' target/corpus.json)"

# Write the warnings the site shows from the corpus at CORPUS as of AS_OF (YYYYMMDD).
.PHONY: site-data
site-data:
	test -n "$(CORPUS)"
	cargo build --release
	./target/release/ritsulint corpus check-corpus --xml-dir "$(CORPUS)" --as-of "$(AS_OF)" \
		| jq -c --arg asof "$(AS_OF)" '{AsOf: ($$asof[0:4] + "-" + $$asof[4:6] + "-" + $$asof[6:8]), Laws, Counts, ExternalChecked, Warnings: [.Warnings[] | {Id, Rule, Status, LawId, LawTitle, Source, Scope, Targets, Quoted, Nearest, Confidence} + (if .Context then {Context} else {} end) + (if .Actual then {Actual} else {} end) + (if .TargetLawTitle then {TargetLawId, TargetLawTitle} else {} end)]}' \
		> site/data/warnings.json

# Write since which revision each warning is given, from every revision of the laws with a
# warning, unpacked at REVISIONS as <revision>/<revision>.xml (the e-Gov API serves them).
.PHONY: site-origins
site-origins:
	test -n "$(REVISIONS)"
	cargo build --release
	./target/release/ritsulint history list-origins --xml-dir "$(REVISIONS)" --as-of "$(AS_OF)" \
		| jq -c '{Origins: [.Origins[] | {Id, FirstRevisionId, Kind}]}' > site/data/origins.json

.PHONY: site-check
site-check:
	cd site && pnpm install --frozen-lockfile && pnpm run check && pnpm run build

# Publish site/dist to the Worker. Needs CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID.
.PHONY: site-deploy
site-deploy: site-check
	cd site && pnpm run deploy
