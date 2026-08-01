.PHONY: test_omni_search test_qid_search

test_omni_search:
	mkdir -p ./target
	gcc src/omni_search.c src/tests/pc_test_omni_search.c -o ./target/test_omni_search -DDEBUG_MODE -O0 -g
	./target/test_omni_search ./../../bin/data_base.bin

test_qid_search:
	mkdir -p ./target
	gcc src/qid_search.c src/tests/pc_test_qid_search.c -o ./target/test_qid_search -DDEBUG_MODE -O0 -g
	./target/test_qid_search ./../../bin/data_base.bin
