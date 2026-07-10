import sys
from libzim.reader import Archive


def dump_html_structure(zim_path, target_path="A/B-Schule"):
    try:
        zim = Archive(zim_path)
        entry = zim.get_entry_by_path(target_path)
        raw_html = bytes(entry.get_item().content).decode("utf-8", errors="ignore")

        # Wir drucken die ersten 1000 Zeichen des HTMLs aus, um die echten Header zu sehen
        print("=" * 60)
        print(f"ROH-HTML HEADERS FÜR: {target_path}")
        print("=" * 60)
        print(raw_html[:1500])
        print("=" * 60)

    except Exception as e:
        print(f"Fehler: {e}")


if __name__ == "__main__":
    TARGET_ZIM = (
        "../zim_files/wikipedia_de_all_nopic_2026-01.zim"  # Passe deinen Pfad an
    )
    dump_html_structure(TARGET_ZIM)
