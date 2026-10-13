app-title = Revel
app-tagline = Création visuelle de sites web
app-version = Version { $version }
app-stage = Étape { $stage } : squelette modulaire

menu-file = Fichier
menu-site = Site
menu-publish = Publier
menu-help = Aide

language = Langue

deploy-warn-no-301 = Cet hébergeur ne gère pas les vraies redirections 301 : { $count } redirection(s) deviendront des redirections côté navigateur, moins bien prises en compte par les moteurs de recherche.
deploy-warn-no-php = Cet hébergeur n’exécute pas PHP : { $count } fichier(s) PHP (par exemple le formulaire de contact) ne fonctionneront pas.
deploy-warn-no-headers = Cet hébergeur ne permet pas d’en-têtes HTTP personnalisés (indexation, cache, sécurité).
deploy-warn-no-htaccess = Cet hébergeur ignore les fichiers .htaccess : leurs règles (HTTPS, redirections, accès) ne s’appliqueront pas.
deploy-warn-site-too-big = Le site ({ $size } Mo) dépasse la limite de cet hébergeur ({ $max } Mo).
deploy-warn-file-too-big = Un fichier ({ $size } Mo) dépasse la limite par fichier de cet hébergeur ({ $max } Mo).
deploy-warn-too-many-files = Le site compte { $count } fichiers ; cet hébergeur en accepte au plus { $max }.
deploy-warn-public-repo = Sur une offre gratuite, cet hébergeur exige que le dépôt source soit public.

home-title = Sites
home-empty = Aucun site pour l’instant. Ouvrez un dossier contenant un site Revel, ou importez un projet Eleventy.
action-open = Ouvrir un site
action-import = Importer un projet Eleventy
action-new = Nouveau site
action-forget = Retirer de la liste
action-cancel = Annuler
action-save = Enregistrer
action-delete = Supprimer
new-site-name = Nom du site
import-overwrite = Ce dossier contient déjà un site Revel. L’importer de nouveau remplace sa structure (pages, gabarits, collections) ; les contenus et images ne sont pas modifiés.
import-done = Import terminé
import-warnings = Points à vérifier
site-back = Tous les sites
site-pages = Pages
site-layouts = Gabarits
site-components = Composants
site-collections = Collections
site-entries = { $count ->
        [one] { $count } entrée
       *[other] { $count } entrées
    }
build-title = Générer
build-env = Environnement
build-images = Images responsives (AVIF, WebP)
build-run = Générer le site
build-running = Génération…
build-phase-pages = Pages
build-phase-copies = Fichiers
build-phase-images = Images { $done }/{ $total }
build-done = { $generated } pages générées, { $copied } fichiers copiés, images : { $produced } nouvelles, { $cached } réutilisées, en { $seconds } s.
build-warnings = Avertissements
build-failed = Échec de la génération. { $message }
preview-open = Aperçu
publish-title = Publier
publish-empty = Aucun hébergement pour ce site. Ajoutez les accès SFTP fournis par votre hébergeur pour publier en un clic.
publish-add = Ajouter un hébergement
publish-name = Nom
publish-env = Environnement généré
publish-host = Serveur SFTP
publish-port = Port
publish-user = Identifiant
publish-remote-dir = Dossier sur le serveur
publish-password = Mot de passe
publish-password-saved = Déjà enregistré dans le trousseau du système. Laisser vide pour le conserver.
publish-check = Vérifier
publish-run = Publier
publish-planned = { $upload ->
        [one] { $upload } fichier à envoyer
       *[other] { $upload } fichiers à envoyer
    } ({ $size } Mo), { $delete } à supprimer, { $total } fichiers sur le site.
publish-progress = Envoi : { $done } / { $total } { $total ->
        [one] fichier
       *[other] fichiers
    }
publish-done = { $uploaded ->
        [0] { $deleted ->
            [0] Rien à publier : le site en ligne est déjà à jour.
           *[other] Publication terminée : { $deleted } { $deleted ->
                [one] fichier supprimé
               *[other] fichiers supprimés
            }.
        }
       *[other] Publication terminée : { $uploaded } { $uploaded ->
            [one] fichier envoyé
           *[other] fichiers envoyés
        }, { $deleted } { $deleted ->
            [one] supprimé
           *[other] supprimés
        }.
    }
publish-dry-done = { $upload ->
        [0] { $delete ->
            [0] Vérification terminée : le site en ligne est déjà à jour.
           *[other] Vérification terminée, rien n’a été modifié sur le serveur. La prochaine publication supprimera { $delete } { $delete ->
                [one] fichier
               *[other] fichiers
            }.
        }
       *[other] Vérification terminée, rien n’a été modifié sur le serveur. La prochaine publication enverra { $upload } { $upload ->
            [one] fichier
           *[other] fichiers
        } ({ $size } Mo) et en supprimera { $delete }.
    }
publish-key-recorded = Première connexion : la clé du serveur est enregistrée et sera exigée aux prochaines connexions. Comparez son empreinte avec celle indiquée par votre hébergeur :
publish-failed = Échec de la publication. { $message }
publish-warnings = Avant de publier
error = Erreur : { $message }
nav-sites = Sites
nav-recent = Récents
nav-all-sites = Tous les sites
nav-overview = Vue d’ensemble
nav-build = Générer
nav-publish = Publier
nav-settings = Réglages
home-col-name = Nom
home-col-folder = Dossier
home-col-opened = Ouvert le
settings-lang = Langue
settings-theme = Thème
theme-system = Système
theme-dark = Sombre
theme-light = Clair
settings-accent = Couleur d’accent
settings-accent-help = Utilisée pour les boutons principaux, la sélection et le focus. « Aucune » garde l’interface neutre.
accent-none = Aucune
accent-custom = Personnalisée
overview-content = Contenus
overview-structure = Structure
overview-environments = Environnements
action-close = Fermer
action-edit = Modifier
action-replace = Remplacer
new-site-title = Nouveau site
new-site-choose = Choisir le dossier
import-overwrite-title = Remplacer le site existant ?
publish-default-name = Mon hébergement
publish-form-new = Nouvel hébergement SFTP
publish-form-edit = Modifier l’hébergement
publish-form-help = Accès SFTP fournis par votre hébergeur (OVH, o2switch, Infomaniak…), généralement dans l’espace client, rubrique FTP-SSH.
publish-host-help = Exemple : ftp.cluster000.hosting.ovh.net
publish-remote-dir-help = Dossier du site sur le serveur, relatif à votre dossier personnel. Exemple : www/
publish-password-help = Enregistré dans le trousseau du système, jamais dans les fichiers du site.
publish-check-help = Se connecte au serveur et compare les fichiers, sans rien envoyer ni supprimer.
publish-connecting = Connexion au serveur…
publish-comparing = Comparaison avec les fichiers en ligne…
publish-check-failed = Échec de la vérification. { $message }
publish-delete-title = Supprimer « { $name } » ?
publish-delete-body = Les accès et le mot de passe enregistrés seront effacés. Les fichiers déjà en ligne ne sont pas modifiés.
error-detail = { $detail }
error-no-desktop = Cette action nécessite l’application de bureau Revel.
error-project = Le site n’a pas pu être lu : { $detail }
error-import = L’import a échoué : { $detail }
error-io = Erreur de lecture ou d’écriture : { $detail }
error-already-imported = Ce dossier contient déjà un site Revel.
error-not-eleventy = Ce dossier n’est pas un projet Eleventy (aucun fichier eleventy.config.js ou .eleventy.js).
error-unknown-profile = Cet hébergement n’existe plus. Rechargez la page Publier.
error-missing-secret = Mot de passe absent du trousseau : modifiez l’hébergement pour le saisir de nouveau.
error-keyring = Le trousseau du système n’est pas accessible : { $detail }
error-blocked = Publication bloquée : corrigez d’abord les points signalés.
error-not-built = Le site n’a pas encore été généré pour cet environnement. Générez-le, puis ouvrez l’aperçu.
error-sftp-connect = Connexion à { $host } impossible. Vérifiez l’adresse du serveur, le port et votre connexion internet. ({ $detail })
error-sftp-host-key = La clé du serveur a changé depuis la dernière connexion. Cela peut venir d’un changement chez l’hébergeur ou d’une tentative d’interception : vérifiez auprès de votre hébergeur avant de continuer.
error-sftp-auth = Identifiant ou mot de passe refusé par le serveur.
error-sftp = Le serveur a renvoyé une erreur : { $detail }
error-sftp-remote-dir = Dossier sur le serveur invalide : « { $dir } ». Indiquez un chemin relatif, par exemple www/
error-sftp-file-name = Nom de fichier non pris en charge : { $name }
publish-fix = Modifier l’hébergement
