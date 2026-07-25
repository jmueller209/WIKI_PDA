import sys
import re
from libzim.reader import Archive
from bs4 import BeautifulSoup


def get_ith_article(zim_path, target_index):
    try:
        zim = Archive(zim_path)
    except Exception as e:
        print(f"Fehler beim Öffnen der ZIM-Datei: {e}")
        return

    article_counter = 0

    # Wir iterieren durch die internen IDs der ZIM-Datei.
    # Da die ZIM-Struktur nach URL (Namespace + Name) sortiert ist,
    # kommen alle "A/" Einträge automatisch in alphabetischer Reihenfolge!
    for i in range(zim.all_entry_count):
        entry = zim._get_entry_by_id(i)
        path = entry.path

        # Wir filtern: Nur Artikel (A/) und keine Weiterleitungen (Redirects)
        if path and path.startswith("A/") and not entry.is_redirect:

            # Prüfen, ob wir unseren gewünschten i-ten Artikel erreicht haben
            if article_counter == target_index:
                title = entry.title
                item = entry.get_item()
                raw_bytes = bytes(item.content)

                # ZIM HTML-Payload dekodieren
                raw_html = raw_bytes.decode("utf-8", errors="ignore")

                # 1. Q-ID extrahieren (Der clevere Weg ohne externe Wikidata-Datenbank)
                q_id = "NICHT GEFUNDEN"
                match = re.search(
                    r'<meta name="X-Wikidata-Id" content="(Q\d+)"', raw_html
                )
                if match:
                    q_id = match.group(1)

                # 2. HTML entfernen und reinen Text für den Cyberdeck extrahieren
                soup = BeautifulSoup(raw_html, "html.parser")
                clean_text = soup.get_text(separator=" ", strip=True)

                # --- Ergebnisse ausgeben ---
                print("=" * 60)
                print(f"ARTIKEL INDEX : {target_index} (Alphabetische Position)")
                print(f"INTERNE ZIM ID: {i} (Die tatsächliche ID im Archiv)")
                print("=" * 60)
                print(f"Titel         : {title}")
                print(f"ZIM Pfad      : {path}")
                print(f"Wikidata Q-ID : {q_id}")
                print(f"Mime-Type     : {item.mimetype}")
                print(f"Dateigröße    : {item.size} Bytes (HTML)")
                print(f"Textlänge     : {len(clean_text)} Zeichen (Clean Text)")
                print("-" * 60)
                print(f"TEXT VORSCHAU :\n{clean_text[:400]}...")
                print("=" * 60)

                return  # Wir haben ihn gefunden, Funktion beenden

            # Falls es nicht der gesuchte Index war, Zähler erhöhen und weitergehen
            article_counter += 1

    print(
        f"Artikel mit Index {target_index} nicht gefunden. Die Datei enthält nur {article_counter} echte Artikel."
    )


if __name__ == "__main__":
    TARGET_ZIM = "../zim_files/wikipedia_de_all_nopic_2026-01.zim"
    WUNSCH_INDEX = 0
    get_ith_article(TARGET_ZIM, WUNSCH_INDEX)
