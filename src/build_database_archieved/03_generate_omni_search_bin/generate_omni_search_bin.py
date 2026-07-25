import struct
import os
from tqdm import tqdm


def get_repo_root():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    step1 = os.path.dirname(script_dir)
    return os.path.dirname(step1)


def text_to_binary():
    repo_root = get_repo_root()
    input_path = os.path.join(repo_root, "processed_files", "final_omni_search.txt")
    output_dir = os.path.join(repo_root, "bin")

    if not os.path.exists(output_dir):
        os.makedirs(output_dir)

    print("--- STEP 1: Finding optimal split point ---")
    # Get total line count for progress bar
    with open(input_path, "r", encoding="utf-8") as f:
        total_lines = sum(1 for _ in f)

    mid_target = total_lines // 2
    split_line_number = 0
    split_char = ""

    with open(input_path, "r", encoding="utf-8") as f:
        for i, line in enumerate(f):
            if i >= mid_target:
                parts = line.split("\t")
                current_term = parts[0]
                # Look for the first character
                if len(current_term) > 0:
                    split_line_number = i
                    split_char = current_term[0]
                    break

    print(f"Total lines: {total_lines}")
    print(f"Optimal split at line: {split_line_number}")
    print(f"Split Point Character: '{split_char}' (First letter of Part 2)")

    print("--- STEP 2: Generating binary files ---")
    binary_format = "64sI"
    FLAG_TITLE = 1 << 31

    with open(input_path, "r", encoding="utf-8") as f_in:
        out1_path = os.path.join(output_dir, "omni_search_0.bin")
        out2_path = os.path.join(output_dir, "omni_search_1.bin")

        with open(out1_path, "wb") as f1, open(out2_path, "wb") as f2:
            for i, line in enumerate(
                tqdm(f_in, desc="Packing binary records", total=total_lines)
            ):
                parts = line.strip().split("\t")
                if len(parts) != 4:
                    continue

                term, qid_str, entry_type, lang = parts
                if not qid_str.startswith("Q"):
                    continue

                try:
                    qid_int = int(qid_str.lstrip("Q"))
                except ValueError:
                    continue

                packed_id = qid_int | FLAG_TITLE if entry_type == "title" else qid_int
                term_bytes = term.encode("utf-8")[:64]
                packed_data = struct.pack(binary_format, term_bytes, packed_id)

                if i < split_line_number:
                    f1.write(packed_data)
                else:
                    f2.write(packed_data)

    config_path = os.path.join(output_dir, "omni_search_split_point.bin")

    with open(config_path, "wb") as f:
        f.write(split_char.encode("utf-8"))

    print(f"Split point '{split_char}' saved to: {config_path}")

    print(f"Success! Files saved in: {output_dir}")


if __name__ == "__main__":
    text_to_binary()
