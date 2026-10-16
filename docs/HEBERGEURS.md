# Hébergeurs proposés à la première mise en ligne (5.2)

Données reprises dans `crates/revel-protocol/src/hosting.rs` (liste de l'assistant de mise en ligne) et, plus
tard, dans le guide du nom de domaine (5.7) et les formulaires (5.8). Aides officielles consultées le
16/10/2026 ; « non vérifié » : pas de source officielle trouvée.

| Hébergeur | SFTP | Serveur | Port | Dossier du site | Prérequis |
|---|---|---|---|---|---|
| OVHcloud | toutes offres sauf 60free, demo1g | `ftp.clusterXXX.hosting.ovh.net` | 22 (Cloud Web : port de l'espace client) | `www/` (colonne « Dossier racine » de « Mes sites ») | cocher SFTP pour l'utilisateur (onglet FTP - SSH) |
| o2switch | toutes offres | domaine ou `xxx.o2switch.net` (courriel « Bienvenue chez o2switch ») | 22 | `public_html/` | adresse IP autorisée : cPanel › Outils › Autorisation SSH (5 au plus) ; compte cPanel principal seulement |
| Infomaniak | Hébergement Web payant (pas Starter) | `xxxx.ftp.infomaniak.com` | 22 | `sites/<domaine>/` (fiche du site, « Informations ») | aucun |
| Hostinger | Premium et au-dessus (pas Single) | adresse IP (« Accès SSH ») | 65002 | `public_html/` ou `domains/<domaine>/public_html/` | activer SSH : Sites web › Tableau de bord › Accès SSH |
| IONOS | Hébergement Web Linux | `accessNNNNNNNNN.webspace-data.io` | 22 | dossier relié au domaine (Domaines & SSL) | créer l'accès : Hébergement › SFTP & SSH › Configurer |

**Où trouver les accès**

- OVHcloud : Web Cloud › Hébergements › l'hébergement › onglet FTP - SSH (« Serveur FTP et SFTP », « Login
  principal ») ; le mot de passe n'est jamais affiché, il se change dans la colonne « Mot de passe ».
  [FTP connection](https://docs.ovhcloud.com/fr/guides/web-cloud/web-hosting/ftp-connection),
  [Activer SFTP](https://docs.ovhcloud.com/fr/guides/web-cloud/web-hosting/enable-sftp),
  [FileZilla](https://docs.ovhcloud.com/fr/guides/web-cloud/web-hosting/ftp-filezilla-user-guide).
- o2switch : identifiant et mot de passe cPanel (courriel « Bienvenue chez o2switch ») ; sans adresse IP
  autorisée, la connexion échoue (délai dépassé).
  [Exception pare-feu](https://faq.o2switch.fr/cpanel/outils/exception-parefeu/),
  [FileZilla](https://faq.o2switch.fr/guides/configuration-logiciel/filezilla/),
  [public_html et www](https://faq.o2switch.fr/questions/difference-public-html-et-www/).
- Infomaniak : Manager › Hébergement Web › FTP / SSH (serveur en haut de page, compte `xyzb_abcdefg`).
  [Accès FTP](https://www.infomaniak.com/fr/support/faq/1083/acceder-au-serveur-par-ftp-avec-une-application),
  [Comptes FTP/SSH](https://www.infomaniak.com/fr/support/faq/1982/gerer-les-comptes-ftp-ssh),
  [Protocoles](https://www.infomaniak.com/fr/support/faq/446/comprendre-les-protocoles-de-transfert-web).
- Hostinger : hPanel › Sites web › Tableau de bord › Accès SSH (adresse IP, port, identifiant `u12345678`) ;
  mot de passe : celui du FTP.
  [SFTP](https://www.hostinger.com/support/5972689-how-to-connect-to-your-hosting-using-sftp-in-hostinger/),
  [Activer SSH](https://www.hostinger.com/support/1583645-how-to-enable-ssh-access-in-hostinger/),
  [Dossier du site](https://www.hostinger.com/support/1583494-what-is-the-path-to-your-website-s-root-home-directory-and-how-to-change-it-in-hostinger/).
- IONOS : Menu › Hébergement › SFTP & SSH › Administrer › identifiant (`u12345678`, `su123456`).
  [Créer un compte SFTP](https://www.ionos.fr/assistance/hosting/ftp-zugaenge-einrichten-und-verwalten/creer-un-compte-sftp/ssh/),
  [Données de connexion](https://www.ionos.fr/assistance/hebergement/configurer-et-gerer-lacces-ftp/donnees-de-connexion-pour-ftp/sftp-dans-lhebergement-web-company-name/).

Libellés anglais (aides officielles en anglais) : OVHcloud « Web Cloud › Hosting plans › FTP - SSH », « FTP and SFTP
server », « Main login » ([enable SFTP](https://docs.ovhcloud.com/en/guides/web-cloud/web-hosting/enable-sftp)) ;
Infomaniak « hosting & website management › FTP / SSH »
([FAQ 1982](https://www.infomaniak.com/en/support/faq/1982/manage-ftp-ssh-accounts)) ; Hostinger « Websites ›
Dashboard › SSH Access › Enable » ([SSH](https://www.hostinger.com/support/1583645-how-to-enable-ssh-access-in-hostinger/)) ;
IONOS « Menu › Hosting › SFTP & SSH › Set up / Manage »
([SFTP user](https://www.ionos.com/help/hosting/setting-up-and-managing-ftp-access/setting-up-the-main-sftp-user/)).
o2switch n'a d'aide qu'en français : libellés français gardés dans l'interface anglaise.

**Envoi de courriels par PHP (formulaires, 5.8)**

| Hébergeur | `mail()` | Limite | Remarque |
|---|---|---|---|
| OVHcloud | oui | 10 à 2000 par heure selon l'offre | `From:` d'un domaine de l'hébergement, visiteur en `Reply-To:` ([bonnes pratiques](https://docs.ovhcloud.com/fr/guides/web-cloud/web-hosting/email-sending-best-practices), [limites](https://docs.ovhcloud.com/fr/guides/web-cloud/web-hosting/hosting-technical-specificities)) |
| o2switch | non vérifié | non publiée | infrastructure d'envoi filtrée ([SMTP sortant](https://faq.o2switch.fr/guides/emails/serveurs-smtp-sortant/)) |
| Infomaniak | désactivé par défaut | 50 par 24 h | à activer par site : paramètres avancés › PHP / Apache › fonction mail ([FAQ 2150](https://www.infomaniak.com/fr/support/faq/2150/utiliser-lenvoi-non-authentifie-non-recommande-de-mail-depuis-un-site), [FAQ 2065](https://www.infomaniak.com/fr/support/faq/2065/comprendre-les-limites-de-mails-sortants-par-24-heures)) |
| Hostinger | oui | 10 par minute, 100 par jour | ([limites](https://www.hostinger.com/support/11393648-php-mail-limitation-explained-how-to-improve-email-delivery-with-smtp/)) |
| IONOS | oui | non vérifié | `From:` d'un domaine du contrat ([expéditeur](https://www.ionos.fr/assistance/hebergement/aucun-envoi-de-mails-avec-une-adresse-dexpediteur-differente/)) |

**Nom de domaine (5.7)**

- OVHcloud : Mes sites › Ajouter un site › Associer un domaine externe ; A (et AAAA facultatif) vers l'IP du
  cluster, TXT `ovhcontrol` obligatoire.
  [Multisite](https://docs.ovhcloud.com/fr/guides/web-cloud/web-hosting/multisites-configure-multisite),
  [IP des clusters](https://docs.ovhcloud.com/fr/guides/web-cloud/web-hosting/clusters-and-shared-hosting-ip).
- o2switch : serveurs `ns1.o2switch.net`, `ns2.o2switch.net`, ou A vers l'IP partagée (cPanel) ; domaine
  déclaré dans cPanel › Domaines configurés.
  [Pointage](https://faq.o2switch.fr/nom-de-domaine/pointage-domaine/).
- Infomaniak : A (et AAAA) vers l'IP du site (fiche du site, « Informations »).
  [Lier un domaine](https://www.infomaniak.com/fr/support/faq/2025/lier-un-nom-de-domaine-a-un-hebergement-web-infomaniak).
- Hostinger : serveurs de noms affichés, ou A pour `@` et `www` vers l'IP ; DNSSEC désactivé.
  [Pointer un domaine](https://www.hostinger.com/support/1863967-how-to-point-a-domain-to-hostinger/).
- IONOS : serveurs de noms IONOS, ou TXT de confirmation sur `@` (domaine externe).
  [Domaine externe](https://www.ionos.fr/assistance/domaines/enregistrer-et-gerer-des-domaines-externes-chez-11-ionos/configurer-un-domaine-externe-chez-ionos-instructions-detaillees/).
- GitHub Pages : A `185.199.108.153`, `185.199.109.153`, `185.199.110.153`, `185.199.111.153` ; AAAA
  `2606:50c0:8000::153` à `2606:50c0:8003::153` ; CNAME `www` vers `<compte>.github.io` ; HTTPS par
  « Enforce HTTPS » une fois le certificat obtenu.
  [Domaine personnalisé](https://docs.github.com/fr/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site),
  [HTTPS](https://docs.github.com/fr/pages/getting-started-with-github-pages/securing-your-github-pages-site-with-https).
