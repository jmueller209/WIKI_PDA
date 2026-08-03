PYTHON = python

# Kleiner Tipp: Bei .PHONY benutzt man Leerzeichen statt Kommas!
.PHONY: test parse binaries-resume binaries-restart

test:
	cargo build --manifest-path ./src/db_generator/Cargo.toml --release
	./src/db_generator/target/release/db_generator ./config/config.toml --test
