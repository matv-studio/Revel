# Revel — état du projet (copie publique de la note de passation)

Copie de la note tenue par Claude dans le projet claude.ai « Revel » (`claude/revel-etat.md`), pour
reprendre le travail depuis n'importe quel compte ou chat : modifications faites, problèmes rencontrés,
solutions, méthodes et pièges. Les emplacements propres à l'ordinateur de l'utilisateur sont remplacés par
`<dossier Revel du PC>` ; l'adresse du chat en cours est donnée par le rappel système de chaque nouveau chat.
Produite par `tools/etat-public.py`, mise à jour en même temps que la note (règle de la section « Reprise »).

Mis à jour le 27/10/2026 (dernière avancée : version 0.7.1, 88de8d4). Section « Reprise » ajoutée le 21/10/2026 pour continuer
dans un nouveau chat du projet « Sites Webs » sans perdre la façon de travailler.

## Travail du 27/10/2026 depuis l'ancien compte (projet « Sites Webs »), après la 0.7.0
Commits 4ce84f4 (CI réussie) et e1a6b98 ; D-142 à D-150 ; ligne 8.13
de `docs/ETAPE-8.md`.
- D-142 colonnes du CMS réordonnées au glisser, vignettes d'image (chemin au survol) ; D-143 vignettes des sites
  (couleur ou capture de l'accueil, `sitethumb.rs`, `thumb_file`) ; galerie des templates qui défile ; contenus
  sous leur collection dans le plan du site ; transitions de page ajoutées, aperçu lent au survol, icônes
  distinctes ; boutons d'écran de l'aperçu centrés ; aperçu de composant à l'échelle du canevas, isolé.
- D-144 catalogues de l'utilisateur (Réglages + « + » dans Nouveau site) ; D-145 « Modèle des pages » devenu
  une page interne de Revel (`/__revel/template/<id>/`, `TEMPLATE_ENTRY = "__revel-template"`, jamais écrit
  sur disque) qui réunit tous les champs de la collection (`crates/revel-core/src/sample.rs`) ; une liste de
  blocs y reçoit un bloc de chaque sorte (D-150).
- D-146 classes dans les Paramètres avancés (boutons à la charte, retirables, Ctrl+Z) ; D-147 corrections.
- D-148 Paramètres › Général › Tailles (`apps/revel-ui/src/sitesizes.rs`) : variables de taille du site,
  renommées partout (commande `RenameVariable`, `revel-edit/src/variables.rs`), valeur fixe ou fluide
  (min → max). Règle de l'utilisateur : **une variable seulement si elle est partagée ; sinon la classe porte
  la valeur** (les tailles de texte d'Alunites ont été remises dans les classes).
- D-149 interrupteur clair / sombre (`crates/revel-render/src/theme.rs`, préréglage `theme_switch`, Ajouter ›
  Interrupteur de thème, option « theme » du réglage d'interrupteur proposée si le site a des couleurs
  sombres) : réécrit les requêtes `prefers-color-scheme` des feuilles avant le premier affichage, choix gardé
  (`localStorage` `rv-theme`), `html[data-revel-scheme]`. **Le site produit par la 0.7.0 n'a pas ce script :
  il faut la version suivante.**
- Règle globale de l'utilisateur (27/10) : animations par les outils de Revel, jamais de code ; un assistant
  (MCP) passe toujours par les outils d'interface qui laissent la main à l'utilisateur, sauf demande contraire.
- Recréation d'Alunites refaite (PC de l'ancien compte : `recreation-alunites-interface`, `site.revel.json`) : nouvelle direction
  « Rayonner » (fond papier, panneaux d'encre, en-tête en capsule flottante, accent orange et icônes gardés),
  variante sombre complète, interrupteur dans l'en-tête ; animations par `data-revel-text`,
  `data-revel-appear`, rosace `data-revel-loop`. Composants rendus modifiables depuis la page : Chiffres et
  Clients détachés sur l'accueil et l'agence, CTA à propriétés (titre, accent, aparté, texte, bouton, lien),
  cartes avec valeurs par défaut, `partials/blocks` détaché dans les gabarits projet et article. `check_site`
  : tout éditable.
- Essais : éditeur 348/348, templates 26/26, tests du dépôt, clippy, format.
- Pièges : `revel build` sur Alunites remplit le disque (variantes d'images) : ne pas le lancer ; wasm-bindgen
  tué si des ponts tournent : les arrêter avant `trunk build` ; une image paresseuse d'un cadre hors écran ne se
  charge jamais (cadre dans la vue à opacité 0, images passées en `eager`).
- **Version 0.7.1** (27/10/2026) : commit 88de8d4 « Version 0.7.1 », CI entièrement réussie (flux 40516107297) ;
  étiquette `v0.7.1` (flux « Étiquette » 40522665705) ; brouillon « Revel v0.7.1 » (préversion, id 425599865,
  17 fichiers, `latest.json` 0.7.1 à 11 cibles, clé EA5F2855415468CD), **à publier par l'utilisateur**. Contient
  l'interrupteur clair / sombre du site produit (D-149).


## Travail du 26/10/2026 depuis l'ancien compte (projet « Sites Webs »), après la 0.6.9
Commits 933e49c, cd65eca et dd29dda sur `main` ; CI entièrement réussie sur dd29dda (flux 40388623717). Décisions D-126 à D-139 (D-132 à trancher : modèles partagés en ligne) ;
`docs/ETAPE-8.md`, lignes 8.10 et 8.11. À intégrer à la note du projet « Revel » :
- D-126 liens de variantes carré + point ; D-127 sites du compte GitHub (liste, téléchargement, synchroniser).
- D-128 26 modèles de site par métier (`crates/revel-core/templates/`, kit `_kit/`, `build.rs`), galerie dans
  « Nouveau site » ; Médias › images en ligne copiées dans le site (`localize.rs`, `media_online`,
  `media_localize`, outil MCP `copy_online_images`), bandeau discret sous la recherche, retiré une fois fait.
- D-129 panier (`data-revel-cart*`) ; D-130 galerie du CMS ; D-131 volet du CMS réglable, « Modèle des pages ».
- D-133 colonnes du CMS (épingle dans la fiche, menu, largeur) ; D-134 « garder le texte d'origine »
  (Localisation, `Locale.kept`, commande `MarkKept`) ; D-135 contenus sous leur collection (plan du site) ;
  D-136 défilement doux refait ; D-137 Paramètres (fiche Structure + pastille d'environnement en haut à droite,
  Organisation sous « + », Code sous Publication, annuler/rétablir dans la barre du haut) ; D-138 variantes
  de composant (`:hover` réel coupé dans le canevas des composants, `liveOff` de `canvas.js`), aperçu du
  composant seul ou de la page affichée, liens externes de l'aperçu (`_blank` → navigateur, sinon chargés dans
  le cadre ; CSP `frame-src https: http:`) ; D-139 deuxième revue d'ergonomie (reste listé dans D-139).
- Recréation d'Alunites mise à D-119 sur le PC de l'ancien compte (composants « Chiffre », « Logo client »,
  « Offre d'emploi » ; site généré identique).
- Essais : éditeur 341/341, modèles (`tools/templates-e2e.mjs`) 20/20, animations 17/17.
- Consigne de l'utilisateur : pas de 0.7.0 avant que tout soit fini (recréation d'Alunites comprise : faite).
- Pièges : mémoire limitée à 5,8 Go par commande dans la session : `trunk build` seul, avec
  `CARGO_PROFILE_DEV_DEBUG=0` (sinon rustc tué) ; jamais en parallèle d'un build release. Un moniteur expire
  après 5 min : attendre par `Bash` en arrière-plan avec une boucle `until`.

## Version 0.7.0 (27/10/2026, depuis l'ancien compte, projet « Sites Webs »)
Commits f2e1d7a, 2c2c5a4, ecd2fe9 (« Version 0.7.0 », CI entièrement réussie, flux 40419996924). Étiquette
`v0.7.0` (flux « Étiquette » 40423930294) ; brouillon « Revel v0.7.0 » (préversion, id 424911276, 17 fichiers,
`latest.json` 0.7.0 à 11 cibles, clé EA5F2855415468CD), **à publier par l'utilisateur**.
- D-132 prise : fichier de template (`.revel-template`) et catalogue GitHub (`crates/revel-core/src/sharing.rs`,
  `apps/revel-ui/src/templates.rs`), dans Paramètres du site › Publication › Template ; dépôt du catalogue
  `matv-studio/revel-templates` encore à créer (Réglages › Templates › « Créer un catalogue sur GitHub »).
- D-139 terminée, D-140 (« Nouveau site » compact : Vide, template au hasard, Plus), D-141 (interface en
  15 langues ; ajouter une clé = l'ajouter dans les 15 fichiers `locales/<code>/revel.ftl`).
- Mot « template » gardé en français pour les sites prêts à adapter.

## Compte et emplacements actuels (depuis le 24/10/2026 — prime sur les chemins plus bas)
- Travail repris sur un autre compte Claude, projet claude.ai **« Revel »** (ce chat lui est dédié). Les chats
  et le projet « Sites Webs » de l'ancien compte ne sont pas lisibles d'ici : cette note est le seul relais.
- GitHub relié : **matv-studio** (Revel et alunites-revel-lab en écriture ; `gh api` REST fonctionne).
- Dossier du PC (Windows) : **`<dossier Revel du PC>`** pour tout (installateurs, sites, remises,
  copie de cette note). Contenu trouvé : `revel-app.exe`, `uninstall.exe` (installation de Revel),
  `Projects\Alunites`, `Projects\p3r-deltea-space` (sites de Revel). Rangement de Claude :
  `_Claude\Passation` (cette note), `_Claude\Installateurs`, `_Claude\Livrables`, `_Claude\Captures`.
  Les chemins `<ancien dossier du PC>\…` plus bas concernent l'ancien PC et ne sont plus utilisés.
- Session de code : `Claude-Session: <adresse du chat>`.
- Document d'avancement : créé sur l'ancien compte ; s'il n'est pas lisible, en recréer un dans ce projet.
- Environnement remis en place le 24/10/2026 (dossier de travail `SP=/home/claude/sp`, scripts `verify.sh` et
  `run-e2e-ci.sh` recréés) : Rust 1.99.0 (wasm32, musl), trunk 0.21.14, paquets Tauri, nasm, musl-tools, sshd.
  Vérification complète sur 7712ad1 (0.6.6) : construction de `revel` (18 min sur 2 cœurs) et de l'interface,
  tests Rust tous réussis, essais de l'interface 315/315, animations 17/17, cobaye inchangé. Disque : 22 Go
  utilisés sur environ 44.

## Reprise dans un nouveau chat (à lire en premier)

Premier message conseillé : « Lis claude/revel-etat.md (section Reprise d'abord), remets l'environnement en
place, puis … ». Tout ce qui suit vaut sans être redit ; l'historique complet est plus bas.

Le chat en cours tient cette note à jour à chaque avancée (méthodes, outils, pièges, état) et l'enregistre à trois
endroits : le projet (`claude/revel-etat.md`), le PC (`<dossier Revel du PC>\_Claude\Passation\revel-etat.md`) et le
dépôt (`docs/ETAT.md`, copie publique produite par `python3 tools/etat-public.py <note>` : emplacements du PC et
adresse du chat remplacés ; commit avec le reste du travail). Depuis un autre compte Claude sans accès au projet :
lire `docs/ETAT.md` sur https://github.com/matv-studio/Revel (dépôt public).

### 1. Règles de Claude pour ce projet
**Réponses**
- Ton neutre, technique, direct : ni compliment, ni validation, ni formule d'introduction ; commencer par la
  réponse ou l'action. Plusieurs solutions : les lister avec leurs contreparties, sans en désigner une comme
  « la bonne ».
- Avant tout diagnostic ou conseil technique : 1) vérifier par une recherche web, 2) croiser avec les
  informations et captures fournies, 3) confirmer la cohérence ; seulement ensuite conseiller. Jamais de
  supposition. Sources en fin de réponse.
- En fin de travail : le résultat en quelques lignes, les captures, une suite possible ; pas de récit des étapes.

**Produit** : voir « Exigences permanentes de l'utilisateur » plus bas. En bref : tout site 100 % éditable depuis
l'interface ; « quand tu te heurtes à une limite, tu crées l'outil » (pour l'utilisateur et pour un LLM par le
MCP) ; ergonomie primordiale ; aucune phrase d'explication visible (info-bulles) ; code couleur de Webflow ;
chaque écran vérifié dans les deux thèmes ; deux fonctions distinctes = deux outils distincts.

**Dépôt et versions**
- Travail directement sur `main` (ni branche ni demande de fusion).
- Auteur : `matv-studio <285979032+matv-studio@users.noreply.github.com>` (`git config user.name/user.email`
  à régler dans chaque nouvelle copie), jamais l'adresse de l'agence.
- Message de commit en français : titre court, corps en phrases citant les D-xxx ; dernières lignes :
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` puis `Claude-Session: <adresse du chat>` (donnée
  par le rappel système du nouveau chat ; elle change d'un chat à l'autre).
- Étiquettes uniquement par le flux `tag.yml`, lancé juste après l'envoi (le flux refuse un commit dont
  `.github/workflows/` diffère de `main`). L'envoi direct d'étiquette est refusé (403).
- **Jamais de publication de version** : Claude crée le brouillon (flux « Installateurs »), l'utilisateur le
  publie. Anciens brouillons gardés. Réglages persistants (secrets, clés, réglages du dépôt) : jamais par Claude.
- API GraphQL de GitHub refusée depuis la session : `gh api` (REST) uniquement.
- Ne pas relancer la construction npm complète de p3r sans accord.
- Chaque fonction livrée : décision D-xxx dans `docs/DECISIONS.md`, ligne dans `docs/ETAPE-N.md`, textes FR et
  EN (`crates/revel-i18n/locales/{fr,en}/revel.ftl`), essais dans `tools/editor-e2e.mjs`, captures dans les deux
  thèmes envoyées à l'utilisateur, puis cette note et le document d'avancement mis à jour.
- Une demande ajoutée en cours de route (« once finished, also run a new build ») entre dans la liste des tâches.

**Économie de contexte** (coût de chaque échange) : sorties de commandes filtrées (`tail`, `grep`, journaux
écrits dans des fichiers), pas de lecture intégrale des gros fichiers, journaux de CI et recherches bavardes
confiés à un sous-agent, pas de pause de plus d'une heure en plein travail (le cache expire).

### 2. Outils de Claude utilisés
| Outil | Usage pour Revel |
|---|---|
| Bash | cargo, trunk, node et Playwright, git, gh. Commandes longues en arrière-plan ; attente par Monitor (boucle `until`), jamais de `sleep` au premier plan. |
| Read, Edit, Write, Grep, Glob | Fichiers du dépôt (Edit plutôt que des scripts Python : `cargo fmt` déplace le texte). |
| Projects | `project_read` / `project_write` de `claude/revel-etat.md` (cette note) ; copie de travail locale, réécrite en entier. |
| Claude Docs | Document d'avancement « Revel — avancement du projet » (adresse plus bas) : charger la compétence `docs`, puis `guide`, `read`, `update` d'une ligne par sous-étape. |
| WebSearch, WebFetch | Vérification avant tout conseil ; documentation des outils imités (Webflow, Figma, Framer, hébergeurs). |
| SendUserFile | Captures et fichiers remis à l'utilisateur. |
| TaskCreate, TaskUpdate | Liste des tâches de tout travail en plusieurs étapes ; dernière tâche : la vérification. |
| Agent | Relecture indépendante des travaux à fort enjeu ; journaux de CI et recherches longues. |
| add_repo, register_repo_root | Rattacher les dépôts à une nouvelle session (section 3). |
| conversation_search, read_conversation | Retrouver un détail d'un chat précédent (coûte des jetons : cette note d'abord). |
| Outils de l'ordinateur (remote-devices) | Seulement si l'app de bureau est reliée : copie du PC `<ancien dossier du PC>\revel`, remises dans `<ancien dossier du PC>\…` ; shell Linux dans une machine virtuelle (`$HOME/mnt/<dossier>`), binaires pour elle compilés en musl. |

Connecteurs sans usage pour Revel (Higgsfield, Higgsfield Bridge, Webflow) : peuvent rester « À la demande ».

### 3. Remettre l'environnement en place (nouvelle session)
1. `add_repo` `matv-studio/Revel` (accès push) et `matv-studio/alunites-revel-lab` (lecture, le cobaye) ;
   cloner dans `/home/claude/revel` et `/home/claude/alunites-revel-lab` ; `register_repo_root` ; régler
   l'auteur (section 1).
2. Rust : `rustup toolchain install` dans le dépôt (`rust-toolchain.toml` : 1.99.0, wasm32) ;
   `rustup target add x86_64-unknown-linux-musl` pour la machine reliée. trunk 0.21.14 :
   `cargo install trunk --locked` s'il manque.
3. Paquets système : ceux de l'étape « Dépendances système de Tauri » de `.github/workflows/ci.yml`
   (`libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`),
   plus `nasm` (AVIF), `openssh-server` (SFTP), `musl-tools`.
4. Playwright et Chromium déjà présents : `NODE_PATH=/usr/local/lib/node_modules_global`, navigateur dans
   `/opt/pw-browsers` ; ne pas lancer `playwright install`.
5. Serveur SFTP d'essai (comme la tâche `sftp` de la CI) : utilisateur `revelsftp` (mot de passe d'essai
   `essai-sftp-7Qk2`), `sshd` sur 127.0.0.1:2222 (`HostKey`, `PasswordAuthentication yes`, `Subsystem sftp
   /usr/lib/openssh/sftp-server`). **Créer `/home/revelsftp/www`** (`install -d -o revelsftp -g revelsftp`) :
   sans dossier web connu, `connexion_d_essai_et_controles` échoue (`suggested` vide ; constaté le 24/10/2026).
   Configuration et clé de cette session : `/home/claude/sp/sshd/`.
6. Construire : `CARGO_INCREMENTAL=0 cargo build --release -p revel-cli` (→ `target/release/revel`) et
   `cd apps/revel-ui && trunk build` (→ `dist/`). L'interface est un espace de travail à part
   (`apps/revel-ui/Cargo.lock`, `apps/revel-ui/target`).

### 4. Scripts de vérification (à recréer dans le dossier de travail du chat)
`SP` = dossier de travail temporaire du chat. Essais attendus : 315/315 ou plus.

`run-e2e-ci.sh` (essais de l'éditeur, comme la CI) :
```bash
#!/bin/bash
SP=<dossier de travail>
pgrep -x revel | xargs -r kill; sleep 0.5
rm -rf $SP/cobaye-essai; mkdir -p $SP/cobaye-essai
# Copie sans .git (le dépôt local du cobaye est superficiel) :
tar --exclude=./node_modules --exclude=./_site --exclude=./.git -C /home/claude/alunites-revel-lab -cf - . | tar -C $SP/cobaye-essai -xf -
(REVEL_MCP_TOKEN=essai-e2e-0123456789abcdef0123456789abcdef REVEL_MODELS=/nonexistent /home/claude/revel/target/release/revel canvas-serve $SP/cobaye-essai --port 8162 --bridge --mcp-port 8163 > $SP/pont.log 2>&1 &)
(cd /home/claude/revel/apps/revel-ui/dist && python3 -m http.server 8170 --bind 127.0.0.1 > /dev/null 2>&1 &)
sleep 2
cd /home/claude/revel
REVEL_E2E_MCP=http://127.0.0.1:8163/mcp REVEL_MCP_TOKEN=essai-e2e-0123456789abcdef0123456789abcdef NODE_PATH=/usr/local/lib/node_modules_global timeout 900 node tools/editor-e2e.mjs http://127.0.0.1:8170 http://127.0.0.1:8162 $SP/e2e-shots $SP/cobaye-essai
code=$?
# Les essais annulent tout : la copie doit rester identique au cobaye.
cmp /home/claude/alunites-revel-lab/site.revel.json $SP/cobaye-essai/site.revel.json && echo "DOC IDENTIQUE"
diff -rq /home/claude/alunites-revel-lab/src $SP/cobaye-essai/src && echo "SRC IDENTIQUE"
cmp /home/claude/alunites-revel-lab/.pages.yml $SP/cobaye-essai/.pages.yml && echo "PAGES.YML IDENTIQUE"
exit $code
```

`verify.sh` (tout ce que contrôle la CI, avant chaque envoi) :
```bash
#!/bin/bash
SP=<dossier de travail>
cd /home/claude/revel
export CARGO_INCREMENTAL=0
cargo fmt --all && (cd apps/revel-ui && cargo fmt)
cargo clippy -q --workspace --all-targets -- -D warnings 2>&1 | tail -20
cargo test -q --manifest-path apps/revel-ui/Cargo.toml 2>&1 | grep -E "FAILED|panicked" | head -5
(cd apps/revel-ui && cargo clippy -q --target wasm32-unknown-unknown -- -D warnings 2>&1 | tail -20)
REVEL_SFTP_HOST=127.0.0.1 REVEL_SFTP_PORT=2222 REVEL_SFTP_USER=revelsftp REVEL_SFTP_PASSWORD=essai-sftp-7Qk2 REVEL_SFTP_HOME=/home/revelsftp \
  cargo test -q --no-fail-fast --workspace --exclude revel-app > $SP/tests.log 2>&1; echo "tests $?"
grep -E "FAILED|panicked" $SP/tests.log | head -20
(cd apps/revel-ui && trunk build > $SP/trunk.log 2>&1; tail -1 $SP/trunk.log)
cargo build -q --release -p revel-cli 2>&1 | tail -5
bash $SP/run-e2e-ci.sh > $SP/e2e-ci.log 2>&1; echo "e2e $?"
grep -E "essais réussis|IDENTIQUE" $SP/e2e-ci.log; grep -v "^ok " $SP/e2e-ci.log | grep -v IDENTIQUE | head -10
NODE_PATH=/usr/local/lib/node_modules_global timeout 300 node tools/animations-e2e.mjs 2>&1 | tail -3
df -h / | tail -1
```

**Captures d'interface** (deux thèmes) : pont sur une copie du cobaye
(`REVEL_MODELS=/nonexistent target/release/revel canvas-serve <copie> --port 8174 --bridge`) et
`python3 -m http.server 8170` dans `apps/revel-ui/dist` ; script Playwright qui injecte
`apps/revel-ui/dev/mock-tauri.js` (`page.addInitScript`) puis ouvre
`http://127.0.0.1:8170/#lang=fr&theme=dark&accent=8b5cf6&scope=full&bridge=<adresse du pont encodée>`
(`theme=light` pour l'autre thème ; `syncevery=2` : vérification de synchronisation toutes les 2 s ; autres
paramètres lus par `params.get(...)` dans le simulateur). Playwright chargé par
`createRequire('/home/claude/revel/tools/')('playwright')` avec `NODE_PATH` réglé. Toute nouvelle commande Tauri
doit être ajoutée au simulateur (liste des commandes relayées au pont) et à `apps/revel-app/src/lib.rs`.

### 5. Livrer une version
1. Numéro dans : `Cargo.toml` (ligne 9), `apps/revel-ui/Cargo.toml` (ligne 6), `apps/revel-app/tauri.conf.json`,
   `README.md` (lignes 7 et 138), `apps/revel-ui/dev/mock-tauri.js` (`app_info`) ; les deux `Cargo.lock` suivent
   à la construction. Commit « Version X.Y.Z », envoi.
2. CI entièrement réussie sur ce commit (suivi : `gh api "repos/matv-studio/Revel/actions/runs?head_sha=<sha>"`).
   Échec connu sans rapport avec le changement : « Équivalence » (vidéo pas encore prête, donnée traduite
   écrite en retard) → relancer les tâches en échec
   (`gh api -X POST repos/matv-studio/Revel/actions/runs/<id>/rerun-failed-jobs`).
3. `gh workflow run release.yml -f plateformes=tous` et `gh workflow run tag.yml -f tag=vX.Y.Z -f sha=<sha>
   -f message="Revel X.Y.Z"`.
4. Vérifier par `gh api repos/matv-studio/Revel/releases` : brouillon « Revel vX.Y.Z » en préversion, 17
   fichiers, `latest.json` à 11 cibles, signatures à la clé EA5F2855415468CD. Prévenir l'utilisateur : à
   publier par lui.
5. Mettre à jour cette note (historique ci-dessous) et le document d'avancement.

### 6. Pièges connus
- Disque : environ 44 Go alloués (`df` trompe : « Avail » à 0 = allocation épuisée). `target/` ≈ 12 Go,
  `apps/revel-ui/target/` ≈ 6 Go. Plantage SIGBUS à l'édition des liens = disque plein : supprimer
  `target/*/incremental`, `target/release/deps`, les dossiers de travail anciens ; `CARGO_INCREMENTAL=0`.
- `pgrep -f` / `pkill -f` avec un motif présent dans la même ligne de commande se trouvent eux-mêmes (shell tué,
  boucle sans fin) : passer par un script, ou `pgrep -x revel`.
- Un redémarrage du conteneur arrête les processus d'arrière-plan (serveurs, boucles d'attente) : les relancer.
- Copie du cobaye sans `.git` (voir le script) ; sinon l'essai « travail à plusieurs » échoue.
- Étiqueter juste après l'envoi, avant toute modification des workflows.
- Essais de l'éditeur : attentes de 40 s pour ce qui dépend du réseau ou d'une minuterie ; un interrupteur se
  clique par son libellé (la poignée intercepte `page.uncheck`).
- Le nom d'auteur des essais Git vient de la configuration (« Claude » possible) : ne pas tester un nom précis.
- Un redémarrage du conteneur arrête sshd et efface `/home/revelsftp/www` : `mkdir -p /run/sshd; sshd -f $SP/sshd/sshd_config`,
  recréer le dossier (sinon 5 essais SFTP échouent).
- Essais de l'éditeur : un nœud inséré par `invoke('edit_apply')` doit porter ses identifiants (sinon il
  ne se sélectionne pas) ; attendre que l'interface ait repris un retrait fait hors d'elle ; un
  interrupteur se clique par son conteneur `.switch` (la case est au-dessus de la poignée).

### 7. Adresses utiles
- Dépôt : https://github.com/matv-studio/Revel ; cobaye : https://github.com/matv-studio/alunites-revel-lab.
- Document d'avancement : https://claude.ai/code/artifact/a810245c-62be-432d-9d9f-23fe88b5a69f.
- Docs du dépôt : `docs/PLAN.md`, `docs/DECISIONS.md` (D-01 à D-108), `docs/ETAPE-1.md` à `docs/ETAPE-8.md`,
  `docs/DESIGN.md`, `docs/ARCHITECTURE.md`, `docs/HEBERGEURS.md`.

## Exigences permanentes de l'utilisateur (interface)
- Niveau Webflow, Framer, Figma pour des personnes non techniques ; code couleur de Webflow (bleu réglé ici ou
  propre à la langue, orange hérité, vert composants), étendu à tout élément qu'il rend plus lisible.
- Peu de texte : **aucune phrase d'explication visible** dans une fenêtre, un panneau, une liste ou des réglages
  (rappel insistant du 15/10/2026) ; explications en info-bulle. Onglets texte seul, fonctions prévues visibles
  mais désactivées, panneaux ancrables.
- Principe d'indépendance (D-36) : l'outil le plus complet possible sans dépendre d'un prestataire extérieur ni
  d'une connexion avant la mise en ligne ; public international (pas centré sur le français).
- **Même exigence partout** (15/10/2026) : chaque nouvelle fonction reçoit le même soin d'interface que le reste,
  vérifiée visuellement dans les deux thèmes ; prendre des initiatives de design là où le produit est faible
  (principe 8 de `docs/DESIGN.md`).
- Glissières : très fines (trait sombre 2 px, progression 4 px, poignée verticale 3 × 14 px) ; glissière des
  médias sans progression.
- **Tout site éditable depuis l'interface** (18/10/2026) : site fait dans l'interface, par un assistant (MCP) ou
  importé (Eleventy, HTML, lien, Git construit, Webflow, WordPress, Figma) : calques comme un site fait à la
  main, sans variables ni code. « Quand tu te heurtes à une limite, tu crées l'outil pour qu'un LLM et/ou un
  utilisateur puisse le faire. »
- **Donner la main à l'utilisateur, ergonomie primordiale** (19/10/2026) : ce que l'utilisateur ne peut pas
  modifier facilement, ou qui manque de logique (appariement par position, texte utilisé comme conteneur),
  est repris ; deux fonctions distinctes restent deux outils distincts.

## GitHub
- Claude est relié au compte GitHub **matv-studio**. Le compte Alunites n'est plus utilisé par Claude.
- Auteur des commits : `matv-studio <285979032+matv-studio@users.noreply.github.com>`, jamais l'adresse de l'agence.
- La session Claude ne peut ni pousser/supprimer d'étiquette ou de branche, ni changer les réglages d'un dépôt,
  ni écrire dans les versions : étiquettes par le workflow `tag.yml`, le reste par `maintenance.yml` (les deux dépôts).
  L'API GraphQL de GitHub est refusée depuis la session : `gh api` (REST) pour lire les versions.
- Application OAuth « Revel » (matv-studio) : Client ID `Ov23liuo9dnJsp95C633`, device flow activé (vérifié).
- Adresse du dépôt : https://github.com/matv-studio/Revel.git (copie du PC réglée sur cette adresse).

## Dépôt `matv-studio/Revel` (public depuis le 14/10/2026, sans licence : tous droits réservés)
- **Étape 2 terminée** : étiquettes `step-2` (075b76f, 0.2.1) puis `step-2.12` (27b4ec2, **0.2.2**), puis 2.13 et
  2.14 (corrections du 15/10/2026).
- **Étape 3 terminée** (plan : `docs/ETAPE-3.md`, version **0.3.0**) : 3.1 contenus du CMS, 3.2 texte riche,
  3.3 champs des collections (D-27), 3.4 pages et référencement (D-32), 3.5 médias, 3.6 composants à propriétés
  (D-34), 3.7 blocs et éléments, 3.8 langues (D-35), 3.8b traduction automatique hors ligne (D-36), 3.9 réglages
  du site (D-37), 3.10 versions et synchronisation GitHub (D-38), 3.11 canevas (D-39 : poignées de taille,
  sélection multiple, palette Ctrl+K, écrans côte à côte affichables ou masquables, cadre étirable sans changer la
  mise en page), 3.12 réglages lisibles (D-40 : titre de l'onglet, menus, contenus mis en avant, image de partage
  et adresse officielle). Commit final 0ff4b16 (correctif : document de l'éditeur relu dès qu'une autre vue le
  modifie). Étiquette `step-3` **non créée** : le workflow `tag.yml` refuse un commit dont `.github/workflows/`
  diffère de main (ci.yml modifié ensuite) ; à créer depuis GitHub, ou `v0.3.0` à la publication du brouillon.
  Leçon : étiqueter juste après l'envoi, avant toute modification des workflows.
- **Étape 4 terminée** (plan : `docs/ETAPE-4.md`, commit 019f027) : 4.1 serveur MCP (D-41 : 18 outils, stdio et
  HTTP local 127.0.0.1 avec clé, protocoles 2025 et 2026-07-28, relais `revel --mcp` vers l'application ouverte),
  4.2 Réglages › Assistants IA (D-42 : interrupteur, port, clé, configurations Claude Desktop, Claude Code,
  ChatGPT · Codex, LM Studio, Open WebUI, dernières actions, signalement dans la barre d'état de l'éditeur),
  4.3 OpenAPI (D-43). **Écartés par l'utilisateur** : accès à distance depuis un assistant web (D-44 : tunnel et
  prestataire refusés ; code retiré, gardé dans l'historique 61d227a) et discussion intégrée (D-45 : on discute
  dans l'application de l'assistant).
- **Étape 5** (plan : `docs/ETAPE-5.md`, mise en ligne directe ; demande du 16/10/2026) : 5.1 bouton « Publier » à
  panneau (D-47 : pages modifiées calculées sur l'ordinateur), 5.2 mise en ligne guidée (D-48 : liste déroulante
  OVHcloud, o2switch, Infomaniak, Hostinger, IONOS, GitHub Pages + bouton « Autre », message de l'hébergeur collé ;
  sources dans `docs/HEBERGEURS.md`), 5.3 connexion d'essai et clé du serveur silencieuse (D-49), 5.4 contrôles
  avant publication (D-50), 5.5 revenir à la publication précédente (D-51), 5.6 GitHub Pages en un clic (D-52 :
  dépôt créé), 5.7 nom de domaine (D-53 : enregistrements, vérification par Revel), 5.8 formulaires par PHP
  (D-54 : `revel-form.php`, bloc « Formulaire », Paramètres › Formulaires). Commits 176c03e, 6aec4a2 (essai
  de l'interface rendu robuste), 95d8bb9 (plan) ; **étape 5 terminée**, CI réussie sur 6aec4a2, étiquette
  `step-5` créée ; copie du PC à jour. Essais de l'interface : 224/224. Essais SFTP réels contre un serveur
  OpenSSH local (utilisateur d'essai `revelsftp`, port 2222, variables `REVEL_SFTP_*`).
- **Installateurs 0.4.0** : brouillon « Revel v0.4.0 » (préversion, id 414705005, sur 5357187 = fin de l'étape 4),
  9 fichiers prêts le 16/10/2026 ; l'utilisateur a été prévenu (à publier par lui).
- **Installateur** (D-46, puis D-79 du 17/10/2026) : **aucune langue de traduction livrée dès la 0.5.1** (décision
  de l'utilisateur) ; toutes se téléchargent depuis Réglages › Traduction automatique. La 0.5.0 livrait encore le
  français.
- Installateurs 0.3.0 : brouillon « Revel v0.3.0 » en préversion créé le 16/10/2026 (version 414531727, sur
  edf719b : étape 3 complète + 4.1 et 4.2), 9 fichiers (Windows .exe et .msi, macOS Intel et Apple Silicon,
  Linux .deb, .rpm, AppImage), à valider et publier par l'utilisateur (l'étiquette v0.3.0 sera créée à la
  publication). Premier lancement (0ff4b16) en échec : commit dont les workflows différaient de main.
- Hors étape : glissières fines, réglage « Étendue de l'accent » (Minimale, Standard, Étendue), code couleur sur les
  choix à boutons et les glissières (commit 73fd8be).
- Derniers commits de l'étape 4 : 0ff4b16 (fin de l'étape 3), 01633a2 (4.1), edf719b (4.2, CI réussie), 5e9896f (4.3),
  61d227a (4.4, retiré ensuite), f5a8b29 et 35f3ead (signalement dans l'éditeur, AppImage), 019f027 (fin de
  l'étape 4).
- Import de sites : Eleventy, HTML statique, lien, Git, Webflow, Figma, WordPress (étape 7) ; chaque import est
  reconstruit dans la forme de l'interface (0.6.2, D-90 ; plus rien dans le code depuis la 0.6.4, D-102).
- Installateurs 0.2.2 (brouillon « Revel v0.2.2 », non signés). Brouillons plus anciens : v0.1.0, v0.2.0, v0.2.1.
  **Tous les anciens brouillons sont gardés** comme historique du développement (décision du 17/10/2026 : ne pas
  les supprimer).
- Installateur Windows sur le PC : `<ancien dossier du PC>\installateurs\Revel_0.2.2_x64-setup.exe`.
- Copie locale : `<ancien dossier du PC>\revel` (suppression de fichiers autorisée pour cette session, pour git).
- Constat sur le `.pages.yml` du site Alunites : tri écrit `view.sort: date` / `order: desc` (ancienne écriture,
  ignorée par Pages CMS actuel) ; pas de `settings.content.merge`. Non modifié. Un design propre y ajoute un
  champ caché `layout` (D-30) ; une première traduction y ajoute l'entrée de la langue (« Projets (EN) », D-35).
- **Étape 6** (17/10/2026, commits 0244295, 1282a4d, 559a32f ; CI entièrement réussie sur 559a32f, y compris l'application sous Windows) : voir le tableau ci-dessous. À faire par l'utilisateur (réglages
  persistants, jamais faits par Claude) : clé des mises à jour (`cargo tauri signer generate`), secret
  `TAURI_SIGNING_PRIVATE_KEY` (+ mot de passe) et variable `TAURI_UPDATER_PUBKEY` ; signature Windows (Azure
  Artifact Signing, 9,99 USD/mois) et macOS (Apple Developer, 99 USD/an) au choix. **17/10/2026 : pas de signature
  pour l'instant** (choix de l'utilisateur ; voies gratuites étudiées dans D-09). Version **0.5.0** (commit
  c295de6), installateurs construits pour tous les systèmes le 17/10/2026 : brouillon « Revel v0.5.0 » (préversion,
  id 415692291, 9 fichiers d'environ 256 à 355 Mo, langue française livrée ; macOS signé ad hoc, Windows non signé ;
  à publier par l'utilisateur s'il le souhaite). Une mise à jour n'est
  proposée qu'une fois la version publiée (ni brouillon ni préversion). Essais de l'interface : 243/243
  (onglets, accordéon, Lottie, actions, transition, aide ajoutés) ; copie du PC à jour.
- **Retours sur la 0.5.0** (17/10/2026, D-78) : canevas mis à jour sur place (sans rejouer les animations),
  ajout au double-clic et conteneur vide montré (icône, 50 px), composant non déplié dans les calques (double-clic :
  son canevas), disposition à la Webflow (Bloc, Flex, Grille, Masqué + flèche ; matrice d'alignement ; grille par
  nombre de pistes ; enfant de grille ou flex), listes déroulantes à la charte (`appearance: base-select`), barres
  de défilement fines, suppression d'un site vers la corbeille, menu du site gardé dans les Réglages, langues
  installées seules (« Plus de langues »). Pile WebAssembly de l'interface portée à 8 Mio (`apps/revel-ui/build.rs`) :
  la construction de développement plantait la page en sélectionnant une grille dans un flex.
  Commit 517828a, CI entièrement réussie ; essais de l'interface 253/253 ; copie du PC à jour.
  Fichiers de la recréation d'Alunites remis : `<ancien dossier du PC>\recreation-alunites` (+ .zip).
- **Étape 7 terminée** (17/10/2026, `docs/ETAPE-7.md`, D-80 à D-85, version **0.6.0**, commits 43f67f7,
  07941bb, 21dd2c8 ; essais de l'interface 263/263, animations 17/17) : boîte « Importer » à
  tuiles (Dossier, Lien, Dépôt Git, Figma, WordPress) avec bilan de fin (chiffres repris, points à vérifier,
  « Ouvrir dans l'éditeur »). 7.1 HTML statique (dossier), 7.2 dépôt Git (option 1 : pas de construction), 7.3
  Webflow (export décompressé ; formulaires envoyés par Revel ; CMS par CSV ou API, jeton jamais gardé ; page
  modèle → gabarit), 7.4 import depuis un lien (`revel_import::crawl` : robots.txt et Crawl-delay respectés,
  fichiers des serveurs extérieurs copiés dans `/_externe/`, styles des pages réunis dans `/assets/styles.css`,
  seule voie pour Framer), 7.5 Figma (API REST, jeton personnel gardé dans le trousseau, cadres → pages, flex,
  couleurs du site, composants à propriétés texte, SVG, polices Google copiées), 7.6 WordPress (WXR, modèle
  Blog, `permalink` gardé, images de `/wp-content/uploads/` et de `*.files.wordpress.com`), 7.7 animations
  (apparitions et effets de texte Framer, interactions d'apparition Webflow, AOS → `data-revel-*`). 7.8 :
  portfr et zolt (Framer), techstudio (Webflow), alunites.com (statique), données d'essai des thèmes WordPress
  (ajoutées à la CI) ; écarts < 1,5 % sauf contenus animés ; six défauts trouvés et corrigés. Captures remises :
  `<ancien dossier du PC>\verification-import` (le sous-dossier `outil/` peut être supprimé). Non
  vérifiés sur données réelles : Figma (jeton de l'utilisateur) et CMS Webflow (export payant ou jeton).
  Commandes : `revel import-url`, `revel import-wordpress`, `revel import-figma` (jeton dans `FIGMA_TOKEN`) ;
  programme de vérification `crates/revel-import/examples/verifier.rs` (compilé en musl pour la machine liée).
- **Clés des mises à jour** : ajoutées par l'utilisateur le 17/10/2026. **Installateurs 0.6.0** (17/10/2026,
  commit 21dd2c8, flux « Installateurs » 39057663726 lancé à la main — l'envoi direct d'étiquette est refusé
  (403) ; étiquettes via le flux « Étiquette », `step-7` créée) : brouillon « Revel v0.6.0 » (préversion, id
  415995863), 17 fichiers : .exe/.msi, .dmg ×2, .app.tar.gz ×2, .deb/.rpm/.AppImage, chacun avec sa `.sig`
  (sauf .dmg), et `latest.json` (11 cibles). Tailles 14–20 Mo (AppImage 99 Mo) contre 256–355 Mo en 0.5.0.
  Signature du .deb vérifiée avec la clé publique lue dans le binaire (identifiant EA5F2855415468CD).
  6.6 et 6.9 « Terminée » (doc d'avancement rev 108, commit 6117374, D-77 complétée). Pour qu'une version
  soit proposée en mise à jour : la publier en décochant « préversion » ; 0.5.0 et avant ne se mettent pas
  à jour seules.
- **Préversions (D-86, 17/10/2026)** : Réglages › Mises à jour › « Recevoir les préversions » (désactivé par
  défaut) ; liste publique des versions du dépôt lue (API GitHub), plus haut numéro portant un `latest.json`
  retenu, repli sur l'adresse inscrite. Commits a345982, 6c00d74 (essais 264/264, CI réussie), b5b7a30 (docs).
  Livré dans le **même brouillon 0.6.0** (flux 39066418973, les 17 fichiers remplacés).
- **0.6.0 publiée en préversion** par l'utilisateur le 17/10/2026 (étiquette `v0.6.0` sur b5b7a30 = code de
  6c00d74 + documents : conforme aux installateurs). D-87 (retrait de .msi/.rpm, 5bda7d5) **annulée** à sa
  demande : installateurs de remplacement gardés, `bundle.targets` = "all" (74b8e65). Même commit : README
  réécrit pour qui découvre Revel (captures `docs/captures/editeur.png`, `importer.png`, site de démonstration
  « Atelier Lumière ») ; mises à jour : `ReleaseNotFound` (aucune version publiée hors préversion, l'adresse
  « latest » répond 404) → « Revel est à jour » au lieu de l'erreur anglaise (vaut à partir de la version
  suivante ; dans la 0.6.0, « Rechercher une mise à jour » affiche cette erreur tant qu'aucune version normale
  n'est publiée, sauf avec « Recevoir les préversions » coché).
- **Version 0.6.1** (18/10/2026, demandes du 17/10/2026) : commits ecc26dc (D-88, D-89), 6edaedd (numéro de
  version), 127f4f1 (outil MCP insert_html : pages XML et texte ; mise en forme), eb79cec, 0c1de7a (un compte
  GitHub par personne). CI entièrement réussie sur 0c1de7a (flux 39086056796). D-88 : sites à construire
  construits par Revel sur demande (crate `revel-build` : Node.js LTS et Hugo téléchargés et vérifiés, npm,
  pnpm, Yarn par corepack, 17 outils reconnus) ; « Importer le site publié » ; Réglages › Outils de
  construction ; `revel import-build`. Vérifié sur `deltea/p3r-pause-menu` : 0,0 % d'écart avec
  p3r.deltea.space. D-89 : Paramètres › Versions › Travail à plusieurs (Seul, Dossier partagé = dépôt central
  `<site>.git` sur le NAS, Dépôt en ligne), fusion par endroit du document, « Votre nom » (Réglages ›
  Comptes). Brouillon v0.6.1 (préversion, id 416153491, 17 fichiers, latest.json 0.6.1), étiquette `v0.6.1`
  sur 0c1de7a ; publiée en préversion par l'utilisateur le 18/10/2026.
- **Recréation complète d'Alunites** (18/10/2026, demande « moins de 1 % ») : outils de Revel pour
  assistants (MCP insert_html, commandes de l'éditeur) : 10 pages, 4 gabarits, 13 composants, 5 collections,
  438 éléments de style ; 66 pages × 1280/375 px, 0,00 %. Remis sur le PC :
  `<ancien dossier du PC>\recreation-alunites-complete`. Étape 6 passée « Terminée » (rev 120).
- **Version 0.6.2** (18/10/2026 ; demandes : tout site éditable depuis l'interface, imports reconstruits, outils
  manquants créés, p3r en exemple) : commits 41c9252 à 070b1ad (15 commits), D-90 à D-95.
  - Relevé (`revel audit`, outil MCP `check_site`) et reconstruction (`revel rebuild`, automatique à chaque
    import) dans la forme des panneaux, à rendu identique (D-90).
  - Formes ajoutées (D-91) : conditions par clauses (et / ou), propriété « Liste », « Mettre en premier »,
    classe et adresse selon un champ, nombre de valeurs, mise en valeur `*mots*`, filtres de liste,
    consentement aux cookies + GA4, onglets au survol, lecteur vidéo, enfant au hasard, Échap.
  - En-tête écrit par Revel d'après les réglages (D-92 : Paramètres › Référencement, réglages de page et de
    modèle, date de publication, `aria-current`) ; l'en-tête écrit à la main devient ces réglages à la
    reconstruction quand le site produit reste le même.
  - Alunites recréé dans la forme de l'interface (D-93) : script propre retiré, relevé vide, écart visuel
    ≤ 0,02 %.
  - Outils pour sites animés et sonores (D-94, exemple p3r) : ombre du texte, point d'appui, découpe
    (`clip-path`, points du polygone), états « Actif » / « Dans un élément actif » / « Éteint », actions « Jouer
    un son » et « Lancer une musique », interrupteurs Musique/Sons (On/Off), onglets au clavier et sonores,
    animation « Devenu actif », icône par son nom. p3r.deltea.space recréé avec les seuls outils MCP : relevé
    vide, 0,01 % (accueil), 0,2 à 0,34 % (menu).
  - Redirections (D-95) : la liste de Paramètres › Navigation › Redirections n'était qu'enregistrée ; écrite
    désormais (règles `mod_rewrite` dans `.htaccess`, paramètres `?p=12` compris, et page `meta refresh` à
    l'ancienne adresse pour GitHub Pages).
  - Vérifications : import HTML d'Alunites relevé 0, 72/72 pages identiques ; WordPress et lien relevé 0 ;
    Eleventy (site cobaye) 359 → 186 points, styles et captures identiques ; essais de l'interface 273/273 ;
    CI entièrement réussie sur 070b1ad (flux 39167444891 ; la comparaison des captures attend désormais la
    première image des vidéos et redemande une image non chargée).
  - Installateurs : flux 39169466396 → brouillon « Revel v0.6.2 » (préversion, id 416737756, cible 070b1ad,
    17 fichiers, `latest.json` 0.6.2 à 11 cibles, signatures .deb et .exe à la clé EA5F2855415468CD).
    Étiquette `v0.6.2` créée sur 070b1ad (flux « Étiquette » 39171637506). Contient le défaut corrigé en
    0.6.3 : ne pas le publier.
  - Copie du PC (`<ancien dossier du PC>\revel`) restée sur 0c1de7a.
- **Version 0.6.3** (19/10/2026, commits 7782627 et 476876e ; D-96 à D-98) :
  - Propriétés du gabarit (D-96) : valeur choisie par chaque page (Paramètres de la page › « Propriétés du
    gabarit » : ajout, nom, retrait), éléments du gabarit reliés depuis Styles, utilisation de composant qui
    reprend une propriété (inversée au besoin). Variables de page lues par le gabarit converties à la
    reconstruction (ex. `hideFooterCta` → interrupteur « Hide footer cta »).
  - Reconstruction étendue (D-97), chaque réécriture gardée seulement si les fichiers produits restent
    identiques : réglages de page tirés des données, plan du site écrit à la main → plan de Revel avec
    « Dates de mise à jour » (nouveau réglage), listes « mis en avant, sinon une liste », contenus choisis dans
    plusieurs collections (liste à plusieurs collections, « + Autre collection » dans le panneau), premier
    contenu d'une liste, contenu relié lu directement, valeur de tableau dans une variable. Utilisation de
    composant : propriété reliée à un champ avec champ de repli et écriture de date, réglable.
  - **Défaut de la 0.6.2 corrigé** (D-98) : la reconstruction (donc chaque import) écrivait les valeurs de la
    production à la place des variables d'environnement : une recette publiée perdait `noindex` et
    `X-Robots-Tag`. Corrigé, vérifié dans chaque environnement, contrôle ajouté à la CI
    (`tools/compare-robots.py`). Conditions sur l'environnement possibles dans l'interface.
  - Site cobaye (Eleventy) : 331 → 63 points dans le code ; styles et captures identiques (13 pages × 1280/375),
    recette identique ; essais de l'interface 280/280.
  - CI entièrement réussie sur e053676 (flux 39184324814). Installateurs : flux 39185822073 → « Revel v0.6.3 »
    (id 416825746, cible e053676, 17 fichiers, clé EA5F2855415468CD), **publiée en préversion par
    l'utilisateur le 19/10/2026**. Étiquette `v0.6.3` sur e053676 (flux « Étiquette » 39187990780).
- **Version 0.6.4** (19/10/2026, commits df8b441 et f3b8528 ; D-99 à D-102) :
  - Média au survol (D-99, demande « deux outils distincts ») : outil à part de la liste de collection qui le
    pilote (élément média `data-revel-hover-media`, éléments survolés réglés dans le panneau, « Affiché dans »,
    bouton pour aller de l'un à l'autre) ; héros d'Alunites refait : une vidéo + une liste de contenus choisis.
  - Informations du site (D-100) : Paramètres › Informations du site (valeurs, menus `site.menus.<nom>`)
    reliées aux textes, liens (`mailto:`, `tel:`, plan), images, listes, conditions ; gardées à la reconstruction.
  - Ergonomie (D-101) : un seul choix « Rôle » pour un élément sans rôle (groupé), paires onglet ↔ panneau,
    attributs gérés retirés des « Autres attributs », conteneurs en ligne montrés comme conteneurs (plus de « T »).
  - **7.10 terminée** (D-102) : fichiers du serveur modifiés comme texte avec leurs balises (listés dans Pages,
    relus, texte illisible refusé) ; valeurs reliées des attributs et du style (Inspecteur › Attributs) ;
    écritures du site, valeurs d'un champ liste séparées, texte si vide ; conditions premier/dernier de la liste,
    contenu qui a une page ; `pick_first(…, page.url)`. Reconstruction : choix écrits d'avance, variables de
    composant en propriétés, image par cas, liste de données lue par adresse → collection du CMS (contenus +
    `.pages.yml`), boucles reliées réunies, composants dépliés, listes construites parcourues directement, HTML
    assemblé en éléments. Site cobaye Eleventy : 273 → 0 point ; 13 pages × 1280/375 : 0 écart de styles et de
    captures ; recette identique.
  - Canevas : une page qui ne s'affiche plus après une modification garde la précédente et donne la raison.
  - Essais de l'interface 292/292 (rôle, média au survol, fichier du serveur, attribut relié ajoutés). CI
    entièrement réussie sur f3b8528 (flux 39272772579). Installateurs : flux 39277020774 → brouillon « Revel
    v0.6.4 » (préversion, id 417464049, cible f3b8528, 17 fichiers, `latest.json` 0.6.4 à 11 cibles, signatures
    .deb et .exe à la clé EA5F2855415468CD), **publiée en préversion par l'utilisateur le 19/10/2026**.
    Étiquette `v0.6.4` sur f3b8528 (flux « Étiquette » 39277024864).
  - Recréation d'Alunites dans la forme de l'interface (héros, informations du site, menus, collection
    « chiffres », listes « autres projets » où un contenu suggéré reste montré) : 66 pages × 1280/375, aucune
    capture différente ; remise : `<ancien dossier du PC>\recreation-alunites-interface` (exige la 0.6.4 ;
    `manifest.md5` pour vérifier ; le .zip peut être supprimé ; une image SVG reçoit des métadonnées de
    provenance à l'écriture, sans effet sur l'affichage).
  - p3r revu : les `span` qui contiennent des éléments sont dans des boutons (seul contenu valide) ; montrés
    comme conteneurs dans les calques.
- **Après la 0.6.4** (21/10/2026, commit 0b09c57, D-103 à D-105 ; demandes du 19/10/2026) :
  - Plan du site nodal (D-103, 8.2) : onglet « Plan du site » avant les Paramètres ; pages, collections, menus
    et parties communes reliés par leurs liens (`revel_edit::linkmap`) sur un plan infini (glisser, molette,
    Ctrl+molette, F, minicarte) ; nœuds déplacés gardés dans le site (`library.map`, commande
    `set_map_positions`), « Ranger » ; destination d'un lien changée en tirant le bout d'un trait ou par
    « Mener à » ; lien ajouté en tirant le point d'une page (fin de la page) ou d'un menu ; panneau (nom,
    adresse, brouillon, ouvrir, nouvelle page dedans, dupliquer, supprimer, créer la page manquante,
    rediriger, liens entrants et sortants, « Montrer dans l'éditeur ») ; pages sans lien et adresses sans
    page signalées ; Ctrl+Z ; outil MCP `get_site_map`.
  - Paramètres réorganisés (D-104, 8.3) : site et Revel avec un rail ; la rubrique ouverte se raccorde à la
    page comme l'onglet actif (contour, coins incurvés) et liste ses parties (suivies au défilement) ;
    parties en deux colonnes (titre et présentation, réglages) ; partie vide retirée ; flux RSS seulement
    pour les collections qui ont des pages ; informations du site en colonnes alignées.
  - Corrections (D-105) : code de vue affiché à côté des informations du site et des menus (valeur
    d'attribut contenant `>`), calque masqué caché aussitôt dans le canevas, « Vidéo de chaque contenu »
    limitée aux champs qui donnent une vidéo, texte commençant par une lettre accentuée après une balise
    (plantage du rendu d'un fichier du serveur).
  - Essais de l'interface 304/304 ; CI entièrement réussie sur 0b09c57 (flux 39534881260).
  - Leçon : la copie du cobaye pour les essais se fait sans `.git`, comme la CI (le dépôt local du cobaye est
    superficiel et fait échouer l'essai « travail à plusieurs »).
  - Puis (21/10/2026, commit 1fbc3a9, D-106) : profil en haut à droite (photo seule, à la demande de
    l'utilisateur ; fenêtre : photo recadrée et gardée dans les réglages `avatar`, nom, compte GitHub ; sans
    choix : photo et nom du compte GitHub), mêmes réglages dans Réglages › Profil et comptes ; « Revenir à
    <site> » en bouton de la barre (plus dans le menu du site) ; polices dans Paramètres › Général ; tri et
    filtre des médias en listes à la charte (une règle `appearance: none` laissait la liste du système,
    illisible sur thème sombre). Essais 306/306 ; CI entièrement réussie sur 1fbc3a9 (flux 39542544559, après
    relance de « Équivalence » : la vidéo du héros n'était pas encore « prête » à une capture de l'import HTML,
    écart sans rapport avec le changement).
- **Version 0.6.5** (21/10/2026, commit d239941 « Version 0.6.5 » ; contenu : commits 0b09c57 et 1fbc3a9,
  D-103 à D-106) : CI entièrement réussie sur d239941 (flux 39547866464). Installateurs : flux 39550520392 →
  brouillon « Revel v0.6.5 » (préversion, id 419203029, cible d239941, 17 fichiers, `latest.json` 0.6.5 à
  11 cibles, signatures à la clé EA5F2855415468CD), **publiée en préversion par l'utilisateur le 21/10/2026**.
  Étiquette `v0.6.5` sur d239941 (flux « Étiquette » 39550522679).
- **Après la 0.6.5** (21/10/2026, commit fb43fd2, D-107 et D-108, 8.4 ; demandes du 21/10/2026) :
  - Plan du site, groupe « Médias et réseaux » (D-107) : vidéos, sons et réseaux sociaux par plateforme ;
    adresses relevées aussi dans les médias intégrés, les champs et le texte des contenus du CMS et les
    informations du site ; vérifiées par Revel (oEmbed pour YouTube et Vimeo, sinon premier octet ; résultats
    dans `.revel/links.json`, refaits au-delà d'un jour) ; « N adresses à revoir » ; panneau : état, endroits,
    ouvrir, remplacer partout (commande `ReplaceInEntry` dans les contenus : mise en forme gardée) ; outils
    MCP `check_links` et `replace_link`. Corrections : fond de la rubrique active sans parties, « Ranger »
    centré, zoom Alt+molette.
  - Synchronisation automatique du travail à plusieurs (D-108) : vérification toutes les minutes (rien de
    téléchargé), réception à la pause sans créer de version (changements en cours gardés), versions
    enregistrées envoyées aussitôt, Ctrl+Z gardé (étape vérifiée sur une copie, abandonnée si un collègue a
    changé le même endroit), conflit : pause, « Vérifier » / « Plus tard » ; indicateur en haut à droite ;
    interrupteur dans Paramètres › Versions (`auto_sync`, actif par défaut).
  - Essais de l'interface 315/315 ; CI entièrement réussie sur fb43fd2 (flux 39575657729, après relance de
    « Équivalence » : donnée traduite d'une langue pas encore écrite dans le délai de l'essai, sans rapport
    avec le changement ; passée à la relance).
  - Leçons : `pgrep -f`/`pkill -f` dans une commande dont le texte contient le motif se trouve lui-même (tue
    le shell ou boucle sans fin) : passer par un script ; un champ actif qui est une case à cocher n'est pas
    une saisie (sinon la synchronisation automatique attend indéfiniment).
- **Version 0.6.6** (21/10/2026, commit 7712ad1 « Version 0.6.6 » ; contenu : commit fb43fd2, D-107 et D-108) :
  CI entièrement réussie sur 7712ad1 (flux 39581879145). Installateurs : flux 39585580323 → brouillon
  « Revel v0.6.6 » (préversion, id 419444691, cible 7712ad1, 17 fichiers, `latest.json` 0.6.6 à 11 cibles,
  signatures à la clé EA5F2855415468CD), **à publier par l'utilisateur**. Étiquette `v0.6.6` sur 7712ad1 (flux
  « Étiquette » 39585583765).
- Document d'avancement pour non-techniciens : Claude Docs « Revel — avancement du projet »
  (https://claude.ai/code/artifact/a810245c-62be-432d-9d9f-23fe88b5a69f), une ligne par sous-étape (étapes 0 à 7
  terminées, 7.10 comprise depuis la 0.6.4 ; 8.1 « À venir », optionnelle ; 8.2, 8.3 et 8.4 terminées).

- **Site matv (24/10/2026, demande : import Framer réparé puis site recréé)** : https://matv.framer.website/,
  fichiers fournis : `ScrollProgress.tsx`, `Projects.csv` (extension « CMS Export » de Framer), captures.
  - Import Framer réparé (D-109) : modules du site relus (`crates/revel-import/src/framer.rs`, `framer_site.rs`) :
    apparitions, survol, barre de progression, Lenis ; CMS `.framercms` décodé (format binaire gros-boutiste,
    adresse `/modules/`→`/cms/`), collection + gabarit + listes complètes + maçonnerie ; langues (pages `/fr/`
    alignées, paragraphe découpé aux tirets, contenus `content/fr/projects`) ; vidéos locales et `muted` ;
    texte riche du CMS (`rich-text-wrapper` + règles `p.framer-text` étendues).
  - Outils créés (D-110) : défilement doux (Paramètres › Général), barre de progression (Ajouter ; « Suit »
    colonne/page), maçonnerie (bouton de la grille), lecture des vidéos dans l'aperçu (réponses 206).
  - D-111 : navigation qui suit la section (`data-revel-spy`, interrupteur de l'inspecteur), cible
    d'interaction `closest:.classe`, variantes au clic de Framer relues dans le code du composant
    (`framer_jsx.rs`, `framer_click.rs`).
  - D-112 : composants Framer utilisés plusieurs fois → composants Revel avec propriétés
    (`framer_components.rs` ; matv : Stats, Experience, Primary) ; classes `framer-xxxx` renommées d'après
    les calques (`framer_names.rs`, 465 classes) ; maçonnerie « colonnes à tour de rôle » (`order`, ordre de
    Framer) ; défilement doux qui atteint le bas des colonnes ; apparition jouée en fin de colonne ;
    interrupteurs de l'interface cliquables sur la poignée.
  - Mesures (Chromium) : accueil 0,36 % à 1280 px, 0,07 à 1,9 % à 375 px ; cartes « Outils » ouvertes 1,3 à
    2,7 % ; textes visibles identiques (EN) ; FR : seul écart « Lancer un projet » des pages projet (une
    traduction par texte).
  - Outils de mesure : `$SP/tools/` (`reimport.sh`, `serve.cjs` serveur local avec Range, `textdiff.cjs`,
    `scrollshots.cjs`, `dd.cjs` cartes ouvertes, `proj.cjs`).
  - Différence voulue : le site en ligne charge les projets 7 à 10 en arrivant en bas de la colonne
    (pagination de Framer) ; l'import les montre tous d'emblée. Captures en défilement : passer par la
    molette (`wheelshots.cjs`), Lenis remet `scrollTop` à sa valeur.
  - Reprise après limite d'utilisation : demande du 25/10/2026 — programmer une vérification
    (`send_later`, 3 h) qui reprend le travail interrompu et se reprogramme tant qu'il en reste.
  - Pièges : `http.server` de Python ne sert pas les morceaux (vidéos MP4 non lues) ; sshd et le dossier
    `/home/revelsftp/www` disparaissent à un redémarrage du conteneur (relancer `sshd -f $SP/sshd/sshd_config`).
  - **Version 0.6.7** (25/10/2026) : commits e513766, a3e6881, e1fe7c9 « Version 0.6.7 », fab502d
    (maçonnerie suivie dans le canevas, sans écart final), e545702 (outil de comparaison : événement
    `playing` envoyé aux vidéos des deux sites, l'écart « video.hero__video opacity » de « Équivalence »
    venait de là, reproduit 3/3 en local). CI entièrement réussie sur e545702 (flux 40153431527).
    Installateurs : flux 40156222153 → brouillon « Revel v0.6.7 » (préversion, id 423121324, cible
    e545702, 17 fichiers, `latest.json` 0.6.7 à 11 cibles), **à publier par l'utilisateur** ; étiquette
    `v0.6.7` (flux 40156223294). Installateur Windows copié dans `_Claude\Installateurs`.
    `gh workflow run` passe par GraphQL (refusé) : lancer les flux par
    `gh api -X POST repos/matv-studio/Revel/actions/workflows/<fichier>/dispatches -f ref=main -f 'inputs[x]=y'`.
  - Commits e513766 puis a3e6881 sur `main` (D-109 à D-112) ; essais de l'interface 320/320, animations
    17/17 (élément en bas de page ajouté), tests Rust réussis. CI de e513766 : clippy (type complexe,
    corrigé dans a3e6881) et « Équivalence » (opacité de la vidéo du héros à 600 px, écart déjà vu
    passer à la relance).
  - Projet remis : `<dossier Revel du PC>\Projects\matv` (import de matv.framer.website par le code de
    a3e6881, 225 Mo dont 184 Mo de vidéos) ; morceaux de transfert dans
    `_Claude\Livrables\matv-transfert` (supprimables). Transfert vers le PC : `device_commit_files`
    refuse les fichiers de 28 Mo (HTTP 400) ; morceaux de 9 Mo acceptés (`split -b 9M`), puis
    `cat … | tar -x` et `sha256sum -c` dans le shell du PC.

- **Site matv recréé et outils de structure (25/10/2026, 8.6, D-113 à D-117)** — demande : recréer tout matv
  bien organisé (composants pour services, outils, rangs…, classes nommées), gérer les classes, corriger la
  surbrillance du rail des paramètres, créer les outils manquants. Ticker de l'import non converti en
  Défilement et « afficher les 10 projets mais garder le chargement après 6 » : traités dans la recréation,
  l'import Framer n'est plus à retravailler pour l'instant.
  - Rail des paramètres (`site.rs`) : aller à une partie verrouille la surbrillance (`GOTO`) jusqu'à 150 ms
    sans défilement ; plus de va-et-vient.
  - D-113 panneau **Classes** (à droite avec Styles et Interactions ; `apps/revel-ui/src/classes.rs`,
    `crates/revel-edit/src/classes.rs`, `Command::RenameClass`, outils MCP `list_classes`, `rename_class`,
    `remove_class`) ; classes des images (`"classe"` écrite) comprises. Piège : la classe `.classes` existait
    déjà (pastilles de l'inspecteur) → racine du panneau `.class-manager`.
  - D-114 `data-revel-more` (`crates/revel-render/src/more.rs`, champ « Afficher d'abord » de la liste).
  - D-115 Paramètres › Général › Couleurs (`sitecolors.rs`) + bouton lune « Aperçu en mode sombre » (message
    `scheme` du canevas : requêtes `prefers-color-scheme` réécrites dans le cadre).
  - D-116 états `[open]` / `^[open]` (message `state-preview` : le canevas ouvre l'accordéon) ; GIF animés
    servis tels quels (`revel-assets`, `gif_frames`).
  - D-117 site : générateur `$SP/matv/rebuild/gen.py` (copie sur le PC : `_Claude\Livrables\matv-gen.py`) →
    `$SP/matv/rebuild/matv` ; données des outils relevées par `$SP/tools/tools-data.cjs`, traductions
    `$SP/matv/en-fr.json`. Remis dans **`<dossier Revel du PC>\Projects\matv-revel`** (154 Mo ; l'import reste
    dans `Projects\matv`) ; morceaux dans `_Claude\Livrables\matv-revel-transfert` (supprimables).
    Comparaison avec l'import (`$SP/tools/cmp.cjs`, colonnes dépliées) à 1280, 1000 et 375 px, FR et mode
    sombre : rendu identique à quelques pixels près (espacement des cartes Outils régularisé). `revel audit`
    et `check_site` : tout est éditable dans l'interface.
  - Visite de l'éditeur sur le site : `$SP/tools/tour-serve.sh` (pont 8164, MCP 8165) puis `tour.mjs`,
    `tour2.mjs` ; appel MCP en ligne de commande : `$SP/tools/mcp.sh <outil> '<json>'`.
  - Piège : `revel build` écrit son cache d'images dans `<site>/.revel` (79 Mo) : l'exclure des archives.
  - Piège : le shell du PC (`device_bash`) peut répondre « Workspace unavailable » : réessayer.
  - Commits 68c10b5, 5d3e078 (pastilles bordées), 776308c « Version 0.6.8 », 039e650 (essai natif de la
    disposition : `cargo test --manifest-path apps/revel-ui/Cargo.toml`, ajouté à `verify.sh`) ; essais de
    l'interface 326/326, animations 17/17 ; CI entièrement réussie sur 039e650 (flux 40212658729).
    Installateurs 0.6.8 : flux 40215053080 → brouillon « Revel v0.6.8 » (préversion, id 423549259, cible
    039e650, 17 fichiers), **à publier par l'utilisateur** ; installateur Windows copié dans
    `_Claude\Installateurs` (morceaux dans `_morceaux`, supprimables) ; étiquette `v0.6.8`
    sur 039e650 (flux 40215054689). Captures `_Claude\Captures\0.6.8-matv-*` (deux thèmes).

- **Composants à emplacement, CMS réservé aux pages (25/10/2026, 8.7, D-118, D-119)** — règle de
  l'utilisateur : « les composants sont à part du CMS ; le CMS ne sert qu'à construire des pages entières ;
  le contenu se range dans le composant et se remplace par utilisation dans Styles ; les cartes de projets
  lisent la page du projet ; pas de sous-CMS pour contourner ; ne pas répartir sur plusieurs outils ce qu'un
  seul peut faire ».
  - D-118 emplacement : `Node::Use.children` (contenu propre), élément `data-revel-slot` du composant
    (`revel_model::SLOT_ATTR`), rendu `slots_in` dans `eval.rs`, `data-rv-slot-filled` dans le canevas
    (`inOwnSlot` de `canvas.js`), `make::{slot_of, customize_slot, reset_slot, set_slot}`, Styles
    « Contenu » (Modifier ici / Revenir au contenu du composant), interrupteur « Emplacement du contenu »
    dans le composant, calques sous l'utilisation, ajout et dépôt dans l'utilisation, détacher. Cadre d'une
    utilisation calculé sur ses éléments (`instBox` : un `details` fermé gardait la hauteur de son contenu).
  - D-119 : nouvelle collection = collection + modèle + adresse `/<slug>/{{ slug }}/` en une commande
    (`expand` d'une suite simule la création de collection sur sa copie) ; instructions MCP réécrites.
  - Site matv refait : seule la collection `projects` ; Services (points dans l'emplacement), Toolset
    (outils = utilisations de Tool dans l'emplacement), Experience, Rank, Stat placés sur la page avec leurs
    valeurs ; logos dans le Défilement ; sections nommées dans les calques. Rendu comparé : 1280 px hauteur
    3336 contre 3337, FR et sombre identiques à quelques pixels.
  - Pièges : le `revel` de `target/release` doit être reconstruit après un changement de `canvas.js` ou du
    rendu (sinon l'ancien rendu) ; le disque (quota ~44 Go) se remplit vite : supprimer
    `target/debug` et `apps/revel-ui/target/wasm32-unknown-unknown/debug/incremental` (7 Go), lancer avec
    `CARGO_INCREMENTAL=0`.
  - Remis sur le PC : **`<dossier Revel du PC>\Projects\matv-revel`** (version à emplacements) ; ancienne
    version déplacée dans `Projects\_to_delete\matv-revel-v1` (à supprimer par l'utilisateur) ; dossiers de
    transfert `_Claude\Livrables\matv-revel-transfert`, `matv-revel-v2-transfert`, `matv-transfert`
    supprimables. Générateur à jour : `_Claude\Livrables\matv-gen.py`.
  - Essais : e2e 328/328, tests Rust, clippy, fmt. CI de 52fe932 en échec sur « canevas : nouvel onglet »
    (l'essai cliquait le premier interrupteur, désormais celui de l'emplacement) → interrupteur
    `#insp-new-tab` et essai ciblé (d0e8bd3). (0.6.9 ensuite : voir plus bas.)

- **Revue d'ergonomie (25-26/10/2026, 8.8 et 8.9, D-120 à D-125)** — demandes de l'utilisateur : revue complète
  de l'interface, puis réglages de l'éditeur.
  - D-120 classes proposées (`classes::suggest`) et menu d'état selon l'élément (`--rv-states` du canevas) ;
    D-121 flèches entre variantes, variante au clic (`click class:is-<v> self` sur la racine) ; D-122 dossier
    déposé sur l'accueil (WebView2 `postMessageWithAdditionalObjects` → `drops.rs`, ailleurs recherche par nom) ;
    D-123 éléments des outils dans Styles (`items.rs` : défilement, carrousel, onglets `tabs::add_tab`, liens,
    accordéon `name` exclusif) ; D-124 interrupteurs, barre du composant, paramètres sans phrases (ⓘ), noms des
    éléments ajoutés (`names::PRESET_NAMES`), pastilles du gestionnaire de classes.
  - D-125 : interrupteur allumé à poignée verticale + accent ; vignettes continues ; `clamp_non_negative`
    (revel-style) dans `Sx::set_many`/`preview` ; Réglages de Revel › Éditeur (liens `variant_links`
    curve/square/straight, bout `variant_link_end`, familles `palette_hidden`), Couleurs (`ui_colors` : comp,
    set, inherit, auto ; composants violets, traduction cyan), Raccourcis (`shortcuts`, `keys.rs`,
    `Ed::run_action`) ; liens de variantes à la Blender (prises, contour, flèche sur la tangente) ; « Image
    révélée » (`Preset::Reveal`, `data-revel-reveal` fade/wipe/spot, `actions.js`) ; 95 icônes en onglets +
    import SVG (`icons::clean_svg`, `icons::set_icon`) ; `Ed::apply_keep` (ajout sans changer la sélection).
  - CI verte sur ca126e7 (revel-app : `unsafe_code = "deny"` au lieu de `forbid`, autorisé sur `drops::watch` seulement ; essai Figma de l'équivalence instable une fois, relancé).
  - Version 0.6.9 (ed100c0) : installateurs flux 40282598293 → brouillon « Revel v0.6.9 » (préversion, id
    423951337, cible ed100c0, 17 fichiers), **à publier par l'utilisateur** ; étiquette `v0.6.9` (flux
    40282600411) ; CI verte (40282584449). Installateur Windows copié dans `_Claude\Installateurs`
    (`Revel_0.6.9_x64-setup.exe`, SHA-256 d385b6c2…, morceaux dans `_morceaux`, supprimables).
  - Pièges : `pgrep -f` dans une commande Bash qui contient le motif tue le shell (code 144) : lancer depuis
    un script ; WebView2 non vérifiable ici (compile pour Windows dans un crate de contrôle sous /tmp).

| Étape | État |
|---|---|
| 0 | squelette modulaire, CI |
| 1 | projet sur disque, import Eleventy, génération, images AVIF/WebP, aperçu, publication SFTP et GitHub Pages, interface FR/EN |
| 2 | éditeur visuel — **terminée** (0.2.2, puis 2.13 et 2.14) |
| 3 | éditeur complet (CMS, pages, médias, composants, langues, traduction hors ligne, versions, canevas, réglages lisibles) — **terminée** (0.3.0) |
| 4 | assistants IA installés sur l'ordinateur (D-31, D-41 à D-45) — **terminée** ; ni accès à distance ni discussion intégrée |
| 5 | mise en ligne directe (bouton Publier, hébergeurs guidés, contrôles, retour arrière, GitHub Pages, domaine, formulaires) — **terminée** (0.4.0) |
| 6 | animations et interactions, Lottie, mises à jour, signature, documentation, installateur 1.0 — **terminée** (18/10/2026) ; 0.6.2 : outils des sites animés et sonores (D-94) |
| 7 | import depuis d'autres outils — **terminée** (0.6.0) ; 7.9 sites à construire (0.6.1, D-88) ; 7.10 imports reconstruits dans la forme de l'interface (0.6.2 à 0.6.4, D-90 à D-92, D-96 à D-98, D-102) — **terminée** |
| 8 | fonctions optionnelles : 8.1 signature du code (ancienne 6.7, D-09), à venir ; 8.2 plan du site nodal, 8.3 paramètres réorganisés (D-103 à D-106), 8.4 liens externes vérifiés et synchronisation automatique (D-107, D-108), 8.5 import Framer complet et outils de défilement (D-109 à D-112), 8.6 outils de structure et site matv recréé (D-113 à D-117), 8.7 composants à emplacement et CMS réservé aux pages (D-118, D-119), 8.8 revue d'ergonomie (D-120 à D-124), 8.9 réglages de l'éditeur, liens de variantes, image révélée, icônes (D-125) — **faites** |

## `matv-studio/alunites-revel-lab` (public)
Copie du site Alunites servant de cobaye (avec son `.pages.yml`). Copie sur le PC :
`<ancien dossier du PC>\_obso\alunites-revel-lab` (rangée par l'utilisateur ; ses médias ont servi à la
remise de la recréation).

## Décisions
- Prises : D-05 Leptos, D-11 OAuth device flow, D-12 git2, D-13 commandes prêtes pour MCP, D-14 import,
  D-15 direction artistique, D-16 échecs = clés de traduction, D-17 dépôt public, D-18 styles détenus par Revel,
  D-19 textes liés modifiables sur la page et par le CMS, D-20 panneau de styles non technique,
  D-21 espace de travail modulable et code couleur de Webflow, D-22 parcours par vues et canevas comme Figma,
  D-23 un dossier par langue (Webflow), D-24 polices copiées dans le site, D-25 synchronisation GitHub à la
  publication, D-26 aperçu sans génération et export, D-27 schéma des collections dans `.pages.yml` (sans copie),
  D-28 panneau Variables (fermé par défaut), D-29 dossiers de médias et de composants (rangement de l'éditeur),
  D-30 modèle de collection et design propre à un contenu, D-31 étape 4 pour tout assistant IA, D-32 pages
  (référencement, adresses), D-33 étape 7 compatibilité Webflow/Framer/Figma, D-34 composants à propriétés,
  D-35 langues (textes par texte d'origine, données et contenus par dossier, `hreflang`), D-36 indépendance et
  traduction automatique hors ligne, D-37 réglages du site (code personnalisé, polices), D-38 versions du site et synchronisation GitHub, D-39 canevas
  (poignées, sélection multiple, palette, écrans côte à côte, cadre étirable), D-40 réglages lisibles, D-41 serveur MCP, D-42 brancher un assistant,
  D-43 OpenAPI, D-44 pas d'accès à distance, D-45 pas de discussion intégrée, D-46 installateur, D-47 à D-54
  mise en ligne (panneau Publier, hébergeurs guidés, connexion d'essai, contrôles, retour arrière, GitHub Pages,
  domaine, formulaires), D-55 calques et noms simples, D-56 pas d'étape « feuille de style » (gabarit de base,
  fond sombre ou clair), D-57 installation dans « Programmes » et dossier `Projects`, D-58 canevas (marges et
  espacements glissés, largeur du cadre, Ctrl+Espace, double-clic sur un texte de composant), D-59 animations
  prêtes à l'emploi (attributs `data-revel-*`, script du site sans bibliothèque), D-60 à D-73 passe complète
  (espacements, calques lisibles, canevas, composants à variantes, mise en forme dans le texte, réglages ajoutés
  après la recréation), D-74 Lottie, D-75 actions entre éléments, D-76 transitions entre les pages, D-77 mises à jour, D-78 retours sur la 0.5.0, D-79 aucune langue livrée, D-80 import depuis Git, D-81 import depuis un lien,
  D-82 moteurs des outils retirés et animations reprises, D-83 CMS de Webflow, D-84 WordPress, D-85 Figma,
  D-86 préversions, D-87 (annulée), D-88 sites à construire construits par Revel, D-89 travail à plusieurs,
  D-90 tout site éditable depuis l'interface (relevé, reconstruction), D-91 formes de l'interface ajoutées,
  D-92 en-tête écrit d'après les réglages, D-93 Alunites dans la forme de l'interface, D-94 outils des sites
  animés et sonores, D-95 redirections écrites dans le site produit, D-96 propriétés du gabarit,
  D-97 reconstruction étendue, D-98 environnements gardés à la reconstruction, D-99 média au survol,
  D-100 informations du site, D-101 ergonomie (rôles, onglets, conteneurs), D-102 7.10 terminée,
  D-103 plan du site nodal, D-104 paramètres réorganisés, D-105 calque masqué dans le canevas et champs de média,
  D-106 profil, retour au site, polices dans Général, listes des médias.
- Ouvertes : D-07 identifiant d'application, D-08 licence, D-09 signature de code (options rappelées le
  19/10/2026 : Azure Artifact Signing 9,99 USD/mois pour les organisations de l'UE, certificat OV 150 à 300 USD/an,
  Microsoft Store en MSIX, Apple Developer 99 USD/an ; SmartScreen avertit tant que la réputation n'est pas faite).

Détails : `revel/docs/PLAN.md`, `revel/docs/DECISIONS.md`, `revel/docs/ETAPE-2.md`, `revel/docs/ETAPE-3.md`,
`revel/docs/ETAPE-4.md`, `revel/docs/ETAPE-5.md`, `revel/docs/ETAPE-6.md`, `revel/docs/ETAPE-7.md`, `revel/docs/ETAPE-8.md`, `revel/docs/HEBERGEURS.md`, `revel/docs/DESIGN.md`.
