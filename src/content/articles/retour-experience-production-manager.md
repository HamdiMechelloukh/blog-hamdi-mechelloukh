Pendant deux ans et demi, j'ai quitté le code pour gérer la production des données de ventes chez Decathlon Digital. Un rôle que j'ai découvert en arrivant — il s'appelait "Production Expert" dans la fiche de poste, et j'ai très vite compris que ça allait devenir du temps plein.

Voici ce que j'ai appris en passant de l'autre côté.

## Le contexte : Perfeco et les données de ventes

Perfeco était le produit data qui mettait à disposition les données de performance économique et de ventes de l'entreprise. Concrètement, c'était :

- Un pipeline d'ingestion basé sur **Talend** et **Redshift** — les données étaient traitées et stockées dans Redshift puis déversées dans S3
- **2 à 3 millions de ventes par jour** ingérées
- Des données exposées dans le datalake et via une API consommée par plusieurs équipes métier
- Des messages Kafka avec des payloads XML convertis en CSV avant chargement
- Un scheduler (OpCon) pour orchestrer les jobs d'ingestion

Mon rôle : **garantir que tout ça tourne**, tous les jours, sans interruption.

## Ce que "Production Manager" veut dire au quotidien

Quand on vient du développement, on imagine que la production c'est du monitoring et quelques alertes. La réalité est très différente.

### Réduire le run

Mon objectif principal n'était pas de réagir aux incidents, mais de **réduire leur fréquence**. Ça passait par :

- **Alerting proactif** — mettre en place les bons dashboards (QuickSight, Tableau) et les bonnes alertes pour détecter les anomalies avant qu'elles ne deviennent des incidents. Création automatique de tickets Jira quand un seuil est franchi.
- **Qualité des données à la source** — analyser et détecter les problèmes de qualité en amont, puis remonter ces problèmes aux équipes sources. C'est un travail d'animation, pas de code : convaincre une équipe en amont que leur donnée est mal formatée, ça prend du temps.
- **Documentation du run** — écrire et maintenir les procédures d'astreinte pour que n'importe quel membre de l'équipe puisse intervenir à 3h du matin sans dépendre de la mémoire d'une seule personne.
- **KPIs de run** — scripter la collecte de métriques pour mesurer objectivement la stabilité : nombre d'incidents, temps de résolution, disponibilité des données.

### Animer, pas coder

La surprise la plus grande a été la **dimension relationnelle** du poste. Je passais plus de temps à gérer de l'humain que de la technique :

- **Animation des équipes consommatrices** — améliorer la communication incident. Quand une ingestion est en retard, 5 équipes différentes ont besoin de savoir pourquoi et quand ça sera résolu. Il faut un canal clair, un message clair, et de la régularité.
- **Animation des sources** — travailler avec les équipes qui produisent les données en amont pour qu'elles corrigent les problèmes de qualité à la racine.
- **Planification de l'astreinte** — organiser les rotations pour l'équipe, s'assurer que tout le monde est formé et que la charge est répartie équitablement.
- **Postmortems** — j'organisais des points réguliers avec les sources et les consommateurs de données. C'est lors de ces réunions que les postmortems étaient remplis collectivement : ce qui s'est passé, pourquoi, et les actions pour éviter la récurrence. Ce format collaboratif permettait d'aligner tout le monde et d'éviter le jeu du blâme.

## L'incident type : quand le scheduler plante

Ma bête noire pendant ces deux ans, c'était le client du scheduler OpCon qui plantait sur la machine. Silencieusement.

Le scénario était toujours le même :

1. Le client OpCon crashe → plus aucun job ne se lance
2. Les ventes continuent d'arriver via Kafka (des messages avec des payloads XML)
3. Les messages s'accumulent — des centaines de milliers en quelques heures
4. Quand on relance le scheduler, le job de conversion XML → CSV se retrouve face à une charge massive
5. Le job Talend souffre, les temps de traitement explosent, Redshift est saturé, les données arrivent en retard dans S3

Les plus gros incidents qu'on a eus étaient liés à ce problème. Ce qui rendait la chose frustrante, c'est que le crash du client était silencieux — pas d'alerte, pas de log explicite. On ne le découvrait qu'en constatant l'absence de données en aval.

La leçon : **monitorer l'absence d'événements est aussi important que monitorer les erreurs**. Si un job qui tourne toutes les 15 minutes ne s'est pas exécuté depuis 30 minutes, c'est un signal fort.

## Ce que j'ai appris

### La production, c'est de l'ingénierie

Réduire le run, ce n'est pas juste "mettre des alertes". C'est concevoir un système d'observabilité, automatiser la détection, documenter les procédures, et mesurer l'amélioration. C'est un travail d'ingénierie à part entière.

### La communication est un skill technique

Rédiger un message d'incident clair, animer un postmortem sans chercher de coupable, convaincre une équipe source de corriger un format de donnée — ce sont des compétences aussi importantes que savoir écrire du code. Et elles se travaillent.

### L'alerting proactif change tout

La différence entre un PM qui subit et un PM qui gère, c'est la proactivité. Quand on découvre un incident par une alerte automatique à 8h au lieu d'un appel d'un métier à 10h, on a gagné 2 heures et beaucoup de sérénité.

### Monitorer le silence

Les incidents les plus vicieux ne génèrent pas d'erreur — ils génèrent du silence. Un pipeline qui ne tourne plus, un scheduler qui a crashé, un message qui n'arrive jamais. Les alertes sur l'absence d'activité m'ont sauvé plus souvent que les alertes sur les erreurs.

### La documentation n'est pas optionnelle

En dev, on peut parfois s'en sortir avec un code lisible et quelques commentaires. En production, si la procédure d'astreinte n'est pas écrite, elle n'existe pas. La personne d'astreinte à 3h du matin n'a pas le temps de deviner.

## Pourquoi je suis revenu côté technique

Après deux ans et demi, j'ai pris la décision de revenir vers un rôle de Data Engineer. La raison est simple : **je sentais que je régressais techniquement**.

Le quotidien du PM est passionnant — la diversité des problèmes, la dimension humaine, l'impact direct sur la fiabilité des données. Mais je passais mes journées à animer, documenter et communiquer, et de moins en moins à concevoir et coder.

J'avais peur de décrocher, de ne plus être au niveau sur les technologies qui évoluent vite — Spark, Databricks, les architectures lakehouse. Le risque de devenir un profil purement gestion sans expertise technique ne me convenait pas.

Aujourd'hui, avec le recul, je ne regrette pas cette expérience. Elle m'a donné une compréhension de la production que beaucoup de développeurs n'ont pas. Quand je conçois un pipeline maintenant, je pense naturellement à l'observabilité, à la reprise sur erreur, à la documentation d'exploitation. Ce sont des réflexes que le code seul ne m'aurait pas donnés.

## En résumé

Si vous êtes développeur et qu'on vous propose un rôle orienté production, voici ce que j'en retiens :

- **C'est un vrai métier**, pas un rôle de support. Ça demande de l'ingénierie, de la rigueur et beaucoup de soft skills.
- **Vous allez apprendre des choses que le dev ne vous apprendra jamais** — la communication de crise, la gestion des priorités sous pression, la vision end-to-end d'un produit data.
- **Fixez-vous une durée**. C'est enrichissant, mais si votre cœur de métier est technique, ne restez pas trop longtemps au risque de décrocher.
- **Ramenez ces réflexes dans votre code**. L'observabilité, la documentation, le monitoring du silence — ce sont des compétences qui font de meilleurs ingénieurs.
