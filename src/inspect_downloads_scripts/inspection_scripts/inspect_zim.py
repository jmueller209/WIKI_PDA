import os
from libzim.reader import Archive


def peek_zim(zim_path, limit=50):
    print(f"🔍 Peeking into: {zim_path}")
    archive = Archive(zim_path)

    print(f"Total entries in archive: {archive.all_entry_count}\n")
    print(f"{'ID':<6} | {'NS':<4} | {'MIME Type':<15} | {'Path / Title'}")
    print("-" * 60)

    for i in range(min(limit, archive.all_entry_count)):
        entry = archive._get_entry_by_id(i)

        # Safely grab attributes
        path = getattr(entry, "path", "N/A")
        namespace = getattr(entry, "namespace", "N/A")
        title = getattr(entry, "title", "N/A")

        mimetype = "Redirect"
        if not entry.is_redirect:
            try:
                item = entry.get_item()
                mimetype = item.mimetype
            except Exception:
                mimetype = "Unknown/Error"

        print(f"[{i:<4}] | {str(namespace):<4} | {mimetype:<15} | {path}")


if __name__ == "__main__":
    # Pointing to your exact HT file
    ZIM_FILE = "../../../data/wiki/wikipedia_de_all_nopic_2026-01.zim"

    if os.path.exists(ZIM_FILE):
        peek_zim(ZIM_FILE)
    else:
        print(f"🚨 ERROR: Could not find {ZIM_FILE}")
