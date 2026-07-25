import gzip
import json
import os


def create_large_mock_dump(total_records=2000000):
    """
    Generates only the mock compressed Wikidata dump file.
    Inflates the file size with heavy 'claims' blocks to test fast-skip performance.
    Now includes 'sitelinks' to test Article Title extraction.
    """
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    zim_dir = os.path.join(repo_root, "zim_files")
    os.makedirs(zim_dir, exist_ok=True)

    output_path = os.path.join(zim_dir, "mock-latest-all.json.gz")

    # 1. Define our golden target records that match your current whitelist numbers
    golden_records = {
        42: {
            "id": "Q42",
            "type": "item",
            "labels": {
                "en": {"language": "en", "value": "Douglas Adams"},
                "de": {"language": "de", "value": "Douglas Adams"},
            },
            "aliases": {
                "en": [
                    {"language": "en", "value": "DNA"},
                    {"language": "en", "value": "Douglas Noel Adams"},
                ]
            },
            # NEW: Sitelinks block for Article Titles
            "sitelinks": {
                "enwiki": {"title": "Douglas_Adams"},
                "dewiki": {"title": "Douglas_Adams"},
            },
        },
        2: {
            "id": "Q2",
            "type": "item",
            "labels": {"en": {"language": "en", "value": "Earth"}},
            "aliases": {
                "en": [
                    {"language": "en", "value": "The World"},
                    {"language": "en", "value": "Blue Planet"},
                ]
            },
            # NEW: Sitelinks block for Article Titles
            "sitelinks": {
                "enwiki": {"title": "Earth_(planet)"},
                "dewiki": {"title": "Erde"},
            },
        },
        12345: {
            "id": "Q12345",
            "type": "item",
            "labels": {"fr": {"language": "fr", "value": "Pomme"}},
            "sitelinks": {"frwiki": {"title": "Pomme"}},
        },
    }

    print(f"Generating large mock compressed Wikidata dump at {output_path}...")
    print(
        f"Targeting {total_records:,} total lines. This will take a moment to compress..."
    )

    junk_payload = "X" * 1024

    with gzip.open(output_path, "wt", encoding="utf-8", compresslevel=4) as f:
        f.write("[\n")

        for i in range(1, total_records + 1):
            if i in golden_records:
                entity = golden_records[i]
                entity["claims"] = {
                    "P31": [{"mainsnak": {"datavalue": {"value": junk_payload}}}]
                }
            else:
                entity = {
                    "id": f"Q{i}",
                    "type": "item",
                    "labels": {
                        "en": {"language": "en", "value": f"Random Entity Title {i}"},
                        "es": {"language": "es", "value": f"Título Aleatorio {i}"},
                    },
                    # NEW: Sitelinks block for noise data
                    "sitelinks": {
                        "enwiki": {"title": f"Random_Entity_{i}"},
                        "eswiki": {"title": f"Titulo_{i}"},
                    },
                    "claims": {
                        "P31": [{"mainsnak": {"datavalue": {"value": junk_payload}}}],
                        "P21": [{"mainsnak": {"datavalue": {"value": junk_payload}}}],
                        "P569": [{"mainsnak": {"datavalue": {"value": junk_payload}}}],
                    },
                }

            # Strictly minified format for the Rust parser
            json_str = json.dumps(entity, separators=(",", ":"))

            if i < total_records:
                f.write(json_str + ",\n")
            else:
                f.write(json_str + "\n")

            if i % 500000 == 0:
                print(f" -> Wrote {i:,} / {total_records:,} records...")

        f.write("]\n")

    print(f"\nSuccessfully generated {output_path}!")
    file_size_mb = os.path.getsize(output_path) / (1024 * 1024)
    print(f"Compressed file size on disk: {file_size_mb:.2f} MB")


if __name__ == "__main__":
    create_large_mock_dump(total_records=2000000)
