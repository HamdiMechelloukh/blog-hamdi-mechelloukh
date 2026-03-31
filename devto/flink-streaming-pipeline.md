---
title: "Real-time streaming pipeline with Apache Flink 2.0, Kafka and Iceberg"
published: false
description: "How I built an end-to-end streaming pipeline on the Olist dataset: CEP anomaly detection, event-time watermarks, Iceberg integration and the integration issues I didn't anticipate."
tags: java, dataengineering, kafka, opensource
canonical_url: https://www.hamdimechelloukh.com/blog/flink-kafka-iceberg-streaming-pipeline
---

It's 2:03 PM. A flash sale just started.

In the warehouse, an operator is entering incoming orders into the management system. He types a quantity, makes a mistake, corrects it immediately. Two events, one reality. Thirty seconds apart.

The batch job that runs at 2 AM will see both. It won't know which one is right. Depending on how the reconciliation logic is written, if it exists at all, it picks one of the two, often non-deterministically. And if the correction falls into the next batch window, the problem doesn't surface right away: the morning's numbers are wrong, cleanly, with no technical error in sight.

This is a real and recurring source of data quality problems in data teams.

Processing events as they arrive, in order, with their temporal context intact, fundamentally changes how this problem is handled. That's the starting point for this project: an end-to-end streaming pipeline on the Olist e-commerce dataset, built with Apache Flink 2.0, Kafka and Iceberg.

## The dataset and the problem

The [Olist dataset](https://www.kaggle.com/datasets/olistbr/brazilian-ecommerce) is a public Brazilian e-commerce dataset: orders, products, sellers, customers, reviews. 100,000 orders over two years.

I had already built a batch lakehouse on this same dataset. The logical next step was to go to the other extreme: stream processing, one-minute calculation windows, anomaly detection at the second level. Three concrete needs:

1. **Revenue by category in real time** — know which category is performing at every minute
2. **Anomaly detection** — a customer placing multiple orders within a few minutes, or an order at an abnormal price
3. **Global KPIs** — average order value, order rate, total revenue in real time

These are the three jobs that make up the pipeline.

## Why Apache Flink?

The question is worth asking. There are other options for streaming in Java:

- **Kafka Streams** — easy to operate, no separate cluster, but limited to Kafka-in/Kafka-out topologies
- **Apache Spark Structured Streaming** — micro-batches, minimum latency of a few seconds, but familiar if you already know Spark
- **Flink** — true event-by-event streaming, native event-time processing, built-in CEP

Flink was the natural choice for two reasons.

**CEP (Complex Event Processing).** Detecting "3 orders from the same customer within 5 minutes" is not an aggregation, it's a temporal correlation between events. Flink CEP handles this natively with a pattern DSL. In Kafka Streams, it requires maintaining manual state and writing the temporal logic by hand.

**Flink 2.0.** Version 2.0 brought native Java 21 support. Working on the current version rather than an end-of-life one was a deliberate choice.

## The architecture

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

Three independent jobs, one shared source topic, three output topics, and an optional Iceberg data lake.

The independence of the jobs is a deliberate choice. In production, you want to be able to restart `AnomalyDetectionJob` without affecting `RevenueAggregationJob`. Each job has its own checkpoint, its own state, its own topology.

## Job 1: RevenueAggregationJob

The simplest of the three. It aggregates revenue by product category over one-minute windows.

```
orders → filter nulls → map to RevenueByCategory → keyBy(category)
       → TumblingWindow(1 min) → reduce + ProcessWindowFunction
       → Kafka sink + Iceberg sink (optional)
```

A few details that matter.

**Watermark strategy.** The pipeline uses event time, meaning the event timestamp in the Kafka message, not the arrival time. The strategy is `forBoundedOutOfOrderness(10 seconds)` with a 5-second idleness timeout.

Why idleness? If a stream is empty for several minutes (the simulator is stopped, for example), Flink can no longer advance its watermark. Without `withIdleness`, windows never close. With `withIdleness(5s)`, Flink ignores silent partitions and advances anyway.

**Side outputs.** Invalid events (null price, missing timestamp) are not silently dropped. They are routed to a side output that logs them. This avoids the scenario where events disappear without a trace.

**Two-phase reduction.** Before the window is applied, a `reduce` combines events by category on the fly. The `ProcessWindowFunction` then only attaches the window start and end timestamps. Less state to store, less work at window closure.

## Job 2: AnomalyDetectionJob

This one is more interesting. It detects two types of anomalies through two different mechanisms.

### Threshold detection: price anomaly

A filter on price:

```java
ordersStream
    .filter(event -> event.getPrice() != null
                  && event.getPrice().compareTo(priceThreshold) > 0)
    .map(event -> OrderAlert.priceAnomaly(event.getCustomerId(), event.getPrice()))
```

The threshold (500 BRL by default) is configurable via environment variable. One subtlety: the filter is `> 0`, not `>= 0`. An order at exactly 500 BRL is not an anomaly. This behavior is covered by a specific test.

### Pattern detection: suspicious frequency

This is where CEP comes in.

```java
Pattern<OrderEvent, ?> pattern = Pattern.<OrderEvent>begin("orders")
    .timesOrMore(suspiciousOrderCount)
    .within(Duration.ofMinutes(5));

PatternStream<OrderEvent> patternStream = CEP.pattern(
    ordersStream.keyBy(OrderEvent::getCustomerId),
    pattern
);
```

This pattern says: if the same customer places 3 or more orders within a 5-minute window, it's suspicious.

The key is `keyBy(customerId)`. Without it, Flink would compare orders from different customers. With `keyBy`, each customer has their own independent CEP state.

Both streams, price alerts and frequency alerts, are then merged with `union()` before being sent to the Kafka output topic.

## Job 3: RealtimeKpiJob

Global KPIs: average order value, orders per minute, total revenue. The calculation is straightforward, but the implementation reveals an interesting trade-off.

### windowAll: the acknowledged bottleneck

To calculate total revenue across all orders in one minute, you need to aggregate all events together, without splitting by key. In Flink, this is called `windowAll`.

`windowAll` forces all events through a single processing instance. It's a bottleneck by design. At this volume (50 events per second), it's more than sufficient. If throughput rose to 50,000 events per second, a pre-aggregation by key followed by a merge would be necessary. We don't do that here because adding complexity for a hypothetical need is not good engineering.

### Two-phase aggregation

The KPI calculation uses the `AggregateFunction` + `ProcessAllWindowFunction` pattern:

- `KpiAggregateFunction` accumulates the count and sum as events arrive, continuously
- `KpiWindowFunction` computes the average and derived metrics at window closure

This separation maintains minimal state (two numbers) instead of buffering all raw events. The `ProcessAllWindowFunction` only receives the final accumulator.

### The BoundedHistogram

An optional but interesting detail: a custom Flink `Histogram` implementation.

The Flink Metrics API exposes three standard types: `Counter`, `Gauge`, `Histogram`. For a `Histogram`, Flink expects an implementation that returns percentiles, mean and standard deviation via a `HistogramStatistics` interface.

The `BoundedHistogram` is a fixed-size circular buffer (1000 values). When the buffer is full, new values overwrite the oldest ones.

```java
public synchronized void update(long value) {
    values[writeIndex % values.length] = value;
    writeIndex++;
}
```

Simple, thread-safe, bounded memory. It allows Grafana to show the distribution of average order values, not just the latest single value.

## Iceberg integration: what I didn't anticipate

The Apache Iceberg integration was optional in the initial architecture. In practice, this is where I spent the most time.

### The classloader problem

Flink 2.0 loads its filesystem plugins (including `flink-s3-fs-hadoop`) in an isolated classloader, invisible to user code. When `iceberg-flink-runtime` tries to instantiate `S3AFileSystem` at write time, it can't find the class provided by the Flink plugin.

The solution: bundle `hadoop-aws` and the AWS SDK directly in the fat JAR, with aggressive exclusions to avoid dependency conflicts.

```kotlin
implementation("org.apache.hadoop:hadoop-aws:3.4.1") {
    exclude(group = "com.amazonaws", module = "aws-java-sdk-bundle")
}
implementation("com.amazonaws:aws-java-sdk-s3:1.12.780")
implementation("com.amazonaws:aws-java-sdk-sts:1.12.780")
```

The fat JAR reaches ~710 MB. Not ideal, but that's the real cost of an Iceberg + Flink + S3 integration outside a managed service.

### Credential timing

Second surprise: `HadoopCatalog` reads its S3 configuration at construction time, not after. The intuitive pattern of creating the catalog and then injecting configuration doesn't work:

```java
// Credentials are injected too late
HadoopCatalog catalog = new HadoopCatalog();
catalog.setConf(hadoopConf);
```

```java
// Credentials must be in the Configuration before construction
Configuration hadoopConf = new Configuration();
config.toProperties().forEach(hadoopConf::set);
HadoopCatalog catalog = new HadoopCatalog(hadoopConf, config.warehouse());
```

Same applies to `CatalogLoader.hadoop()`. This behavior is not prominently documented. It's the kind of error you only discover through end-to-end testing.

### Docker Compose and .env resolution

A less expected issue: Docker Compose v2 resolves the `.env` file from the directory containing `docker-compose.yml`, not from the current working directory.

```bash
# From the project root, this command ignores the .env at the root
docker compose -f docker/docker-compose.yml up -d

# You need to pass the path explicitly
docker compose --env-file .env -f docker/docker-compose.yml up -d
```

Without this, `ICEBERG_ENABLED=true` in the `.env` is ignored and jobs start without an Iceberg sink, with no error message.

## Observability

Flink exposes its metrics via Prometheus on port 9249. Each job exposes custom metrics:

| Metric | Type | Job |
|--------|------|-----|
| `windowsEmitted` | Counter | RevenueAggregationJob |
| `kpiWindowsEmitted` | Counter | RealtimeKpiJob |
| `lastWindowOrderCount` | Gauge | RealtimeKpiJob |
| `orderValueDistribution` | Histogram | RealtimeKpiJob |
| `priceAnomalyAlertsEmitted` | Counter | AnomalyDetectionJob |
| `suspiciousFrequencyAlertsEmitted` | Counter | AnomalyDetectionJob |
| `deserializationErrors` | Counter | All |

These metrics land in Prometheus every 15 seconds and are visualized in Grafana. The `deserializationErrors` metric is particularly useful: if the simulator sends a malformed message, the counter rises and you see it immediately in the dashboard, without the job crashing.

## Testing

The tests use Flink's `MiniCluster`, an embedded Flink cluster that runs in the test process, with no external infrastructure.

This choice has a cost: tests are slower (a few seconds each). But they test the actual behavior of Flink operators, not a mock. The `AnomalyDetectionJobTest` specifically validates CEP edge cases:

- 2 orders in 5 minutes → no alert
- 3 orders in 5 minutes → alert triggered
- Order at exactly 500 BRL → no price alert

18 tests in total, covering all three jobs, the `BoundedHistogram` and the deserialization schema. The CI (GitHub Actions) compiles and runs all tests on every push, with a JaCoCo report as an artifact.

## Batch or streaming: the real debate

Back to the opening scene.

Streaming is often perceived as expensive: the cluster runs continuously, the infrastructure never shuts down. That's true. But this comparison is incomplete.

A batch pipeline that handles events which correct themselves over time accumulates its own debt. Timeline reconciliation logic. Re-processing when an event arrives late. Alerts, manual interventions, data engineers spending time explaining why numbers are inconsistent across two windows. This cost is diffuse: it doesn't appear on any cloud bill, but it accumulates in sprints, in support, in technical debt.

**Nobody actually does this calculation in practice — because it's too costly to conduct seriously.**

This project doesn't claim to settle the debate. What it shows is that an end-to-end streaming pipeline with Flink 2.0 is accessible today without managed infrastructure, without Databricks, without Confluent Cloud. A `docker compose up` and the pipeline runs. The complexity is in the integration details, not in the paradigm itself.

The code is on [GitHub](https://github.com/HamdiMechelloukh/olist-flink-streaming). The `start-e2e.sh` script launches the entire pipeline in a single command.

---

You can also read this and other articles on [my portfolio](https://www.hamdimechelloukh.com).
