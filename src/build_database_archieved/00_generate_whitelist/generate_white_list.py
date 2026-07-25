import os
import gzip
import re
from tqdm import tqdm


def get_repo_root():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.dirname(os.path.dirname(script_dir))


def load_downloader_config(repo_root):
    config_path = os.path.join(repo_root, "config", "downloader.config")
    params = {}
    if os.path.exists(config_path):
        with open(config_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#"):
                    if "=" in line:
                        key, val = line.split("=", 1)
                        params[key.strip()] = val.strip()
    return params


def generate_global_whitelist():
    repo_root = get_repo_root()
    cfg = load_downloader_config(repo_root)

    download_dir = cfg.get("DOWNLOAD_PATH", "zim_files")
    zim_dir_abs = os.path.join(repo_root, os.path.normpath(download_dir))
    processed_dir_abs = os.path.join(repo_root, "processed_files")

    sql_gz_path = os.path.join(
        zim_dir_abs, "wikidatawiki-latest-wb_items_per_site.sql.gz"
    )
    output_path = os.path.join(processed_dir_abs, "whitelist.txt")
    log_path = os.path.join(repo_root, "logs", "whitelist_generation.log")

    os.makedirs(processed_dir_abs, exist_ok=True)

    if not os.path.exists(sql_gz_path):
        print(f"🚨 Error: Could not find SQL file at {sql_gz_path}")
        return

    approved_q_ids = set()
    site_counts = {}

    tuple_pattern = re.compile(r"\(\d+,(\d+),'([a-zA-Z0-9_-]+)'")

    print(f"Scanning SQL dump: {sql_gz_path}")
    print("Targeting: ALL global Wikipedia languages.")

    excluded_wikis = {
        "wikidatawiki",
        "commonswiki",
        "specieswiki",
        "metawiki",
        "sourceswiki",
    }

    with gzip.open(sql_gz_path, "rt", encoding="utf-8", errors="ignore") as f:
        for line in tqdm(f, desc="Parsing SQL Lines", unit=" lines"):

            if not line.startswith("INSERT INTO `wb_items_per_site`"):
                continue

            for match in tuple_pattern.finditer(line):
                item_id_str = match.group(1)
                site_id = match.group(2)

                if site_id.endswith("wiki") and site_id not in excluded_wikis:
                    approved_q_ids.add(int(item_id_str))
                    site_counts[site_id] = site_counts.get(site_id, 0) + 1

    print(
        f"\n✅ Scan complete. Found {len(approved_q_ids):,} globally notable entities."
    )

    print(f"Writing integers to {output_path}...")
    with open(output_path, "w", encoding="utf-8") as out_f:
        for q_id in sorted(approved_q_ids):
            out_f.write(f"{q_id}\n")

    print(f"Writing global language statistics to {log_path}...")
    os.makedirs(os.path.dirname(log_path), exist_ok=True)
    with open(log_path, "w", encoding="utf-8") as log_f:
        log_f.write("=== WIKIPEDIA ARTICLE COUNTS BY LANGUAGE ===\n\n")
        for site, count in sorted(
            site_counts.items(), key=lambda item: item[1], reverse=True
        ):
            log_f.write(f"{site}: {count:,}\n")

    print("🎉 Global Whitelist generation complete!")


if __name__ == "__main__":
    generate_global_whitelist()
