#!/usr/bin/env python3
"""Produit crates/revel-translate/src/catalog.rs (D-36) : langues de traduction téléchargeables.

Pour chaque langue, deux modèles Marian OPUS-MT de Helsinki-NLP (vers et depuis l'anglais) :
révision figée (branche principale, sinon conversion `model.safetensors` proposée par le robot de
Hugging Face dans une demande de fusion), poids lisibles sans Python (`model.safetensors` ou
`flax_model.msgpack`), empreintes SHA-256 de chaque fichier.

Usage : python3 -I tools/models-catalog.py fr,de,es,…   (bibliothèque standard seulement)
"""
import concurrent.futures as cf
import hashlib
import json
import pathlib
import sys
import urllib.request

API = "https://huggingface.co/api/models/Helsinki-NLP/"
SMALL = ("config.json", "source.spm", "vocab.json")
OUT = pathlib.Path(__file__).resolve().parent.parent / "crates/revel-translate/src/catalog.rs"


def get(url):
    try:
        with urllib.request.urlopen(url, timeout=60) as r:
            return json.load(r)
    except Exception:
        return None


def tree(repo, rev):
    t = get(f"{API}{repo}/tree/{rev}")
    return {f["path"]: f for f in t} if isinstance(t, list) else None


def source(repo):
    """Révision et fichiers d'un modèle, ou None."""
    info = get(API + repo)
    if not info:
        return None
    # Branche principale, puis demandes de fusion de la plus ancienne à la plus récente.
    prs = (get(f"{API}{repo}/refs?include_prs=1") or {}).get("pullRequests", [])
    prs.sort(key=lambda p: int(p["ref"].rsplit("/", 1)[-1]))
    revs = [info["sha"]] + [p["targetCommit"] for p in prs]
    for rev in revs:
        t = tree(repo, rev)
        if not t or not all(k in t for k in SMALL):
            continue
        weights = next((w for w in ("model.safetensors", "flax_model.msgpack") if w in t), None)
        if not weights:
            continue
        files = []
        for name in SMALL:
            with urllib.request.urlopen(f"https://huggingface.co/Helsinki-NLP/{repo}/resolve/{rev}/{name}", timeout=60) as r:
                b = r.read()
            files.append((name, len(b), hashlib.sha256(b).hexdigest()))
        files.append((weights, t[weights]["size"], t[weights]["lfs"]["oid"]))
        return {"rev": rev, "files": files}
    return None


def rust_source(name, s):
    files = ",\n".join(
        f'                    RemoteFile {{\n                        name: "{n}",\n                        size: {size},\n                        sha256: "{h}",\n                    }}'
        for n, size, h in s["files"]
    )
    return f'            Source {{\n                name: "{name}",\n                rev: "{s["rev"]}",\n                files: &[\n{files},\n                ],\n            }}'


def main():
    langs = sys.argv[1].split(",")
    repos = [f"opus-mt-{l}-en" for l in langs] + [f"opus-mt-en-{l}" for l in langs]
    with cf.ThreadPoolExecutor(6) as ex:
        found = dict(zip(repos, ex.map(source, repos)))
    text = OUT.read_text()
    head, _, rest = text.partition("pub static PACKS: &[Pack] = &[\n")
    _, _, tail = rest.partition("\n];\n")
    packs = []
    for l in langs:
        a, b = found[f"opus-mt-{l}-en"], found[f"opus-mt-en-{l}"]
        if not a or not b:
            print(f"{l} : modèle manquant, langue ignorée", file=sys.stderr)
            continue
        packs.append(
            f'    Pack {{\n        code: "{l}",\n        models: [\n{rust_source(f"opus-mt-{l}-en", a)},\n{rust_source(f"opus-mt-en-{l}", b)},\n        ],\n    }},'
        )
    OUT.write_text(head + "pub static PACKS: &[Pack] = &[\n" + "\n".join(packs) + "\n];\n" + tail)
    print(f"{len(packs)} langues écrites dans {OUT}")


if __name__ == "__main__":
    main()
