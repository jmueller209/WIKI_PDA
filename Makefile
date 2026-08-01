PYTHON = python

# Kleiner Tipp: Bei .PHONY benutzt man Leerzeichen statt Kommas!
.PHONY: test parse binaries-resume binaries-restart

test:
	cargo build --manifest-path ./src/02_create_binaries/Cargo.toml --release
	./src/02_create_binaries/target/release/create_binaries ./config/config.toml --test


parse:
	cargo build --manifest-path ./src/01_parse_wikidata_database_dump/Cargo.toml --release
	./src/01_parse_wikidata_database_dump/target/release/parse_wikidata_database_dump ./config/config.toml

binaries-resume:
	cargo build --manifest-path ./src/02_create_binaries/Cargo.toml --release
	./src/02_create_binaries/target/release/create_binaries ./config/config.toml --resume

binaries-restart:
	cargo build --manifest-path ./src/02_create_binaries/Cargo.toml --release
	./src/02_create_binaries/target/release/create_binaries ./config/config.toml --restart


