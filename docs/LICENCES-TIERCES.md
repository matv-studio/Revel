# Licences des composants tiers

Revel est sous licence MIT ([LICENSE](../LICENSE)). Il embarque ou utilise les composants ci-dessous, chacun
sous sa propre licence. Relevé du 23/11/2026 (version 0.8.1), à refaire avant chaque version majeure
(`cargo metadata` sur les deux espaces de travail, champ `license` de chaque paquet).

## Fichiers embarqués dans le dépôt

| Composant | Emplacement | Licence |
| --- | --- | --- |
| three.js | `crates/revel-render/vendor/three/` | MIT |
| Draco (décodeur) | `crates/revel-render/vendor/three/draco_*` | Apache-2.0 |
| Basis Universal (transcodeur) | `crates/revel-render/vendor/three/basis_transcoder.*` | Apache-2.0 |
| lottie-web | `crates/revel-render/vendor/lottie-web/` | MIT |
| modern-screenshot | `crates/revel-core/vendor/modern-screenshot/` | MIT |
| Police Geist | `apps/revel-ui/fonts/` | SIL Open Font License 1.1 |

## Dépendances Rust

Espace de travail principal : 812 paquets tiers ; interface (`apps/revel-ui`) : 206. Toutes sous une licence
approuvée par l'OSI, aucune sous GPL, LGPL ou AGPL imposée (les paquets qui citent GPL ou LGPL la proposent au
choix avec MIT ou Apache-2.0).

| Licence | Remarque |
| --- | --- |
| MIT, Apache-2.0 (seules ou au choix) | la quasi-totalité |
| BSD-2-Clause, BSD-3-Clause, ISC, Zlib, 0BSD, BSL-1.0, Unlicense, CC0-1.0, NCSA, IJG | permissives |
| Unicode-3.0 | données Unicode (ICU) |
| MPL-2.0 | `cssparser`, `cssparser-macros`, `selectors`, `dtoa-short`, `option-ext` : copyleft limité au fichier ; utilisés sans modification, leur source reste disponible sur crates.io |
| CDLA-Permissive-2.0 | `webpki-roots`, `webpki-root-certs` (certificats racines de Mozilla) |

## Téléchargés à la demande, non distribués avec Revel

- Modèles de traduction Helsinki-NLP (Hugging Face), sous leurs licences propres (CC-BY-4.0 pour la plupart).
- Polices Google Fonts choisies par l'utilisateur (OFL ou Apache-2.0 selon la police).
- Node.js (MIT), pour les sites qui se construisent avec npm.
