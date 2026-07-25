import os
import gzip


def get_repo_root():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.dirname(script_dir)


def load_master_config(repo_root):
    config_path = os.path.join(repo_root, "config", "downloader.config")
    params = {}
    if not os.path.exists(config_path):
        raise FileNotFoundError(f"Master config not found at: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                if "=" in line:
                    key, val = line.split("=", 1)
                    params[key.strip()] = val.strip()
    return params


def peek_json_dump():
    repo_root = get_repo_root()

    try:
        cfg = load_master_config(repo_root)
    except Exception as e:
        print(f"Error loading config: {e}")
        return

    download_dir_rel = os.path.normpath(cfg.get("DOWNLOAD_PATH", "zim_files"))
    filename = "latest-all.json.gz"

    file_path = os.path.join(repo_root, download_dir_rel, filename)

    print(f"Attempting to open and read: {file_path}\n")

    if not os.path.exists(file_path):
        print(f"CRITICAL ERROR: File not found at {file_path}")
        print("Check if the download finished or if the filename is correct.")
        return

    try:
        with gzip.open(file_path, "rt", encoding="utf-8") as f:
            for i in range(5):
                line = f.readline()
                if not line:
                    print(f"\n[EOF] End of file reached prematurely at line {i+1}.")
                    break

                print(f"--- Line {i+1} ---")
                print(f"Length: {len(line):,} characters")
                print(f"Content: {repr(line[:300])} ... [TRUNCATED]\n")

    except Exception as e:
        print(f"\n🚨 CRITICAL ERROR: {e}")
        print(
            "If you see an 'unexpected end of file', 'Not a gzipped file', or CRC error here, your download is corrupted or incomplete."
        )


if __name__ == "__main__":
    peek_json_dump()
