"""Puts Wowhead list pages you saved yourself into the shared download cache.

Wowhead's profession list pages answer scripts with a bot challenge, which the
generators do not bypass. Open each page in your own browser, save it as HTML
(Ctrl+S, "Webpage, HTML only" is enough) into one folder, and import the folder:

    python3 data-generators/import_wowhead_pages.py --list         # the 18 pages to save
    python3 data-generators/import_wowhead_pages.py ~/Downloads/wowhead
    python3 data-generators/run_all.py --offline --only gen_professions,gen_enhancements

A saved file is matched to its page by the canonical / og:url address in the HTML;
if the browser dropped it, name the file exactly like the cache entry
(page_alchemy.html, recipeitems_alchemy.html; see --list). Each page must still
contain Wowhead's embedded list (listviewspells / listviewitems); anything else
is refused, so a challenge or login page can never enter the cache. The recipe
and item tooltips are not affected: they download normally.
"""
import argparse
import hashlib
import json
import pathlib
import re
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE / "TALOD-data-generators" / "tools"))
from navzeroth_datagen import http  # noqa: E402

SPELLS = "https://www.wowhead.com/classic/spells/"
RECIPES = "https://www.wowhead.com/classic/items/recipes/"
# Wowhead list path under /classic/spells/ (the slugs gen_professions and gen_enhancements read).
SLUGS = ["professions/alchemy", "professions/blacksmithing", "secondary-skills/cooking", "professions/enchanting",
         "professions/engineering", "secondary-skills/first-aid", "professions/leatherworking",
         "professions/tailoring", "professions/mining"]


def expected():
    """{url: (cache name, marker the page must contain)} for every page the generators read."""
    pages = {}
    for slug in SLUGS:
        last = slug.split("/")[-1]
        pages[SPELLS + slug] = (f"page_{last}.html", "var listviewspells")
        pages[RECIPES + last] = (f"recipeitems_{last}.html", "var listviewitems")
    return pages


def identify(name, text, pages):
    """The page URL a saved file stands for, or None."""
    for m in re.finditer(r'(?:rel="canonical"[^>]*href|property="og:url"[^>]*content)="([^"]+)"', text):
        url = m.group(1).split("#")[0].split("?")[0].rstrip("/")
        if url in pages:
            return url
    for url, (cache_name, _) in pages.items():
        if name == cache_name:
            return url
    return None


def store(url, cache_name, data):
    body, meta = http._paths(url, cache_name)
    body.parent.mkdir(parents=True, exist_ok=True)
    body.write_bytes(data)
    meta.write_text(json.dumps({"url": url, "status": 200, "retrieved_at_utc_ms": int(time.time() * 1000),
                                "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data),
                                "imported": True}, sort_keys=True), encoding="utf-8")


def import_dir(folder, pages=None):
    """Import every .html in folder; returns (imported urls, problems)."""
    pages = pages or expected()
    done, problems = set(), []
    for path in sorted(pathlib.Path(folder).glob("*.htm*")):
        data = path.read_bytes()
        text = data.decode("utf-8", errors="replace")
        url = identify(path.name, text, pages)
        if url is None:
            problems.append(f"{path.name}: not one of the expected pages (no canonical address, unknown name)")
            continue
        cache_name, marker = pages[url]
        if marker not in text:
            problems.append(f"{path.name}: {url} has no embedded list ({marker}); save the page again")
            continue
        store(url, cache_name, data)
        done.add(url)
    problems += [f"missing: {u}" for u in pages if u not in done]
    return sorted(done), problems


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("folder", nargs="?", help="folder with the saved .html pages")
    p.add_argument("--list", action="store_true", help="print the pages to save and exit")
    p.add_argument("--cache", help="download cache folder (default data-generators/.cache)")
    args = p.parse_args(argv)
    if args.cache:
        http.configure(cache=args.cache)
    if args.list or not args.folder:
        for url, (name, _) in expected().items():
            print(f"{url}\n    save as {name}")
        return 0
    done, problems = import_dir(args.folder)
    print(f"imported {len(done)} of {len(expected())} pages")
    for line in problems:
        print("  " + line)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
