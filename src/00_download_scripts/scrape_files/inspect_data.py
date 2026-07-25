import os
import gzip


def inspect_sql_dump(sql_path):
    print(f"\n--- Inspecting {os.path.basename(sql_path)} ---")
    if os.path.exists(sql_path):
        # Using a simple line-by-line read to avoid BadGzipFile issues
        with gzip.open(sql_path, "rt", encoding="utf-8", errors="ignore") as f:
            count = 0
            for line in f:
                if "INSERT INTO" in line:
                    print(line[:150] + "...")
                    count += 1
                if count >= 5:
                    break
    else:
        print(f"File not found: {sql_path}")


def inspect_data():
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    zim_dir = os.path.join(repo_root, "zim_files")

    # Just point to the SQL file
    sql_path = os.path.join(zim_dir, "wikidatawiki-latest-wb_items_per_site.sql.gz")

    # Run inspection once
    inspect_sql_dump(sql_path)


if __name__ == "__main__":
    inspect_data()
