import os
import re
import pycountry
import requests
from bs4 import BeautifulSoup


def get_repo_root():
    """Dynamically finds the repository root folder relative to this script."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.dirname(script_dir)


def load_master_config(repo_root):
    """Parses global parameters using cross-platform safe path combining."""
    config_path = os.path.join(repo_root, "config", "downloader.config")
    params = {}

    if not os.path.exists(config_path):
        return {
            "BASE_URL": "https://ftp.fau.de/kiwix/zim/wikipedia/",
            "MATCH_PATTERN": r"wikipedia_{lang}_all_nopic_(\d{4}-\d{2})\.zim",
            "LANGUAGES_CONFIG_PATH": os.path.join("config", "languages.config"),
        }

    with open(config_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                key, val = line.split("=", 1)
                params[key.strip()] = val.strip()
    return params


def get_language_name(code):
    wiki_overrides = {
        "en-simple": "Simple English",
        "zh-min-nan": "Min Nan Chinese",
        "bat-smg": "Samogitian",
        "be-tarask": "Belarusian (Taraškievica)",
    }
    if code in wiki_overrides:
        return wiki_overrides[code]

    try:
        lang = pycountry.languages.get(alpha_2=code) or pycountry.languages.get(
            alpha_3=code
        )
        return lang.name if lang else f"Unknown ({code})"
    except:
        return f"Unknown ({code})"


def generate_config_file():
    repo_root = get_repo_root()
    cfg = load_master_config(repo_root)

    url = cfg.get("BASE_URL")
    pattern_template = cfg.get("MATCH_PATTERN")

    lang_config_rel = cfg.get(
        "LANGUAGES_CONFIG_PATH", os.path.join("config", "languages.config")
    )
    lang_config_rel = os.path.normpath(lang_config_rel)
    lang_config_abs = os.path.join(repo_root, lang_config_rel)

    print(f"Scanning repository mirror: {url}")
    try:
        response = requests.get(url)
        response.raise_for_status()
    except Exception as e:
        print(f"Error connecting to server: {e}")
        return

    soup = BeautifulSoup(response.text, "html.parser")
    languages = set()

    scanner_pattern_str = pattern_template.replace("{lang}", r"([a-z0-9\-]+)")
    pattern = re.compile(scanner_pattern_str)

    for link in soup.find_all("a"):
        href = link.get("href")
        if href:
            match = pattern.match(href)
            if match:
                languages.add(match.group(1))

    # Safely construct parent configuration directory
    os.makedirs(os.path.dirname(lang_config_abs), exist_ok=True)

    with open(lang_config_abs, "w", encoding="utf-8") as f:
        f.write("# Cyberdeck Language Configuration\n")
        f.write("# Format: { 'code': 'name' }\n")
        f.write("# Uncomment (remove the #) to include in your download queue.\n\n")

        lang_list = sorted(
            [(lang, get_language_name(lang)) for lang in languages],
            key=lambda x: x[1],
        )

        for code, name in lang_list:
            f.write(f"{{ '{code}': '{name}' }},\n")

    print(f"Configuration file generated successfully at: {lang_config_abs}")


if __name__ == "__main__":
    generate_config_file()
