Quand on travaille dans la data, on finit toujours par se poser la même question : **que se passe-t-il si demain on doit changer de plateforme ?**

J'ai pu constater de première main que les éditeurs de logiciels sont assez agressifs dans leur politique de pricing, et qu'ils n'hésitent pas à abandonner une solution qui ne génère pas assez de chiffre d'affaires. Quand ça arrive, il faut pouvoir migrer rapidement — sous peine de coûts faramineux en migration, en re-développement, et en temps perdu.

C'est cette conviction qui m'a poussé à construire **un lakehouse open-source et vendor-neutral de bout en bout**, du messaging à la visualisation. Voici les choix d'architecture, les compromis, et ce que j'en ai appris.

## Le stack : Kafka → Spark → Iceberg

```
Sources → Kafka → Spark (Bronze) → Spark (Silver) → Spark (Gold) → Streamlit
                                                                   ↑
                                                          Great Expectations
                                                          (qualité à chaque couche)
```

### Kafka pour l'ingestion

Le choix du event-driven pour le transfert de données n'est pas anodin. En informatique, **gérer le temps est un des problèmes les plus complexes**. Côté opérationnel, on va de plus en plus vers des architectures event-driven justement pour ça — un événement arrive quand il arrive, et le système le traite. Pas de fenêtre batch à respecter, pas de "le fichier aurait dû arriver à 6h".

Kafka est le standard de fait pour ce type d'architecture. Open-source, battle-tested, et surtout : pas de lock-in vendor. On peut le déployer sur n'importe quel cloud ou on-premise.

### Spark pour le compute

On pourrait se demander : pourquoi Spark dans une architecture event-driven ? Ma position est pragmatique. Le streaming pur via Kafka convient très bien pour l'ingestion dans le bronze, voire le silver, pour **gérer la temporalité en amont**. Mais dès qu'on arrive aux transformations lourdes — agrégations, jointures, enrichissements — Spark reste l'outil le plus éprouvé et le plus portable.

L'avantage de Spark, c'est qu'il tourne partout : sur un cluster YARN, sur Kubernetes, sur Databricks, sur EMR, ou en local pour le développement. C'est un des rares outils de compute qui ne vous enferme pas.

### Iceberg pour le format de table

Iceberg est le format de table ouvert qui monte en puissance. Mon choix était en partie de la curiosité technologique — au travail j'utilise Delta Lake au quotidien, donc je voulais explorer l'alternative.

Mais au-delà de la curiosité, Iceberg a des propriétés qui le rendent particulièrement adapté à un lakehouse vendor-neutral :

- **Format ouvert** — pas de dépendance à un éditeur spécifique
- **Time travel** — requêter les données à un instant T
- **Schema evolution** — ajouter ou modifier des colonnes sans réécrire les données
- **Partition evolution** — changer le schéma de partitionnement sans migration
- **Compatible avec tous les moteurs** — Spark, Trino, Flink, Dremio, Athena...

Le projet pourrait tout aussi bien tourner avec Delta Lake ou Hudi. D'ailleurs, ce serait intéressant de proposer le choix du format à qui voudrait forker le projet.

## L'architecture en couches : bronze, silver, gold

Le pattern médaillon (bronze/silver/gold) structure la donnée en trois niveaux de raffinement :

- **Bronze** — donnée brute telle qu'elle arrive, sans transformation
- **Silver** — donnée nettoyée, dédupliquée, typée correctement
- **Gold** — donnée agrégée et prête à la consommation métier

Honnêtement, ces termes sont récents. Il y a quelques années, on parlait de dataraw, dataprep, dataset. Le vocabulaire change, le principe reste le même. L'important est de **rester dans cette logique de raffinement progressif sans être rigide** — la réalité fonctionnelle prévaut toujours sur les règles techniques. Si une donnée n'a pas besoin de passer par trois couches, elle n'a pas besoin de passer par trois couches.

## MinIO : un S3-compatible sans le lock-in

Un point qui peut surprendre : pourquoi MinIO plutôt que S3 directement ?

Parce que **S3 est un service AWS**, et utiliser S3, c'est s'enfermer chez AWS. MinIO implémente l'API S3 à l'identique — tous les outils qui parlent S3 parlent MinIO sans modification. On peut développer et tester en local, déployer sur n'importe quel cloud, et migrer vers S3, GCS ou Azure Blob Storage sans changer une ligne de code applicatif.

C'est exactement le principe du vendor-neutral : **utiliser les standards ouverts plutôt que les services managés propriétaires**.

## Qualité des données : Great Expectations et ses limites

Great Expectations est l'outil de validation de données le plus utilisé dans l'écosystème Python/Spark. Je l'ai intégré à chaque couche du pipeline pour valider les données en entrée et en sortie.

L'outil fait bien son travail pour les règles de qualité simples : nullité, unicité, plages de valeurs, formats. C'est aussi un outil que j'ai vu utilisé en entreprise, ce qui validait le choix.

Mais il a des **limites réelles** :

- Les règles de qualité complexes (cohérence inter-tables, règles métier conditionnelles) sont difficiles à exprimer
- Les checks coûteux en ressources (jointures massives pour la détection de doublons cross-sources) ne passent pas à l'échelle facilement
- Et surtout : **découvrir des problèmes de qualité ne suffit pas**

Ce dernier point est crucial et vient directement de mon expérience en production chez Decathlon. Vous pouvez mettre en place toutes les alertes de qualité du monde — si les équipes sources n'ont aucun engagement à corriger les problèmes, rien ne changera. Il faut travailler sur des **contrats de service en termes de qualité de données** : SLAs sur les délais de correction, responsabilités partagées, escalade claire. Sans ça, les sources feront peu d'effort dans la résolution.

## La difficulté du vendor-neutral

Le plus gros défi de ce projet n'était pas technique au sens classique. C'était de **résister à la tentation du service managé**.

À chaque étape, il existe une option managée qui fait gagner du temps :

- Pourquoi gérer son Kafka quand il y a Amazon MSK ou Confluent Cloud ?
- Pourquoi MinIO quand S3 est là, configuré en 2 clics ?
- Pourquoi Airflow self-hosted quand il y a MWAA ?

La réponse est toujours la même : **parce que le jour où le pricing change ou le service est déprécié, vous devez pouvoir partir**. Ça ne veut pas dire qu'il ne faut jamais utiliser de services managés — ça veut dire qu'il faut le faire en connaissance de cause, et s'assurer que la couche d'abstraction permet de switcher.

En pratique, construire vendor-neutral demande plus d'effort initial :

- Terraform pour gérer l'infra de manière déclarative et multi-cloud
- Docker pour l'isolation et la portabilité
- Des interfaces standard partout (S3 API, JDBC, etc.)

Mais une fois que c'est en place, la liberté que ça procure est inestimable.

## Orchestration : Airflow

Airflow est le choix naturel pour l'orchestration dans un stack vendor-neutral. Open-source, extensible, et surtout : la communauté est massive. Quand vous avez un problème avec Airflow, quelqu'un l'a déjà eu et a posté la solution sur Stack Overflow.

L'alternative serait Dagster ou Prefect, mais Airflow reste le plus déployé en production et le plus demandé sur le marché. Pragmatisme.

## IaC : Terraform pour le multi-cloud

Terraform est la pièce qui rend le vendor-neutral viable à l'échelle. L'infrastructure est décrite en code, versionnée dans Git, et déployable sur AWS, GCP ou Azure avec des changements de provider — pas de réécriture complète.

Dans ce projet, les modules Terraform provisionent l'infrastructure AWS, mais la même logique pourrait être portée sur un autre cloud sans refondre l'architecture applicative.

## Ce que j'en retiens

### Le vendor-neutral a un coût, mais le lock-in aussi

Construire vendor-neutral demande plus de travail initial. Mais le lock-in a un coût caché qui explose le jour où vous devez migrer — et ce jour arrive toujours plus tôt qu'on ne le pense.

### Les formats ouverts sont l'assurance vie de vos données

Iceberg, Parquet, Avro — tant que vos données sont dans un format ouvert, vous pouvez changer de moteur de compute sans perdre vos données. C'est la décision la plus importante d'une architecture data.

### La qualité des données est un problème organisationnel, pas technique

Les outils comme Great Expectations sont nécessaires mais pas suffisants. Sans contrats de service avec les sources, les alertes de qualité ne sont que du bruit.

### La réalité fonctionnelle prévaut sur les patterns

Bronze/silver/gold est un bon guide, pas une religion. Si votre donnée n'a besoin que de deux couches, n'en faites pas trois pour respecter un pattern. L'architecture doit servir le besoin métier, pas l'inverse.

### Le streaming ne remplace pas le batch, il le complète

Kafka pour l'ingestion temps réel, Spark pour les transformations lourdes. Les deux coexistent, et c'est sain. Vouloir tout faire en streaming est aussi dogmatique que tout faire en batch.

## Pour aller plus loin

Le code source est disponible sur [GitHub](https://github.com/HamdiMechelloukh/olist-lakehouse). Le projet utilise le dataset Olist (e-commerce brésilien) comme source de données, ce qui permet de le tester sans infrastructure lourde.
