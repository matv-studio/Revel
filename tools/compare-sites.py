#!/usr/bin/env python3
"""Compare deux sites générés (référence Eleventy / Revel).

Usage : compare-sites.py <référence> <revel> [--details]

Pour chaque page HTML : titre, balises meta/link, texte visible, liens, images (nom
d'origine et texte alternatif), structure des balises. Les fichiers texte (XML,
robots.txt, .htaccess) sont comparés ligne à ligne, espaces normalisés.
Les différences attendues sont neutralisées : paramètre de version `?v=…`,
variantes d'images (`<picture>`, `srcset`, noms `nom-abc123-800.webp`).
Code de sortie 0 si aucune différence.
"""
import re
import sys
from pathlib import Path

from bs4 import BeautifulSoup, Comment

VERSION = re.compile(r"\?v=[0-9a-z]+")
VARIANT = re.compile(r"-[A-Za-z0-9_-]{6}-\d{2,4}\.(avif|webp|jpe?g|png|gif)$", re.I)
PICTURE = re.compile(r"</?picture[^>]*>|<source[^>]*>", re.I)
WS = re.compile(r"\s+")


def media_name(src: str) -> str:
    name = src.split("?")[0].rsplit("/", 1)[-1]
    name = VARIANT.sub("", name)
    for _ in range(2):
        name = re.sub(r"\.(avif|webp|jpe?g|png|gif|svg)$", "", name, flags=re.I)
    return name


def norm(s: str) -> str:
    return WS.sub(" ", VERSION.sub("", s or "")).strip()


def html_facts(text: str) -> dict:
    # <picture>/<source> : variantes d'images, neutralisées avant analyse.
    soup = BeautifulSoup(PICTURE.sub("", text), "lxml")
    for c in soup.find_all(string=lambda t: isinstance(t, Comment)):
        c.extract()
    head = soup.head or soup
    def meta_value(m):
        key = m.get("name") or m.get("property") or m.get("charset") or ""
        v = norm(m.get("content", ""))
        return (key, media_name(v) if key in ("og:image", "twitter:image") else v)

    metas = sorted(meta_value(m) for m in head.find_all("meta"))
    links = sorted((" ".join(l.get("rel", [])), norm(l.get("href", ""))) for l in head.find_all("link"))
    scripts = [norm(s.get_text()) for s in soup.find_all("script") if s.get("type") == "application/ld+json"]
    body = soup.body or soup
    for s in body.find_all(["script", "style", "noscript"]):
        if s.get("type") != "application/ld+json":
            s.decompose()
    images = []
    for img in body.find_all("img"):
        images.append((media_name(img.get("src", "")), norm(img.get("alt", "")), img.get("class") or []))
    for pic in body.find_all("picture"):
        pic.unwrap()
    for s in body.find_all("source"):
        s.decompose()
    anchors = [(norm(a.get("href", "")), norm(a.get_text(" "))) for a in body.find_all("a")]
    tags = [t.name for t in body.find_all(True)]
    attrs = []
    for t in body.find_all(True):
        for k, v in sorted(t.attrs.items()):
            if t.name == "img" and k in ("src", "srcset", "sizes", "width", "height", "decoding", "fetchpriority", "style"):
                continue
            if isinstance(v, list):
                v = " ".join(v)
            attrs.append((t.name, k, norm(str(v))))
    return {
        "title": norm(soup.title.get_text() if soup.title else ""),
        "metas": metas,
        "links": links,
        "jsonld": scripts,
        "text": norm(body.get_text(" ")),
        "anchors": anchors,
        "images": images,
        "tags": tags,
        "attrs": attrs,
    }


def text_facts(text: str) -> list:
    return [norm(l) for l in text.splitlines() if norm(l)]


def first_diff(a, b):
    if isinstance(a, list) and isinstance(b, list):
        for i, (x, y) in enumerate(zip(a, b)):
            if x != y:
                return f"élément {i} : {x!r} ≠ {y!r}"
        return f"longueurs {len(a)} ≠ {len(b)}"
    if isinstance(a, str) and isinstance(b, str):
        for i, (x, y) in enumerate(zip(a, b)):
            if x != y:
                return f"position {i} : …{a[max(0, i - 60):i + 60]!r}… ≠ …{b[max(0, i - 60):i + 60]!r}…"
        return f"longueurs {len(a)} ≠ {len(b)}"
    return f"{a!r} ≠ {b!r}"


def main() -> int:
    ref, rev = Path(sys.argv[1]), Path(sys.argv[2])
    details = "--details" in sys.argv
    pattern = lambda p: p.suffix in (".html", ".xml", ".txt", ".php") or p.name == ".htaccess"
    files_ref = {p.relative_to(ref).as_posix() for p in ref.rglob("*") if p.is_file() and pattern(p)}
    files_rev = {p.relative_to(rev).as_posix() for p in rev.rglob("*") if p.is_file() and pattern(p)}
    problems = 0
    for f in sorted(files_ref - files_rev):
        print(f"ABSENT de Revel : {f}")
        problems += 1
    for f in sorted(files_rev - files_ref):
        print(f"EN PLUS dans Revel : {f}")
    ok = 0
    for f in sorted(files_ref & files_rev):
        a = (ref / f).read_text(encoding="utf-8", errors="replace")
        b = (rev / f).read_text(encoding="utf-8", errors="replace")
        if f.endswith(".html"):
            fa, fb = html_facts(a), html_facts(b)
            diffs = [k for k in fa if fa[k] != fb[k]]
            if diffs:
                problems += 1
                print(f"DIFFÉRENT {f} : {', '.join(diffs)}")
                if details:
                    for k in diffs:
                        print(f"    {k} : {first_diff(fa[k], fb[k])}")
            else:
                ok += 1
        else:
            ta, tb = text_facts(a), text_facts(b)
            if f == "sitemap.xml":  # ordre sans signification
                ta, tb = sorted(ta), sorted(tb)
            if ta != tb:
                problems += 1
                print(f"DIFFÉRENT {f}")
                if details:
                    print(f"    {first_diff(ta, tb)}")
            else:
                ok += 1
    print(f"\n{ok} fichiers identiques, {problems} différences, {len(files_ref)} fichiers de référence")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
