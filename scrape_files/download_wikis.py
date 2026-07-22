import os
import re
import requests
from bs4 import BeautifulSoup
from tqdm import tqdm
from datetime import datetime


def get_repo_root():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.dirname(script_dir)


def load_master_config(repo_root):
    config_path = os.path.join(repo_root, "config", "downloader.config")
    params = {}
    if not os.path.exists(config_path):
        raise FileNotFoundError(f"Missing master config file at: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                if "=" in line:
                    key, val = line.split("=", 1)
                    params[key.strip()] = val.strip()
    return params


def get_enabled_languages(auto_config_path):
    enabled = []
    if not os.path.exists(auto_config_path):
        print(f"🚨 Error: Could not find language config at {auto_config_path}")
        return enabled

    with open(auto_config_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                enabled.append(line)
    return enabled


def format_size(bytes_size):
    """Converts raw bytes into human-readable MB/GB formats."""
    for unit in ["B", "KB", "MB", "GB", "TB"]:
        if bytes_size < 1024.0:
            return f"{bytes_size:.2f} {unit}"
        bytes_size /= 1024.0
    return f"{bytes_size:.2f} PB"


def download_latest_zims():
    print("🚀 Booting up downloader script...")
    repo_root = get_repo_root()

    # Setup Logging Infrastructure
    log_dir = os.path.join(repo_root, "logs")
    os.makedirs(log_dir, exist_ok=True)
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    log_path = os.path.join(log_dir, f"download_audit_{timestamp}.log")

    try:
        cfg = load_master_config(repo_root)
    except Exception as e:
        print(f"🚨 Config Error: {e}")
        return

    auto_config_abs = os.path.join(repo_root, "config", "discovered_languages.config")
    languages = get_enabled_languages(auto_config_abs)

    if not languages:
        print(f"⚠️ No active languages found. Check {auto_config_abs}")
        return

    base_url = cfg.get("BASE_URL")
    download_rel = os.path.normpath(cfg.get("DOWNLOAD_PATH", "zim_files"))
    download_dir_abs = os.path.join(repo_root, download_rel)
    os.makedirs(download_dir_abs, exist_ok=True)

    print(f"✅ Loaded {len(languages)} active target languages.")
    print(f"📡 Fetching server repository directory from {base_url} ...")

    try:
        response = requests.get(base_url)
        response.raise_for_status()
    except requests.RequestException as e:
        print(f"🚨 Network Error reaching Kiwix server: {e}")
        return

    soup = BeautifulSoup(response.text, "html.parser")
    latest_files = {}

    for link in soup.find_all("a"):
        href = link.get("href")
        if not href:
            continue

        for lang in languages:
            specific_pattern = cfg.get("MATCH_PATTERN").replace("{lang}", lang)
            match = re.match(specific_pattern, href)

            if match:
                date = match.group(1)
                if lang not in latest_files or date > latest_files[lang][0]:
                    latest_files[lang] = (date, href, base_url + href)

    if not latest_files:
        print(
            "⚠️ No matching ZIM files found on the server. Check your MATCH_PATTERN in downloader.config!"
        )
        return

    print(f"🎯 Found {len(latest_files)} matching ZIM files to process.")

    # Metrics for the log
    stats = {"downloaded": 0, "skipped": 0, "total_bytes_downloaded": 0}

    # Open log file to write as we go
    with open(log_path, "w", encoding="utf-8") as log_file:
        log_file.write(
            f"=== DOWNLOAD AUDIT LOG: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')} ===\n"
        )
        log_file.write("Mode: SKIP EXISTING\n")
        log_file.write("=========================================================\n\n")

        for lang, (date, filename, url) in latest_files.items():
            destination_path = os.path.join(download_dir_abs, filename)

            # Skip existing files
            if os.path.exists(destination_path):
                msg = f"⏭️ SKIPPED '{lang}': {filename} (Already exists on disk)"
                print(msg)
                log_file.write(f"{msg}\n")
                stats["skipped"] += 1
                continue

            print(f"\n📥 Processing target '{lang}' [{date}]: {filename}")

            try:
                head_resp = requests.get(url, stream=True)
                head_resp.raise_for_status()
                total_size = int(head_resp.headers.get("content-length", 0))
                size_readable = format_size(total_size)
            except requests.RequestException as e:
                err_msg = f"❌ FAILED '{lang}': {filename} (Network error: {e})"
                print(err_msg)
                log_file.write(f"{err_msg}\n")
                continue

            # Execute the download
            with open(destination_path, "wb") as f, tqdm(
                desc=filename,
                total=total_size,
                unit="iB",
                unit_scale=True,
                unit_divisor=1024,
            ) as bar:
                for chunk in head_resp.iter_content(chunk_size=8192):
                    if chunk:
                        f.write(chunk)
                        bar.update(len(chunk))

            # Record success
            stats["downloaded"] += 1
            stats["total_bytes_downloaded"] += total_size
            success_msg = f"✅ DOWNLOADED '{lang}': {filename} ({size_readable})"
            print(success_msg)
            log_file.write(f"{success_msg}\n")

        # Write Final Summary
        log_file.write("\n=========================================================\n")
        log_file.write("=== FINAL SUMMARY ===\n")
        log_file.write(f"Total Target Languages Processed: {len(latest_files)}\n")
        log_file.write(f"Files Successfully Downloaded:    {stats['downloaded']}\n")
        log_file.write(f"Files Skipped (Already Existed):  {stats['skipped']}\n")
        log_file.write(
            f"Total Data Acquired This Session: {format_size(stats['total_bytes_downloaded'])}\n"
        )
        log_file.write("=========================================================\n")

    print(f"\n🎉 Operations complete! Detailed run log saved to: {log_path}")


if __name__ == "__main__":
    download_latest_zims()
