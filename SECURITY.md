# Politique de Sécurité

## Versions supportées

Seule la version stable la plus récente de Ygégé est garantie d'être mise à jour avec des correctifs de sécurité. **Utiliser des versions obsolètes peut vous exposer à des risques de sécurité** ; veillez à toujours utiliser la dernière version, ou évitez d'exposer votre instance sur l'Internet public pour minimiser les risques. De plus, comprenez bien que donner un accès administrateur à votre configuration Ygégé est un risque, car les administrateurs peuvent effectuer de nombreuses actions destructrices, indépendamment de toute faille potentielle.

Par conséquent, cette politique de sécurité s'applique uniquement à la version stable la plus récente de Ygégé. Les vulnérabilités présentes dans les anciennes versions qui ne sont plus présentes dans la version actuelle **ne seront pas** corrigées.

## Triage des vulnérabilités

Nous vous demandons de bien vouloir examiner les détails suivants avant de signaler un problème :

* Nous sommes conscients que de nombreuses opérations de configuration et d'administration peuvent avoir des implications en matière de sécurité. En raison du fonctionnement interne de Ygégé, beaucoup de ces implications sont inévitables. Toute vulnérabilité nécessitant **exclusivement des privilèges administrateur ou un accès direct au système de fichiers** sera considérée comme de faible priorité et pourra être signalée publiquement dans les Issues GitHub classiques.

* Nous avons une liste publique des vulnérabilités connues dans la section [Avis de Sécurité (Security Advisories)](https://github.com/UwUDev/ygege/security/advisories). Si votre trouvaille y figure **déjà**, merci de ne pas dupliquer le travail de notre équipe en le signalant de nouveau.

* Les vulnérabilités qui ne peuvent pas être **exploitées à distance** sont également considérées comme des bugs de priorité faible à moyenne (ex : tout ce qui nécessite un accès shell au serveur Ygégé, la manipulation manuelle de la base de données ou des logs, etc.).

* Les rapports de vulnérabilité concernant l'infrastructure du projet (serveurs en ligne, CI/CD, etc.) sont les bienvenus, mais veuillez les étiqueter avec `[Infrastructure Ygégé]`.

## Signaler une vulnérabilité

Une fois triée personnellement, et si la faille s'avère nouvelle et pertinente, veuillez nous contacter pour une divulgation responsable via la plateforme des [Avis de Sécurité GitHub (GitHub Security Advisories)](https://github.com/UwUDev/ygege/security/advisories/new).

Lors de votre signalement, assurez-vous de :

1. Commencer le titre de votre rapport par `[Sécurité Ygégé]`. Cela nous aide pour le tri et la visibilité.
2. Commencer par une section "Vue d'ensemble", **rédigée pour une publication future**, décrivant ce qui est affecté et les conséquences possibles. Idéalement, nous utiliserons ce texte tel quel pour décrire l'avis final.
3. Poursuivre avec une section "Détails" expliquant vos recherches dans le code ou l'API, les étapes exactes de reproduction et/ou un script PoC (Proof of Concept), et si possible, un début de solution ou correctif suggéré.
4. Fournir votre nom d'utilisateur GitHub pour que nous puissions vous inviter dans la discussion privée du GHSA et vous créditer lors de la publication.

Une fois le rapport reçu, il sera examiné. Si pertinent, nous créerons un ticket privé GitHub Security Advisory (GHSA) dans lequel vous serez invité pour échanger de manière sécurisée et tester les correctifs.

## Processus Post-Divulgation

En tant que projet soutenu par des bénévoles de la communauté open-source, **nous reconnaissons que nous pouvons parfois mettre du temps** avant d'apporter une solution complète ; nous apprécions par avance votre patience et l'absence d'ultimatums stricts, particulièrement pour les vulnérabilités complexes.

En règle générale, une version corrective rapide ("point release") sera déployée pour toute vulnérabilité majeure dès que le patch sera prêt. Si une version majeure est sur le point d'arriver dans les jours qui suivent, nous pourrions différer le correctif à cette version pour éviter les fusions de code compliquées.

Une fois la nouvelle version sécurisée publiée, **nous attendrons au moins 7 jours (1 semaine) avant de rendre le GHSA**. Nous pensons que ce délai offre un bon compromis entre une divulgation publique rapide et la certitude que la majorité de nos utilisateurs aura eu le temps de mettre à jour leurs instances privées. Nous vous demandons que toute divulgation par un tiers de votre côté (articles de blogs, tweets, etc.) ait lieu **après** la fin de ce délai de grâce.

Si applicable, les numéros de CVE seront demandés par nos soins via l'interface de sécurité GitHub et publiés avec la divulgation complète.

## Bonnes Pratiques de Sécurité

Lors du déploiement de l'indexeur Ygégé :

- Utilisez toujours la dernière version stable, ou l'image Docker contenant les dernières dépendances.
- Maintenez vos images Docker à jour régulièrement (`docker pull uwucode/ygege:latest`).
- Limitez l'exposition réseau de votre conteneur Ygégé en ne le liant qu'à vos applications locales (Prowlarr, Jackett) plutôt qu'en l'exposant vers l'extérieur (ex: bind IP sur `127.0.0.1` ou réseau interne Docker).
- Envisagez de placer Ygégé derrière un reverse proxy (NGINX, Traefik) avec du chiffrement SSL/TLS si vous deviez y accéder à distance sur un serveur non sécurisé.
- Ne placez jamais de secrets, jetons FlareSolverr, de mot de passe réseau dans des dépôts git publics.
- Surveillez vos logs docker pour détecter une activité suspecte ou des erreurs d'authentification massives.

## Remerciements

Nous sommes incroyablement reconnaissants envers tous les chercheurs en sécurité informatiques et les curieux qui prennent le temps d'auditer notre système et nous rapportent les vulnérabilités de manière responsable, aidant ainsi à garder la communauté Ygégé en constante sécurité.
