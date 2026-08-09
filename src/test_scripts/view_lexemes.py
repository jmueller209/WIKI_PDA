import gzip
import json


dump_path = "latest-lexemes.json.gz"
max_entries_to_inspect = 50

print(f"Opening {dump_path} and inspecting summary info...")

printed_count = 0
with gzip.open(dump_path, "rt", encoding="utf-8") as f:
    for i, line in enumerate(f):
        stripped = line.strip()
        if stripped in ("[", "]"):
            continue

        if stripped.endswith(","):
            stripped = stripped[:-1]

        try:
            entry = json.loads(stripped)
            if entry.get("type") == "lexeme":

                lexeme_id = entry.get("id", "Unknown")


                lemmas = entry.get("lemmas", {})
                lemma_text = "N/A"
                lang_code = "N/A"
                if lemmas:
                    lang_code, lemma_data = next(iter(lemmas.items()))
                    lemma_text = lemma_data.get("value", "N/A")

                lexical_category = entry.get("lexicalCategory", "N/A")


                forms = [
                    f.get("representations", {}).get(lang_code, {}).get("value")
                    for f in entry.get("forms", [])
                ]
                forms_str = ", ".join([f for f in forms if f]) or "None"


                print(
                    f"[{lexeme_id}] ({lang_code}) Lemma: '{lemma_text}' | Category: {lexical_category}"
                )
                print(f"    Forms: [{forms_str}]")
                print("-" * 50)

                printed_count += 1
                if printed_count >= max_entries_to_inspect:
                    break

        except json.JSONDecodeError as e:
            continue
