ELEVATE := sudo

GENERATOR_DIR  = ./src/wiki_pda_tools/
QUERY_LIB_DIR  = ./src/wiki_pda_api/
CONFIG_FILE    = ./config/config.toml

GENERATOR_BIN  = $(GENERATOR_DIR)/target/release/generator
FLASHER_BIN    = $(GENERATOR_DIR)/target/release/flasher

.PHONY: build-rust download parse-wikidata train-dict process-zim qid-bin assemble flash clean purge resume restart-clean restart-purge test-pipeline test-article-processing test test-db-api-debug test-db-api test-db-api-valgrind profile-single-query

build-rust:
	cargo build --manifest-path $(GENERATOR_DIR)/Cargo.toml --release

download: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --download

parse-wikidata: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --parse-wikidata

train-dict: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --train-dict

process-zim: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --process-zim

qid-bin: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --qid-bin

pid-bin: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --pid-bin

assemble: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --assemble

flash: build-rust
	$(ELEVATE) $(FLASHER_BIN) $(CONFIG_FILE)

clean: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --clean

purge: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --purge

resume: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --resume

restart-clean: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --restart-clean

restart-purge: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --restart-purge

test-pipeline: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --test-pipeline

extract-sample-articles: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --extract-sample-articles

test-article-processing: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --test-article-processing

debug-article-processing-anomalies: build-rust
	$(GENERATOR_BIN) $(CONFIG_FILE) --debug-article-processing-anomalies

test-db-api-debug:
	@$(MAKE) _build_test_api CFLAGS_DEBUG="-DDEBUG_MODE -O0 -g" TARGET_NAME="test_api" TEST_SRC="tests/pc_test_api.c"

test-db-api:
	@$(MAKE) _build_test_api CFLAGS_DEBUG="-O3" TARGET_NAME="test_api_release" TEST_SRC="tests/pc_test_api.c"

test-db-api-valgrind:
	@$(MAKE) _build_test_api_norun CFLAGS_DEBUG="-O3 -g" TARGET_NAME="test_api_valgrind" TEST_SRC="tests/pc_test_api.c"
	valgrind --tool=massif --massif-out-file=$(QUERY_LIB_DIR)/target/massif.out $(QUERY_LIB_DIR)/target/test_api_valgrind
	@echo "Heap profiling complete! Run 'ms_print $(QUERY_LIB_DIR)/target/massif.out' to view the graph."

profile-single-query:
	@$(MAKE) _build_test_api_norun CFLAGS_DEBUG="-O3 -g" TARGET_NAME="profile_single_query" TEST_SRC="tests/profile_single_query.c"
	valgrind --tool=callgrind --callgrind-out-file=$(QUERY_LIB_DIR)/target/callgrind.out $(QUERY_LIB_DIR)/target/profile_single_query
	@echo "CPU profiling complete! Run 'kcachegrind $(QUERY_LIB_DIR)/target/callgrind.out' to view the results."

_build_test_api_norun:
	mkdir -p $(QUERY_LIB_DIR)/target
	cd $(QUERY_LIB_DIR) && gcc $(TEST_SRC) \
		src/api/*.c \
		src/core/*.c \
		src/indexes/*.c \
		src/platforms/desktop.c \
		lib/zstd/src/common/*.c \
		lib/zstd/src/decompress/*.c \
		lib/spatial_z/src/*.c \
		lib/tempus/src/*.c \
		-o ./target/$(TARGET_NAME) \
		-I./include -I./lib/zstd/src \
		$(CFLAGS_DEBUG) -DZSTD_DISABLE_ASM -lm

_build_test_api: _build_test_api_norun
	$(QUERY_LIB_DIR)/target/$(TARGET_NAME)
