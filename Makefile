ELEVATE := sudo

GENERATOR_DIR  = ./src/db_generator
QUERY_LIB_DIR  = ./src/wiki_pda_api/
CONFIG_FILE    = ./config/config.toml
GENERATOR_BIN  = $(GENERATOR_DIR)/target/release/db_generator

.PHONY: download parse-wikidata train-dict process-zim qid-bin assemble flash clean purge resume restart-clean restart-purge test-pipeline test-article-processing test test-db-api-debug test-db-api test-db-api-valgrind

$(GENERATOR_BIN):
	cargo build --manifest-path $(GENERATOR_DIR)/Cargo.toml --release

download: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --download

parse-wikidata: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --parse-wikidata

train-dict: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --train-dict

process-zim: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --process-zim

qid-bin: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --qid-bin

assemble: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --assemble

flash: $(GENERATOR_BIN)
	$(ELEVATE) $(GENERATOR_BIN) $(CONFIG_FILE) --flash

clean: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --clean

purge: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --purge

resume: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --resume

restart-clean: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --restart-clean

restart-purge: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --restart-purge

test-pipeline: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --test-pipeline

test-article-processing: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --test-article-processing

test: $(GENERATOR_BIN)
	$(GENERATOR_BIN) $(CONFIG_FILE) --test



test-db-api-debug:
	@$(MAKE) _build_test_api CFLAGS_DEBUG="-DDEBUG_MODE -O0 -g" TARGET_NAME="test_api"

test-db-api:
	@$(MAKE) _build_test_api CFLAGS_DEBUG="-O3" TARGET_NAME="test_api_release"

test-db-api-valgrind:
	@$(MAKE) _build_test_api CFLAGS_DEBUG="-O3" TARGET_NAME="test_api_valgrind"
	valgrind --tool=massif --massif-out-file=$(QUERY_LIB_DIR)/target/massif.out $(QUERY_LIB_DIR)/target/test_api_valgrind
	@echo "Heap profiling complete! Run 'ms_print $(QUERY_LIB_DIR)/target/massif.out' to view the graph."

_build_test_api:
	mkdir -p $(QUERY_LIB_DIR)/target
	cd $(QUERY_LIB_DIR) && gcc tests/pc_test_api.c src/api/*.c src/indexes/*.c src/storage/*.c src/platforms/desktop.c lib/zstd/src/common/*.c lib/zstd/src/decompress/*.c -o ./target/$(TARGET_NAME) -I./include -I./lib/zstd/src $(CFLAGS_DEBUG) -DZSTD_DISABLE_ASM
	$(QUERY_LIB_DIR)/target/$(TARGET_NAME)


