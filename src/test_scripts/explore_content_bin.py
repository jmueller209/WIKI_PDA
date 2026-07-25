import sys
import os


def explore_bin(index_path, bin_path):
    if not os.path.exists(index_path):
        print(f"Error: Index file not found at '{index_path}'")
        return
    if not os.path.exists(bin_path):
        print(f"Error: Binary file not found at '{bin_path}'")
        return

    print(f"Opening index: {index_path}")
    print(f"Opening binary: {bin_path}\n")

    with open(bin_path, "rb") as bin_file, open(
        index_path, "r", encoding="utf-8"
    ) as idx_file:
        count = 0
        for line in idx_file:
            parts = line.strip().split("\t")
            if len(parts) != 4:
                continue

            qid, wiki_lang, offset_str, length_str = parts
            try:
                offset = int(offset_str)
                length = int(length_str)
            except ValueError:
                continue

            count += 1

            # Seek and read the exact chunk from content.bin
            bin_file.seek(offset)
            raw_bytes = bin_file.read(length)

            # Decode safely as UTF-8 (handling any potential weird character encoding)
            article_text = raw_bytes.decode("utf-8", errors="replace")

            # Display metadata
            print("\n" + "=" * 80)
            print(f"📄 Article #{count} | QID: {qid} | Wiki: {wiki_lang}")
            print(f"📍 Offset: {offset} | Length: {length} bytes")
            print("=" * 80)

            # Print the article content (if it's massive, you can inspect it here)
            print(article_text)

            print("-" * 80)
            user_input = input(
                "Press [Enter] for the next article (or type 'q' and Enter to quit): "
            )
            if user_input.strip().lower() == "q":
                print("Exiting explorer.")
                break


if __name__ == "__main__":
    idx_path = "../../tmp/qid_index_unsorted.txt"
    bin_path = "../../bin/content.bin"
    explore_bin(idx_path, bin_path)
