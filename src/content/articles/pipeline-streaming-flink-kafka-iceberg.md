Il est 14h03. Une vente flash vient de démarrer.

Dans l'entrepôt, un opérateur encode les commandes entrantes dans le système de gestion. Il saisit une quantité, se trompe, corrige dans la foulée. Deux événements, une seule réalité. Trente secondes d'écart.

Le batch qui tourne à 2h du matin verra les deux. Il ne saura pas lequel est bon. Selon comment la logique de réconciliation est écrite, si elle existe, il en prend un des deux, souvent de façon non déterministe. Et si la correction arrive dans la fenêtre suivante, le problème ne se voit même pas tout de suite : les chiffres du matin sont faux, proprement, sans aucune erreur technique.

C'est une source réelle et récurrente de problèmes de qualité dans les équipes data.

Traiter les événements au fil de l'eau, dans l'ordre où ils arrivent, avec leur temporalité intacte, change structurellement la façon dont on gère ce problème. C'est le point de départ de ce projet : un pipeline streaming end-to-end sur le dataset e-commerce Olist, avec Apache Flink 2.0, Kafka et Iceberg.

## Le dataset et le problème

Le [dataset Olist](https://www.kaggle.com/datasets/olistbr/brazilian-ecommerce) est un dataset public de e-commerce brésilien : commandes, produits, vendeurs, clients, avis. 100 000 commandes sur deux ans.

J'avais déjà construit un lakehouse batch sur ce même dataset. L'étape logique était de passer à l'autre extrémité : traitement au fil de l'eau, fenêtres de calcul à la minute, détection d'anomalies à la seconde. Trois besoins concrets :

1. **Revenus par catégorie en direct** — savoir quelle catégorie performe à chaque minute
2. **Détection d'anomalies** — un client qui passe plusieurs commandes en quelques minutes, ou une commande à un prix anormal
3. **KPIs globaux** — valeur moyenne des commandes, cadence des ventes, revenus totaux en continu

Ce sont les trois jobs qui composent le pipeline.

## Pourquoi Apache Flink ?

La question mérite d'être posée. Il existe d'autres options pour du streaming en Java :

- **Kafka Streams** — simple à opérer, pas de cluster séparé, mais limité aux topologies Kafka-in/Kafka-out
- **Apache Spark Structured Streaming** — micro-batches, latence de quelques secondes minimum, mais familier si on connaît déjà Spark
- **Flink** — vrai streaming événement par événement, gestion native du temps événementiel, CEP intégré

Le choix de Flink s'est imposé pour deux raisons.

**Le CEP (Complex Event Processing).** Détecter "3 commandes du même client en moins de 5 minutes" n'est pas une agrégation. C'est une corrélation temporelle entre événements. Flink CEP gère ça nativement avec un DSL de patterns. Dans Kafka Streams, ça nécessite de maintenir un état manuel et d'écrire la logique temporelle à la main.

**Flink 2.0.** La version 2.0 venait de sortir avec un support natif Java 21. C'était l'occasion de travailler sur la version courante plutôt qu'une version en fin de vie.

## L'architecture

```
Olist CSV → Simulator → Kafka (orders)
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
    RevenueAggregation  AnomalyDetection  RealtimeKpi
    (tumbling 1 min)    (CEP 5 min)       (windowAll 1 min)
              │               │               │
              ▼               ▼               ▼
      Kafka (revenue)  Kafka (alerts)  Kafka (kpis)
              │               │               │
              └───────────────┴───────────────┘
                              │
                    Apache Iceberg (MinIO)
```

Trois jobs indépendants, un topic source commun, trois topics de sortie, et un data lake Iceberg en option.

L'indépendance des jobs est un choix délibéré. En production, on veut pouvoir redémarrer `AnomalyDetectionJob` sans impacter `RevenueAggregationJob`. Chaque job a son propre checkpoint, son propre état, sa propre topologie.

## Job 1 : RevenueAggregationJob

Le plus simple des trois. Il agrège les revenus par catégorie de produit sur des fenêtres d'une minute.

```
orders → filter nulls → map to RevenueByCategory → keyBy(category)
       → TumblingWindow(1 min) → reduce + ProcessWindowFunction
       → Kafka sink + Iceberg sink (optionnel)
```

Quelques détails qui comptent.

**Watermark strategy.** Le pipeline utilise le temps événementiel : la date de l'événement dans le message Kafka, pas l'heure d'arrivée. La stratégie est `forBoundedOutOfOrderness(10 secondes)` avec un délai d'inactivité de 5 secondes.

Pourquoi l'inactivité ? Si un flux est vide pendant plusieurs minutes (le simulateur est arrêté, par exemple), Flink ne peut plus faire avancer son watermark. Sans `withIdleness`, les fenêtres ne se ferment jamais. Avec `withIdleness(5s)`, Flink ignore les partitions silencieuses et avance quand même.

**Side outputs.** Les événements invalides (prix null, timestamp absent) ne sont pas silencieusement ignorés. Ils sont routés vers un side output qui les loggue. Ça évite le scénario où des événements disparaissent sans trace.

**Réduction en deux phases.** Avant l'application de la fenêtre, un `reduce` combine les événements par catégorie au fil de l'eau. La `ProcessWindowFunction` n'attache ensuite que les horodatages de début et fin de fenêtre. Moins d'état à stocker, moins de travail à la fermeture de la fenêtre.

## Job 2 : AnomalyDetectionJob

Celui-ci est plus intéressant. Il détecte deux types d'anomalies via deux mécanismes différents.

### Détection par seuil : price anomaly

Un filtre sur le prix :

```java
ordersStream
    .filter(event -> event.getPrice() != null
                  && event.getPrice().compareTo(priceThreshold) > 0)
    .map(event -> OrderAlert.priceAnomaly(event.getCustomerId(), event.getPrice()))
```

Le seuil (500 BRL par défaut) est configurable via variable d'environnement. Une subtilité : le filtre est `> 0`, pas `>= 0`. Une commande exactement à 500 BRL n'est pas une anomalie. Ce comportement est couvert par un test spécifique.

### Détection par pattern : suspicious frequency

C'est ici que le CEP entre en jeu.

```java
Pattern<OrderEvent, ?> pattern = Pattern.<OrderEvent>begin("orders")
    .timesOrMore(suspiciousOrderCount)
    .within(Duration.ofMinutes(5));

PatternStream<OrderEvent> patternStream = CEP.pattern(
    ordersStream.keyBy(OrderEvent::getCustomerId),
    pattern
);
```

Ce pattern dit : si un même client passe 3 commandes ou plus dans une fenêtre de 5 minutes, c'est suspect.

La clé est le `keyBy(customerId)`. Sans ça, Flink comparerait des commandes de clients différents. Avec `keyBy`, chaque client a son propre état CEP indépendant.

Les deux flux (alertes de prix et alertes de fréquence) sont ensuite fusionnés avec `union()` avant d'être envoyés vers le topic Kafka de sortie.

## Job 3 : RealtimeKpiJob

Les KPIs globaux : valeur moyenne des commandes, nombre de commandes par minute, revenu total. Le calcul est simple, mais la mise en œuvre révèle un compromis intéressant.

### windowAll : le goulot d'étranglement assumé

Pour calculer le revenu total de toutes les commandes sur une minute, il faut agréger tous les événements ensemble, sans découper par clé. En Flink, ça s'appelle `windowAll`.

`windowAll` force tous les événements vers une seule instance de traitement. C'est un goulot d'étranglement par conception. À ce volume (50 événements par seconde), c'est largement suffisant. Si le débit montait à 50 000 événements par seconde, une pré-agrégation par clé puis une fusion serait nécessaire. Nous ne faisons pas ça ici car complexifier le code pour un besoin hypothétique n'est pas une bonne ingénierie.

### L'agrégation en deux phases

Le calcul des KPIs utilise le pattern `AggregateFunction` + `ProcessAllWindowFunction` :

- `KpiAggregateFunction` accumule le compte et la somme au fil des événements, en continu
- `KpiWindowFunction` calcule la moyenne et les métriques dérivées à la fermeture de la fenêtre

Cette séparation maintient un état minimal (deux nombres) au lieu de bufferiser tous les événements bruts. La `ProcessAllWindowFunction` ne reçoit que l'accumulateur final.

### Le BoundedHistogram

Un détail non obligatoire mais intéressant : l'implémentation d'un `Histogram` Flink custom.

L'API Flink Metrics expose trois types standards : `Counter`, `Gauge`, `Histogram`. Pour un `Histogram`, Flink s'attend à une implémentation qui retourne percentiles, moyenne et écart-type via une interface `HistogramStatistics`.

Le `BoundedHistogram` est un buffer circulaire de taille fixe (1000 valeurs). Quand le buffer est plein, les nouvelles valeurs écrasent les plus anciennes.

```java
public synchronized void update(long value) {
    values[writeIndex % values.length] = value;
    writeIndex++;
}
```

Simple, thread-safe, mémoire bornée. Ça permet d'avoir dans Grafana la distribution des valeurs moyennes de commandes et pas juste la dernière valeur ponctuelle.

## L'intégration Iceberg : ce que je n'avais pas anticipé

L'intégration Apache Iceberg était optionnelle dans l'architecture initiale. En pratique, c'est là que j'ai passé le plus de temps.

### Le problème de classloader

Flink 2.0 charge ses plugins filesystem (dont `flink-s3-fs-hadoop`) dans un classloader isolé, invisible au code utilisateur. Quand `iceberg-flink-runtime` essaie d'instancier `S3AFileSystem` au moment de l'écriture, il ne trouve pas la classe fournie par le plugin Flink.

La solution : bundler `hadoop-aws` et l'AWS SDK directement dans le fat JAR, avec des exclusions agressives pour éviter les conflits de dépendances.

```kotlin
implementation("org.apache.hadoop:hadoop-aws:3.4.1") {
    exclude(group = "com.amazonaws", module = "aws-java-sdk-bundle")
}
implementation("com.amazonaws:aws-java-sdk-s3:1.12.780")
implementation("com.amazonaws:aws-java-sdk-sts:1.12.780")
```

Le fat JAR monte à ~710 MB. Ce n'est pas idéal, mais c'est le coût réel d'une intégration Iceberg + Flink + S3 en dehors d'un service managé.

### Le timing des credentials

Deuxième surprise : le `HadoopCatalog` d'Iceberg lit sa configuration S3 au moment de la construction, pas après. Le pattern intuitif qui consiste à créer le catalog puis à injecter la configuration ne fonctionne pas :

```java
// Les credentials sont injectés trop tard
HadoopCatalog catalog = new HadoopCatalog();
catalog.setConf(hadoopConf);
```

```java
// Les credentials doivent être dans la Configuration avant la construction
Configuration hadoopConf = new Configuration();
config.toProperties().forEach(hadoopConf::set);
HadoopCatalog catalog = new HadoopCatalog(hadoopConf, config.warehouse());
```

Même chose pour `CatalogLoader.hadoop()`. Ce comportement n'est pas documenté de façon prominente. C'est le type d'erreur qu'on découvre uniquement en testant end-to-end.

### Docker Compose et la résolution du .env

Un bug moins attendu : Docker Compose v2 résout le fichier `.env` depuis le répertoire contenant le `docker-compose.yml`, pas depuis le répertoire courant.

```bash
# Depuis la racine du projet, cette commande ignore le .env à la racine
docker compose -f docker/docker-compose.yml up -d

# Il faut passer explicitement le chemin
docker compose --env-file .env -f docker/docker-compose.yml up -d
```

Sans ça, `ICEBERG_ENABLED=true` dans le `.env` est ignoré et les jobs démarrent sans sink Iceberg, sans aucun message d'erreur.

## L'observabilité

Flink expose ses métriques via Prometheus sur le port 9249. Chaque job expose des métriques custom :

| Métrique | Type | Job |
|----------|------|-----|
| `windowsEmitted` | Counter | RevenueAggregationJob |
| `kpiWindowsEmitted` | Counter | RealtimeKpiJob |
| `lastWindowOrderCount` | Gauge | RealtimeKpiJob |
| `orderValueDistribution` | Histogram | RealtimeKpiJob |
| `priceAnomalyAlertsEmitted` | Counter | AnomalyDetectionJob |
| `suspiciousFrequencyAlertsEmitted` | Counter | AnomalyDetectionJob |
| `deserializationErrors` | Counter | Tous |

Ces métriques arrivent dans Prometheus toutes les 15 secondes et sont visualisées dans Grafana. La métrique `deserializationErrors` est particulièrement utile : si le simulateur envoie un message malformé, le compteur monte et on le voit immédiatement dans le dashboard, sans que le job ne plante.

## Les tests

Les tests utilisent le `MiniCluster` de Flink, un cluster Flink embarqué qui tourne dans le process de test, sans infrastructure externe.

Ce choix a un coût : les tests sont plus lents (quelques secondes chacun). Mais ils testent le comportement réel des opérateurs Flink, pas un mock. L'`AnomalyDetectionJobTest` vérifie notamment les cas limites du CEP :

- 2 commandes en 5 minutes → pas d'alerte
- 3 commandes en 5 minutes → alerte déclenchée
- Commande exactement à 500 BRL → pas d'alerte de prix

18 tests au total, couvrant les trois jobs, le `BoundedHistogram` et le schéma de désérialisation. Le CI (GitHub Actions) compile et exécute tous les tests à chaque push, avec un rapport JaCoCo en artefact.

## Batch ou streaming : le vrai débat

Revenons à la scène du début.

Le streaming est souvent perçu comme cher : le cluster tourne en permanence, l'infrastructure ne s'éteint jamais. C'est vrai. Mais cette comparaison est incomplète.

Un pipeline batch qui gère des événements qui se corrigent dans le temps accumule sa propre dette. De la logique de réconciliation des timelines. Des re-traitements quand un événement arrive en retard. Des alertes, des interventions manuelles, des data engineers qui passent du temps à expliquer pourquoi les chiffres ne sont pas cohérents entre deux fenêtres. Ce coût est diffus. Il ne s'affiche sur aucune facture cloud, mais il s'accumule dans les sprints, dans le support, dans la dette technique.

**Personne ne fait vraiment ce calcul en entreprise — parce qu'il est lui-même trop coûteux à conduire.**

Ce projet ne prétend pas trancher. Ce qu'il montre, c'est qu'un pipeline streaming end-to-end avec Flink 2.0 est aujourd'hui accessible sans infrastructure managée, sans Databricks, sans Confluent Cloud. Un `docker compose up` et le pipeline tourne. La complexité est dans les détails d'intégration, pas dans le paradigme lui-même.

Le code est sur [GitHub](https://github.com/HamdiMechelloukh/olist-flink-streaming). Le `start-e2e.sh` lance l'ensemble du pipeline en une commande.
