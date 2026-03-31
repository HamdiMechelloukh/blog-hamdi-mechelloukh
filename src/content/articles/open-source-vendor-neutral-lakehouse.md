When you work in data, you always end up asking the same question: **what happens if we need to switch platforms tomorrow?**

I've seen firsthand that software vendors can be aggressive with pricing, and they won't hesitate to sunset a product that isn't generating enough revenue. When that happens, you need to migrate quickly, or face massive costs in migration, redevelopment, and lost time.

This conviction led me to build **an end-to-end open-source, vendor-neutral lakehouse**, from messaging to visualization. Here are the architecture choices, the trade-offs, and what I learned.

## The stack: Kafka → Spark → Iceberg

```
Sources → Kafka → Spark (Bronze) → Spark (Silver) → Spark (Gold) → Streamlit
                                                                   ↑
                                                          Great Expectations
                                                          (quality at each layer)
```

### Kafka for ingestion

Choosing event-driven for data transfer isn't trivial. In computing, **managing time is one of the hardest problems**. On the operational side, we're moving more and more toward event-driven architectures precisely for this reason: an event arrives when it arrives, and the system processes it. No batch window to respect, no "the file should have arrived at 6 AM".

Kafka is the de facto standard for this kind of architecture. Open-source, battle-tested, and crucially: no vendor lock-in. You can deploy it on any cloud or on-premise.

### Spark for compute

You might ask: why Spark in an event-driven architecture? My position is pragmatic. Pure streaming via Kafka works well for ingestion into bronze, or even silver, to **handle temporality upstream**. But once you reach heavy transformations (aggregations, joins, enrichments), Spark remains the most battle-tested and portable tool.

Spark's advantage is that it runs everywhere: on a YARN cluster, on Kubernetes, on Databricks, on EMR, or locally for development. It's one of the few compute tools that doesn't lock you in.

### Iceberg for the table format

Iceberg is the open table format that's gaining momentum. My choice was partly technical curiosity: I use Delta Lake daily at work, so I wanted to explore the alternative.

But beyond curiosity, Iceberg has properties that make it particularly suited for a vendor-neutral lakehouse:

- **Open format** — no dependency on a specific vendor
- **Time travel** — query data at any point in time
- **Schema evolution** — add or modify columns without rewriting data
- **Partition evolution** — change partitioning scheme without migration
- **Compatible with all engines** — Spark, Trino, Flink, Dremio, Athena...

The project could just as well run with Delta Lake or Hudi. In fact, it would be interesting to offer format choice to anyone forking the project.

## The layered architecture: bronze, silver, gold

The medallion pattern (bronze/silver/gold) structures data in three levels of refinement:

- **Bronze** — raw data as it arrives, no transformation
- **Silver** — cleaned, deduplicated, properly typed data
- **Gold** — aggregated data ready for business consumption

Honestly, these terms are recent. A few years ago, we called them dataraw, dataprep, dataset. The vocabulary changes, the principle stays the same. What matters is to **follow this progressive refinement logic without being rigid.** Functional reality always takes precedence over technical rules. If data doesn't need three layers, it doesn't need three layers.

## MinIO: S3-compatible without the lock-in

One point that might surprise you: why MinIO rather than S3 directly?

Because **S3 is an AWS service**, and using S3 means locking yourself into AWS. MinIO implements the S3 API identically: every tool that speaks S3 speaks MinIO without modification. You can develop and test locally, deploy on any cloud, and migrate to S3, GCS or Azure Blob Storage without changing a single line of application code.

That's exactly the vendor-neutral principle: **use open standards rather than proprietary managed services**.

## Data quality: Great Expectations and its limits

Great Expectations is the most widely used data validation tool in the Python/Spark ecosystem. I integrated it at each pipeline layer to validate data on input and output.

The tool does its job well for simple quality rules: nullability, uniqueness, value ranges, formats. It's also a tool I've seen used in enterprise settings, which validated the choice.

But it has **real limitations**:

- Complex quality rules (cross-table consistency, conditional business rules) are hard to express
- Resource-intensive checks (massive joins for cross-source duplicate detection) don't scale easily
- And most importantly: **discovering quality issues is not enough**

This last point is crucial and comes directly from my production experience at Decathlon. You can set up all the quality alerts in the world. If source teams have no commitment to fix the issues, nothing will change. You need to work on **data quality service-level agreements**: SLAs on fix turnaround, shared responsibilities, clear escalation paths. Without that, source teams will make little effort to resolve quality problems.

## The difficulty of vendor-neutral

The biggest challenge of this project wasn't technical in the traditional sense. It was **resisting the temptation of managed services**.

At every step, there's a managed option that saves time:

- Why manage your own Kafka when there's Amazon MSK or Confluent Cloud?
- Why MinIO when S3 is there, configured in 2 clicks?
- Why self-hosted Airflow when there's MWAA?

The answer is always the same: **because the day the pricing changes or the service is deprecated, you need to be able to leave**. This doesn't mean you should never use managed services. It means you should do it knowingly, and make sure the abstraction layer allows switching.

In practice, building vendor-neutral requires more upfront effort:

- Terraform for declarative, multi-cloud infrastructure management
- Docker for isolation and portability
- Standard interfaces everywhere (S3 API, JDBC, etc.)

But once it's in place, the freedom it provides is invaluable.

## Orchestration: Airflow

Airflow is the natural choice for orchestration in a vendor-neutral stack. Open-source, extensible, and above all: the community is massive. When you have an Airflow problem, someone has already had it and posted the solution on Stack Overflow.

Alternatives would be Dagster or Prefect, but Airflow remains the most widely deployed in production and the most in-demand on the market. Pragmatism.

## IaC: Terraform for multi-cloud

Terraform is the piece that makes vendor-neutral viable at scale. Infrastructure is described in code, versioned in Git, and deployable on AWS, GCP or Azure with provider changes, no complete rewrite needed.

In this project, Terraform modules provision AWS infrastructure, but the same logic could be ported to another cloud without rebuilding the application architecture.

## What I took away

### Vendor-neutral has a cost, but so does lock-in

Building vendor-neutral requires more upfront work. But lock-in has a hidden cost that explodes the day you need to migrate. And that day always comes sooner than you think.

### Open formats are your data's life insurance

Iceberg, Parquet, Avro: as long as your data is in an open format, you can switch compute engines without losing your data. It's the most important decision in a data architecture.

### Data quality is an organizational problem, not a technical one

Tools like Great Expectations are necessary but not sufficient. Without service-level agreements with sources, quality alerts are just noise.

### Functional reality takes precedence over patterns

Bronze/silver/gold is a good guide, not a religion. If your data only needs two layers, don't make three to respect a pattern. Architecture should serve the business need, not the other way around.

### Streaming doesn't replace batch, it complements it

Kafka for real-time ingestion, Spark for heavy transformations. The two coexist, and that's healthy. Trying to do everything in streaming is as dogmatic as doing everything in batch.

## Going further

The source code is available on [GitHub](https://github.com/HamdiMechelloukh/olist-lakehouse). The project uses the Olist dataset (Brazilian e-commerce) as a data source, making it testable without heavy infrastructure.
