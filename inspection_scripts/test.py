import os


def check_q1():
    # Resolve the path exactly like your other scripts do
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.dirname(script_dir)
    file_path = os.path.join(repo_root, "processed_files", "omni_search_extracted.txt")

    print(f"🕵️ Scanning {file_path} for Q1...\n")

    if not os.path.exists(file_path):
        print(f"🚨 ERROR: Could not find {file_path}")
        return

    count = 0
    # Open the file and stream it line-by-line
    with open(file_path, "r", encoding="utf-8") as f:
        for line in f:
            # Fast check: The Q-ID is bounded by tabs, so we look for exactly that
            if "\tQ1\t" in line:
                print(line.strip())  # Print on the fly!
                count += 1

    print(f"\n✅ Done! Found {count} raw entries for Q1.")


if __name__ == "__main__":
    check_q1()
