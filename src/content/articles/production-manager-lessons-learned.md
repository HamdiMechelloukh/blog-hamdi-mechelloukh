For two and a half years, I stepped away from code to manage data production for sales at Decathlon Digital. A role I discovered upon arrival — the job title said "Production Expert", and I quickly realized it was going to be a full-time commitment.

Here's what I learned from switching to the other side.

## Context: Perfeco and sales data

Perfeco was the data product that served the company's economic performance and sales data. In practice, it meant:

- An ingestion pipeline built on **Talend** and **Redshift** — data was processed and stored in Redshift, then pushed to S3
- **2 to 3 million sales per day** ingested
- Data exposed in the datalake and via an API consumed by multiple business teams
- Kafka messages with XML payloads converted to CSV before loading
- A scheduler (OpCon) to orchestrate ingestion jobs

My role: **make sure all of this runs**, every day, without interruption.

## What "Production Manager" actually means day-to-day

Coming from development, you'd think production is about monitoring and a few alerts. Reality is very different.

### Reducing the operational burden

My main goal wasn't to react to incidents, but to **reduce their frequency**. That meant:

- **Proactive alerting** — setting up the right dashboards (QuickSight, Tableau) and alerts to detect anomalies before they become incidents. Automatic Jira ticket creation when a threshold is breached.
- **Data quality at the source** — analyzing and detecting quality issues upstream, then escalating them to source teams. This is facilitation work, not code: convincing an upstream team that their data is poorly formatted takes time and diplomacy.
- **Run documentation** — writing and maintaining on-call procedures so that any team member can intervene at 3 AM without relying on one person's memory.
- **Run KPIs** — scripting metrics collection to objectively measure stability: incident count, resolution time, data availability.

### Facilitating, not coding

The biggest surprise was the **relational dimension** of the role. I spent more time managing people than technology:

- **Facilitating consumer teams** — improving incident communication. When an ingestion is delayed, 5 different teams need to know why and when it will be resolved. You need a clear channel, a clear message, and consistency.
- **Facilitating source teams** — working with teams that produce upstream data so they fix quality issues at the root.
- **On-call planning** — organizing rotations for the team, making sure everyone is trained and the load is fairly distributed.
- **Postmortems** — I organized regular meetings with both data sources and consumers. Postmortems were filled collaboratively during these sessions: what happened, why, and what actions to take to prevent recurrence. This collaborative format aligned everyone and avoided the blame game.

## The typical incident: when the scheduler crashes

My nemesis during those two years was the OpCon scheduler client crashing on the machine. Silently.

The scenario was always the same:

1. The OpCon client crashes → no jobs are launched
2. Sales keep arriving via Kafka (messages with XML payloads)
3. Messages pile up — hundreds of thousands within hours
4. When we restart the scheduler, the XML → CSV conversion job faces a massive backlog
5. The Talend job struggles, processing times explode, Redshift is overwhelmed, data arrives late in S3

The biggest incidents we had were all tied to this problem. What made it frustrating was that the client crash was silent — no alert, no explicit log. We'd only discover it by noticing the absence of data downstream.

The lesson: **monitoring the absence of events is as important as monitoring errors**. If a job that runs every 15 minutes hasn't executed in 30 minutes, that's a strong signal.

## What I learned

### Production is engineering

Reducing operational burden isn't just "adding alerts". It's designing an observability system, automating detection, documenting procedures, and measuring improvement. It's engineering work in its own right.

### Communication is a technical skill

Writing a clear incident message, running a blameless postmortem, convincing a source team to fix a data format — these are skills as important as writing code. And they can be practiced.

### Proactive alerting changes everything

The difference between a PM who reacts and one who manages is proactivity. When you discover an incident from an automatic alert at 8 AM instead of a call from a business team at 10 AM, you've gained 2 hours and a lot of peace of mind.

### Monitor the silence

The most dangerous incidents don't generate errors — they generate silence. A pipeline that stops running, a scheduler that has crashed, a message that never arrives. Alerts on the absence of activity saved me more often than alerts on errors.

### Documentation is not optional

In dev, you can sometimes get by with readable code and a few comments. In production, if the on-call procedure isn't written down, it doesn't exist. The person on call at 3 AM doesn't have time to guess.

## Why I went back to technical work

After two and a half years, I decided to return to a Data Engineer role. The reason is simple: **I felt I was regressing technically**.

The PM day-to-day is fascinating — the diversity of problems, the human dimension, the direct impact on data reliability. But I was spending my days facilitating, documenting and communicating, and less and less designing and coding.

I was afraid of falling behind, of no longer being up to speed on fast-evolving technologies — Spark, Databricks, lakehouse architectures. The risk of becoming a purely managerial profile without technical expertise didn't sit well with me.

Today, looking back, I don't regret the experience. It gave me an understanding of production that many developers don't have. When I design a pipeline now, I naturally think about observability, error recovery, and operational documentation. These are reflexes that code alone wouldn't have given me.

## In summary

If you're a developer and someone offers you a production-oriented role, here's what I'd tell you:

- **It's a real job**, not a support role. It requires engineering, rigor, and a lot of soft skills.
- **You'll learn things that development will never teach you** — crisis communication, priority management under pressure, the end-to-end view of a data product.
- **Set a time limit**. It's enriching, but if your core expertise is technical, don't stay too long or you risk falling behind.
- **Bring those reflexes back into your code**. Observability, documentation, monitoring the silence — these are skills that make better engineers.
