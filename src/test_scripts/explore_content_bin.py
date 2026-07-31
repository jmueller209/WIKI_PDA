import sys
import os
import zstandard as zstd


def explore_bin(index_path, bin_path, decode_zstd=False, zstd_dict_path=None):
    if not os.path.exists(index_path):
        print(f"Error: Index file not found at '{index_path}'")
        return
    if not os.path.exists(bin_path):
        print(f"Error: Binary file not found at '{bin_path}'")
        return

    # Setup zstd decompressor if requested
    dctx = None
    if decode_zstd:
        if zstd is None:
            print(
                "Error: 'zstandard' package is not installed. Run 'pip install zstandard'."
            )
            return

        dict_data = None
        if zstd_dict_path and os.path.exists(zstd_dict_path):
            print(f"Loading Zstd dictionary from: {zstd_dict_path}")
            with open(zstd_dict_path, "rb") as dict_file:
                dict_bytes = dict_file.read()
            dict_data = zstd.ZstdCompressionDict(dict_bytes)
        elif zstd_dict_path:
            print(
                f"Warning: Dictionary path '{zstd_dict_path}' not found. Proceeding without dictionary."
            )

        dctx = zstd.ZstdDecompressor(dict_data=dict_data)

    print(f"Opening index: {index_path}")
    print(f"Opening binary: {bin_path}")
    print(f"Zstd Decoding Mode: {'ENABLED' if decode_zstd else 'DISABLED'}\n")

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

            # Display metadata header
            print("\n" + "=" * 80)
            print(f"📄 Article #{count} | QID: {qid} | Wiki: {wiki_lang}")
            print(f"📍 Offset: {offset} | Length: {length} bytes (Compressed/Raw)")
            print("=" * 80)

            # 1. Always show compressed view information
            num_raw_bytes = len(raw_bytes)
            print(f"[COMPRESSED / RAW BYTES] ({len(raw_bytes)} bytes):")
            print(raw_bytes[:300])  # Print first 300 bytes safely as snippet
            if len(raw_bytes) > 300:
                print(f"... [Truncated, total raw size: {len(raw_bytes)} bytes]")

            # 2. Decode zstd if enabled and show uncompressed content
            if decode_zstd:
                print("-" * 80)
                try:
                    stream = dctx.stream_reader(raw_bytes)
                    decompressed_bytes = stream.read()
                    uncompressed_text = decompressed_bytes.decode(
                        "utf-8", errors="replace"
                    )
                    num_decompressed_bytes = len(decompressed_bytes)
                    print(f"[DECOMPRESSED CONTENT] ({len(decompressed_bytes)} bytes):")
                    print(uncompressed_text)
                    print(
                        f"Compression Ratio: {num_raw_bytes / num_decompressed_bytes:.2f}"
                    )
                except Exception as e:
                    print(f"❌ Failed to decompress block with Zstd: {e}")

            print("-" * 80)
            user_input = input(
                "Press [Enter] for the next article (or type 'q' and Enter to quit): "
            )
            if user_input.strip().lower() == "q":
                print("Exiting explorer.")
                break


if __name__ == "__main__":
    idx_path = "../../tmp/qid_index.txt"
    bin_path = "../../bin/content.bin"

    ENABLE_ZSTD_DECODE = True
    ZSTD_DICT_FILE = "../../bin/zstd_dictionary.bin"

    explore_bin(
        index_path=idx_path,
        bin_path=bin_path,
        decode_zstd=ENABLE_ZSTD_DECODE,
        zstd_dict_path=ZSTD_DICT_FILE,
    )
