import os
import requests
from tqdm import tqdm


def get_repo_root():
    """Dynamically finds the root of the repository."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.dirname(script_dir)


def load_master_config(repo_root):
    """Reads the master config from config/downloader.config."""
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


def download_wb_terms_dump():
    repo_root = get_repo_root()

    try:
        cfg = load_master_config(repo_root)
    except Exception as e:
        print(f"Error loading config: {e}")
        return

    url = cfg.get("WB_TERMS_URL")
    if not url:
        print("Error: WB_TERMS_URL is missing in downloader.config!")
        return

    filename = url.split("/")[-1]

    terms_dir_rel = os.path.normpath(cfg.get("DOWNLOAD_PATH", "zim_files"))
    terms_dir_abs = os.path.join(repo_root, terms_dir_rel)

    destination_path = os.path.join(terms_dir_abs, filename)

    print(f"Starting download of Wikidata Terms Dump...")
    print(f"Source:      {url}")
    print(f"Destination: {destination_path}")

    os.makedirs(terms_dir_abs, exist_ok=True)

    try:
        headers = {
            "User-Agent": "CyberdeckCompiler/1.0 (Mozilla/5.0; Windows NT 10.0; Win64; x64)"
        }
        response = requests.get(url, headers=headers, stream=True)
        response.raise_for_status()
        total_size = int(response.headers.get("content-length", 0))

        with open(destination_path, "wb") as f, tqdm(
            desc=filename,
            total=total_size,
            unit="iB",
            unit_scale=True,
            unit_divisor=1024,
        ) as bar:
            for chunk in response.iter_content(chunk_size=4096):
                if chunk:
                    f.write(chunk)
                    bar.update(len(chunk))

        print(f"\nSuccessfully downloaded and saved to: {destination_path}")

    except Exception as e:
        print(f"\nDownload failed: {e}")


if __name__ == "__main__":
    download_wb_terms_dump()
