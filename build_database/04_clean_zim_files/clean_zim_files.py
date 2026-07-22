import os
from libzim.reader import Archive
from bs4 import BeautifulSoup
import html2text
import re

h = html2text.HTML2Text()
h.ignore_links = False
h.ignore_images = True
h.body_width = 0


def html_to_text(html_content):
    markdown_text = h.handle(html_content)
    return markdown_text


def get_qid_from_title(title, lang):
    return 1


def clean_html_to_text(html_content):
    soup = BeautifulSoup(html_content, "html.parser")
    body = soup.find("div", class_="mw-parser-output")

    if not body:
        return "No content found."

    # 1. THE "SURGICAL" REMOVAL
    # Remove all tables (Infoboxes, data tables)
    for table in body.find_all("table"):
        table.decompose()

    # Remove all "divs" that are typically navboxes, metadata, or sidebars
    for div in body.find_all(
        "div", class_=["navbox", "metadata", "printfooter", "mw-editsection"]
    ):
        div.decompose()

    # Remove all citation brackets like [1], [2] (usually inside <sup>)
    for sup in body.find_all("sup", class_="reference"):
        sup.decompose()

    # 2. CONVERT TO CLEAN MARKDOWN
    markdown_text = h.handle(str(body))

    return markdown_text


def normalize_links(text, current_lang):
    pattern = r'\[(.*?)\]\((.*?)(?:\s+".*?")?\)'

    def replace_link(match):
        text = match.group(1)
        target = match.group(2)

        # 1. REMOVE NON-WIKI LINKS
        # If it's an external URL (http, https, ftp), just return the plain text
        if target.startswith(("http", "ftp", "mailto")):
            return text

        # 2. RESOLVE WIKI LINKS TO Q-IDS
        # Clean the target (remove internal link prefixes if needed)
        clean_target = target.replace("_", " ")
        qid = get_qid_from_title(clean_target, current_lang)

        if qid:
            return f"[[{text}|{qid}]]"
        else:
            # If QID not found, return plain text (it's a dead/broken link)
            return text

    return re.sub(pattern, replace_link, text)


def cleanup_stray_brackets(text):
    text = re.sub(r"(?<!\[)\[(?!\[)", "", text)  # Remove '[' not followed by '['
    text = re.sub(r"(?<!\])\](?!\])", "", text)  # Remove ']' not followed by ']'
    return text


def save_text_to_file(text, title, output_dir):
    # Create a safe filename (remove weird characters)
    safe_title = "".join(c if c.isalnum() else "_" for c in title)
    out_file = os.path.join(output_dir, f"{safe_title}.md")

    with open(out_file, "w", encoding="utf-8") as f:
        f.write(f"TITLE: {title}\n")
        f.write("=" * 40 + "\n\n")
        f.write(text)


def process_zim_test(zim_path, output_dir, limit=5):
    print(f"📚 Opening ZIM archive: {zim_path}")
    archive = Archive(zim_path)

    os.makedirs(output_dir, exist_ok=True)

    count = 0
    # Loop through all entries using the integer index
    for i in range(archive.all_entry_count):
        entry = archive._get_entry_by_id(i)

        # 1. Skip redirects immediately
        if entry.is_redirect:
            continue

        try:
            item = entry.get_item()
        except Exception:
            continue  # Skip broken entries

        # 2. Only process actual HTML files
        if item.mimetype == "text/html":
            title = entry.title if hasattr(entry, "title") else entry.path
            print(f"⚙️ Processing: {title}")

            # Read raw bytes and decode to string
            raw_html = item.content.tobytes().decode("utf-8", errors="ignore")

            # Strip it down to pure text
            clean_text = clean_html_to_text(raw_html)
            clean_text = normalize_links(clean_text, current_lang="en")
            clean_text = cleanup_stray_brackets(clean_text)

            dirty_text = html_to_text(raw_html)

            save_text_to_file(clean_text, title, output_dir)
            save_text_to_file(dirty_text, title + "_dirty", output_dir)

            count += 1
            if count >= limit:
                break

    print(f"\n✅ Done! Processed {count} articles. Check the '{output_dir}' folder.")


if __name__ == "__main__":
    # ⚠️ CHANGE THIS TO YOUR ACTUAL ZIM FILE PATH
    ZIM_FILE = "../../zim_files/wikipedia_en_all_nopic_2026-06.zim"
    OUTPUT_FOLDER = "./zim_test_output"

    if os.path.exists(ZIM_FILE):
        # Change limit=5 to whatever number you want to test
        process_zim_test(ZIM_FILE, OUTPUT_FOLDER, limit=5)
    else:
        print(f"🚨 ERROR: Could not find ZIM file at: {ZIM_FILE}")
