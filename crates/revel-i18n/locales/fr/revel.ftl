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
deploy-warn-site-too-big = Le site ({ $size } octets) dépasse la limite de cet hébergeur ({ $max } octets).
deploy-warn-file-too-big = Un fichier ({ $size } octets) dépasse la limite par fichier de cet hébergeur ({ $max } octets).
deploy-warn-too-many-files = Le site compte { $count } fichiers ; cet hébergeur en accepte au plus { $max }.
deploy-warn-public-repo = Sur une offre gratuite, cet hébergeur exige que le dépôt source soit public.
