#!/usr/bin/env python3
"""Ce qui dépend de l'environnement de publication, comparé entre deux sites produits.

Usage : compare-robots.py DOSSIER_A DOSSIER_B

Un site reconstruit dans la forme de l'interface (D-90) doit garder ce que la recette ajoute :
fichiers du serveur (`.htaccess`, `robots.txt`) identiques, et pour chaque page HTML les mêmes
consignes aux moteurs de recherche (`<meta name="robots">`), dans le même nombre. Le reste des
pages est comparé par tools/compare-styles.mjs. Code de sortie 1 en cas d'écart.
"""
import html
import os
import re
import sys

ROBOTS = re.compile(r'<meta\b[^>]*\bname=["\']robots["\'][^>]*>', re.I)
CONTENT = re.compile(r'\bcontent=["\']([^"\']*)["\']', re.I)


def robots(path):
    with open(path, encoding='utf-8', errors='replace') as f:
        text = f.read()
    head = text.split('</head>', 1)[0]
    return sorted(
        html.unescape(m.group(1)).replace(' ', '')
        for t in ROBOTS.findall(head)
        for m in [CONTENT.search(t)]
        if m
    )


def main():
    a, b = sys.argv[1], sys.argv[2]
    diffs = []
    pages = 0
    for root, _, files in os.walk(a):
        for name in files:
            rel = os.path.relpath(os.path.join(root, name), a)
            other = os.path.join(b, rel)
            if name in ('.htaccess', 'robots.txt'):
                if not os.path.exists(other):
                    diffs.append(f'{rel} : absent de B')
                elif open(os.path.join(a, rel), 'rb').read() != open(other, 'rb').read():
                    diffs.append(f'{rel} : contenu différent')
            elif name.endswith('.html'):
                pages += 1
                if not os.path.exists(other):
                    diffs.append(f'{rel} : absente de B')
                    continue
                ra, rb = robots(os.path.join(a, rel)), robots(other)
                if ra != rb:
                    diffs.append(f'{rel} : robots {ra} / {rb}')
    print(f'{pages} pages, fichiers du serveur et consignes aux moteurs comparés : {len(diffs)} écart(s).')
    for d in diffs[:30]:
        print(f'- {d}')
    sys.exit(1 if diffs else 0)


if __name__ == '__main__':
    main()
