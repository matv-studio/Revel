#!/usr/bin/env python3
"""Copie publique de la note de passation (docs/ETAT.md).

Usage : python3 tools/etat-public.py <revel-etat.md>

La note complète vit dans le projet claude.ai « Revel » ; cette copie, dans le dépôt public, permet de
reprendre le travail depuis un autre compte. Les emplacements propres à l'ordinateur de l'utilisateur,
son nom de machine, l'adresse de l'agence et l'adresse du chat sont remplacés.
"""
import pathlib
import re
import sys

HEAD = """# Revel — état du projet (copie publique de la note de passation)

Copie de la note tenue par Claude dans le projet claude.ai « Revel » (`claude/revel-etat.md`), pour
reprendre le travail depuis n'importe quel compte ou chat : modifications faites, problèmes rencontrés,
solutions, méthodes et pièges. Les emplacements propres à l'ordinateur de l'utilisateur sont remplacés par
`<dossier Revel du PC>` ; l'adresse du chat en cours est donnée par le rappel système de chaque nouveau chat.
Produite par `tools/etat-public.py`, mise à jour en même temps que la note (règle de la section « Reprise »).

"""

REPLACE = [
    ("G:\\Softwares\\Revel", "<dossier Revel du PC>"),
    ("C:\\MATV_LOCAL\\_Interne\\Revel", "<ancien dossier du PC>"),
    ("(`shaddh-home`, Windows)", "(Windows)"),
    ("jamais contact@alunites.com", "jamais l'adresse de l'agence"),
]


def main() -> None:
    src = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
    for a, b in REPLACE:
        src = src.replace(a, b)
    src = re.sub(r"https://claude\.ai/code/session_\w+", "<adresse du chat>", src)
    src = re.sub(r"^# Revel — état du projet\n\n", "", src)
    out = pathlib.Path(__file__).resolve().parent.parent / "docs" / "ETAT.md"
    out.write_text(HEAD + src, encoding="utf-8")
    left = re.findall(r"MATV_LOCAL|Softwares|shaddh|session_\w{10,}|contact@", HEAD + src)
    if left:
        sys.exit(f"reste à remplacer : {sorted(set(left))}")
    print(f"{out} écrit")


if __name__ == "__main__":
    main()
