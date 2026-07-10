import os
import re
import requests
from bs4 import BeautifulSoup
from tqdm import tqdm


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
                key, val = line.split("=", 1)
                params[key.strip()] = val.strip()
    return params


def get_enabled_languages(lang_config_path):
    enabled = []
    if not os.path.exists(lang_config_path):
        return enabled

    with open(lang_config_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                match = re.search(r"['\"]([a-z0-9\-]+)['\"]", line)
                if match:
                    enabled.append(match.group(1))
    return enabled


def download_latest_zims():
    repo_root = get_repo_root()

    try:
        cfg = load_master_config(repo_root)
    except Exception as e:
        print(e)
        return

    lang_config_rel = os.path.normpath(cfg.get("LANGUAGES_CONFIG_PATH"))
    lang_config_abs = os.path.join(repo_root, lang_config_rel)

    languages = get_enabled_languages(lang_config_abs)
    if not languages:
        print(
            f"No active languages found. Make sure to uncomment your choices inside: {lang_config_abs}"
        )
        return

    base_url = cfg.get("BASE_URL")
    download_rel = os.path.normpath(cfg.get("DOWNLOAD_PATH", "zim_files"))
    download_dir_abs = os.path.join(repo_root, download_rel)

    os.makedirs(download_dir_abs, exist_ok=True)

    print("Fetching server repository directory...")
    response = requests.get(base_url)
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

    for lang, (date, filename, url) in latest_files.items():
        destination_path = os.path.join(download_dir_abs, filename)
        print(f"\nProcessing target '{lang}' [{date}]: {filename}")

        response = requests.get(url, stream=True)
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
        print(f"Successfully saved to: {destination_path}")


if __name__ == "__main__":
    download_latest_zims()
