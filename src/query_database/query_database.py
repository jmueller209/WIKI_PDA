import os
import struct
import mmap
import curses

# 68-byte fixed record: 64s (Term), I (Packed ID)
RECORD_SIZE = 68
FLAG_TITLE = 1 << 31


def get_repo_root():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.dirname(script_dir)


def load_split_char(bin_dir):
    with open(os.path.join(bin_dir, "omni_search_split_point.bin"), "rb") as f:
        return f.read().decode("utf-8")


def binary_search(mm, query_bytes, num_records):
    low, high = 0, num_records - 1
    first_match = -1

    while low <= high:
        mid = (low + high) // 2
        # Read term bytes for comparison
        record_term = mm[mid * RECORD_SIZE : (mid * RECORD_SIZE) + 64].rstrip(b"\x00")

        if record_term.startswith(query_bytes):
            first_match = mid
            high = mid - 1  # Keep looking left for the absolute first match
        elif record_term < query_bytes:
            low = mid + 1
        else:
            high = mid - 1
    return first_match


def search(query, bin_dir, max_depth=10000):
    if not query:
        return []

    split_char = load_split_char(bin_dir)
    query_bytes = query.encode("utf-8")

    target_file = "omni_search_0.bin" if query[0] < split_char else "omni_search_1.bin"
    file_path = os.path.join(bin_dir, target_file)

    if not os.path.exists(file_path):
        return [], 0

    with open(file_path, "rb") as f:
        mm = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
        num_records = mm.size() // RECORD_SIZE

        idx = binary_search(mm, query_bytes, num_records)
        if idx == -1:
            mm.close()
            return [], 0

        # qid_map stores: qid -> {"term": ..., "is_title": ...}
        qid_map = {}

        while idx < num_records:
            # Check depth limit
            if max_depth != -1 and len(qid_map) >= max_depth:
                break

            data = mm[idx * RECORD_SIZE : (idx + 1) * RECORD_SIZE]
            term_bytes, packed_id = struct.unpack("64sI", data)
            term = term_bytes.rstrip(b"\x00").decode("utf-8", errors="ignore")

            if not term.startswith(query):
                break

            qid = packed_id & ~FLAG_TITLE
            is_title = (packed_id & FLAG_TITLE) != 0

            # Deduplication: Keep the version that is a TITLE,
            # or if neither are titles, just keep the first one found.
            if qid not in qid_map or (is_title and not qid_map[qid]["is_title"]):
                qid_map[qid] = {"term": term, "qid": qid, "is_title": is_title}

            idx += 1

        mm.close()
    all_results = list(qid_map.values())
    total_found = len(all_results)
    # Convert map to list and sort
    results = list(qid_map.values())
    query_len = len(query)
    results.sort(
        key=lambda x: (
            0 if x["term"] == query else 1,  # 1st Priority: EXACT MATCH IS KING
            not x["is_title"],  # 2nd Priority: Titles beat Aliases
            len(x["term"]) - query_len,  # 3rd Priority: Shortest length
            x["term"],  # 4th Priority: Alphabetical
            x["qid"],  # 5th Priority: Lowest Q-ID
        )
    )

    return results[:10], total_found


def main(stdscr):
    # Setup screen
    curses.curs_set(1)
    stdscr.clear()

    repo_root = get_repo_root()
    bin_dir = os.path.join(repo_root, "bin")

    query = ""
    while True:
        stdscr.clear()
        stdscr.addstr(0, 0, f"Search: {query}")

        # Perform search
        results, total_found = search(query, bin_dir) if query else ([], 0)

        stdscr.addstr(1, 0, f"Showing Top 10 of {total_found} matches.")
        # Display results
        for i, res in enumerate(results):
            type_label = "[TITLE]" if res["is_title"] else "[LABEL/ALIAS]"
            line = f"{i+1}. {type_label} {res['term']} (Q{res['qid']})"
            stdscr.addstr(i + 3, 0, line)  # Adjusted row to make space for the count
        stdscr.refresh()

        # Get character input
        ch = stdscr.getch()
        if ch == 27:
            break  # ESC to exit
        elif ch in (curses.KEY_BACKSPACE, 127):  # Backspace
            query = query[:-1]
        elif 32 <= ch <= 126:  # Printable characters
            query += chr(ch).lower()


if __name__ == "__main__":
    curses.wrapper(main)
