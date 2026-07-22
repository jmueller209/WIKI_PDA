#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

INPUT_FILE="$REPO_ROOT/processed_files/omni_search_extracted.txt"
TEMP_QID_SORT="$REPO_ROOT/processed_files/temp_sorted_by_qid.txt"
TEMP_DEDUPED="$REPO_ROOT/processed_files/temp_deduped.txt"
FINAL_OUTPUT="$REPO_ROOT/processed_files/final_omni_search.txt"

LOCAL_TMP_DIR="$REPO_ROOT/processed_files/sort_tmp"
mkdir -p "$LOCAL_TMP_DIR"

PYTHON_SCRIPT="$SCRIPT_DIR/stream_deduper.py" 

echo "Checking for input at: $INPUT_FILE"

if [ ! -f "$INPUT_FILE" ]; then
    echo "🚨 ERROR: File not found at $INPUT_FILE"
    exit 1
fi

cleanup() {
    echo "Cleaning up local temporary sort files..."
    rm -rf "$LOCAL_TMP_DIR"
    rm -f "$TEMP_QID_SORT"
    rm -f "$TEMP_DEDUPED"
}
trap cleanup ERR EXIT

echo "--------------------------------------------------------"
echo "Step 1/3: Grouping data by Q-ID (Bash Sort)..."
LC_ALL=C sort -t$'\t' -k2,2 -T "$LOCAL_TMP_DIR" "$INPUT_FILE" -o "$TEMP_QID_SORT"

echo "--------------------------------------------------------"
echo "Step 2/3: Removing substring duplicates (Python)..."
python3 "$PYTHON_SCRIPT" "$TEMP_QID_SORT" "$TEMP_DEDUPED"

echo "--------------------------------------------------------"
echo "Step 3/3: Final alphabetical sort (Bash Sort)..."
LC_ALL=C sort -t$'\t' -k1,1 -T "$LOCAL_TMP_DIR" "$TEMP_DEDUPED" -o "$FINAL_OUTPUT"

echo "--------------------------------------------------------"
echo "🎉 Done! Cleaned and sorted file is ready at:"
echo "$FINAL_OUTPUT"
