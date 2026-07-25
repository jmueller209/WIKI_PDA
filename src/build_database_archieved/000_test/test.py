import gzip
import json

x = 10000
lines_to_skip = 1

ALLOWED_PROPERTY_TYPES = [
    "external-id",
    "string",
    "quantity",
    "math",
    "wikibase-item",
    "time",
    "musical-notation",
    "wikibase-form",
    "globe-coordinate",
    "geo-shape",
    "wikibase-lexeme",
    "monolingualtext",
    "wikibase-property",
    "wikibase-sense",
    "tabular-data",
]

available_datatypes = []
with gzip.open("./zim_files/latest-all.json.gz", "rt", encoding="utf-8") as f:
    for i, line in enumerate(f):
        if i == x:
            break
        if i < lines_to_skip:
            continue

        clean_line = line.strip().rstrip(",")
        if (
            not clean_line
        ):  # Skip empty lines (like the opening/closing brackets of the JSON dump)
            continue

        try:
            data = json.loads(clean_line)
        except json.JSONDecodeError:
            continue

        ID = data.get("id")
        TYPE = data.get("type")
        LABELS = data.get("labels")
        DESCRIPTIONS = data.get("descriptions")
        ALIASES = data.get("aliases")

        if TYPE == "item":
            SITELINKS = data.get("sitelinks")
            DATATYPE = None
        elif TYPE == "property":
            DATATYPE = data.get("datatype")
            SITELINKS = None
        else:
            SITELINKS = None
            DATATYPE = None

        # print all the values
        print(f"ID: {ID}")
        print(f"TYPE: {TYPE}")
        print(f"DATATYPE: {DATATYPE}")
        # print(f"LABELS: {LABELS}")
        # print(f"DESCRIPTIONS: {DESCRIPTIONS}")
        # print(f"ALIASES: {ALIASES}")
        # print(f"SITELINKS: {SITELINKS}")

        if not DATATYPE in available_datatypes and DATATYPE is not None:
            available_datatypes.append(DATATYPE)


print(f"Available datatypes: {available_datatypes}")
