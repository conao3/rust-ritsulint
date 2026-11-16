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
	jq -r '[.Warnings[] | select(.Confidence == "high") | .LawId] | unique | .[]' target/corpus.json > target/corpus-found.txt
	sort tests/corpus-expected.txt | comm -23 - target/corpus-found.txt > target/corpus-missing.txt
	test ! -s target/corpus-missing.txt
	sort tests/corpus-rejected.txt | comm -12 - target/corpus-found.txt > target/corpus-rejected-found.txt
	test ! -s target/corpus-rejected-found.txt
	test "$$(jq '.Warnings | length' target/corpus.json)" = "$$(jq '[.Warnings[].Id] | unique | length' target/corpus.json)"
