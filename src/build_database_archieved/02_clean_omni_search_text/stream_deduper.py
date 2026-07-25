import sys
from tqdm import tqdm


def process_group(q_id, entries, out_file):
    if not entries:
        return

    # Sort by:
    # 1. Length of the lowercased string (shortest first)
    # 2. Type: Titles get priority (0 = title, 1 = everything else)
    entries.sort(key=lambda x: (len(x[3]), 0 if x[1] == "title" else 1))

    seen_strings = set()

    for original_val, entry_type, lang, val_lower in entries:
        # EXACT MATCH CHECK ONLY
        if val_lower in seen_strings:
            continue

        # Keep it! (Write the lowercase version directly)
        seen_strings.add(val_lower)
        out_file.write(f"{val_lower}\t{q_id}\t{entry_type}\t{lang}\n")


def run():
    if len(sys.argv) != 3:
        print("Usage: python stream_deduper.py <input_file> <output_file>")
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2]

    current_qid = None
    current_entries = []

    print("Python: Deduplicating stream...")

    with open(input_path, "r", encoding="utf-8") as f_in, open(
        output_path, "w", encoding="utf-8"
    ) as f_out:

        for line in tqdm(f_in, desc="Processing Lines"):
            parts = line.strip("\n").split("\t")
            if len(parts) != 4:
                continue

            val, q_id, entry_type, lang = parts
            val_lower = val.lower()

            # If we hit a new Q-ID, process the old one and flush memory
            if q_id != current_qid:
                if current_qid is not None:
                    process_group(current_qid, current_entries, f_out)
                current_qid = q_id
                current_entries = []

            current_entries.append((val, entry_type, lang, val_lower))

        # Don't forget to process the very last group in the file!
        if current_qid is not None:
            process_group(current_qid, current_entries, f_out)


if __name__ == "__main__":
    run()
