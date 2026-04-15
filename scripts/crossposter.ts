/**
 * Crossposter — publie les articles EN du blog vers dev.to et LinkedIn,
 * et envoie une notification Telegram pour importer manuellement sur Medium.
 *
 * Usage :
 *   npm run crosspost              → poste le prochain item en queue
 *   npm run crosspost:dry          → affiche ce qui serait posté sans rien envoyer
 *   npm run crosspost -- --mark-done <platform> <slug>
 *                                  → marque une plateforme comme déjà traitée
 *
 * Environnement requis (selon plateforme) :
 *   DEVTO_API_KEY
 *   LINKEDIN_ACCESS_TOKEN, LINKEDIN_USER_URN
 *   TELEGRAM_BOT_TOKEN, TELEGRAM_CHAT_ID
 */

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { blogArticles } from "../src/data";
import type { BlogArticle } from "../src/types";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(SCRIPT_DIR, "..");
const STATE_FILE = join(REPO_ROOT, ".crossposter-state.json");
const ARTICLES_DIR = join(REPO_ROOT, "src/content/articles");
const BASE_URL = process.env.BLOG_BASE_URL ?? "https://hamdimechelloukh.com";

type Platform = "devto" | "linkedin" | "medium";
type PostRecord = { postedAt: string; url?: string; id?: string };
type State = Record<string, Partial<Record<Platform, PostRecord>>>;

const PLATFORMS: Platform[] = ["devto", "linkedin", "medium"];

function loadState(): State {
  if (!existsSync(STATE_FILE)) return {};
  return JSON.parse(readFileSync(STATE_FILE, "utf-8")) as State;
}

function saveState(state: State): void {
  writeFileSync(STATE_FILE, JSON.stringify(state, null, 2) + "\n");
}

function canonicalUrlFor(slug: string): string {
  return `${BASE_URL}/blog/${slug}`;
}

function loadArticleBody(slug: string): string {
  return readFileSync(join(ARTICLES_DIR, `${slug}.md`), "utf-8");
}

function normalizeTag(tag: string): string {
  return tag.toLowerCase().replace(/[^a-z0-9]/g, "");
}

/** Files queue = couples (slug, platform) à traiter, articles EN triés par date croissante. */
function buildQueue(state: State): Array<{ slug: string; platform: Platform }> {
  const enArticles = blogArticles
    .filter((article) => article.lang === "en")
    .sort((a, b) => a.date.localeCompare(b.date));
  const queue: Array<{ slug: string; platform: Platform }> = [];
  for (const article of enArticles) {
    for (const platform of PLATFORMS) {
      if (!state[article.slug]?.[platform]) {
        queue.push({ slug: article.slug, platform });
      }
    }
  }
  return queue;
}

async function postToDevto(article: BlogArticle, body: string): Promise<PostRecord> {
  const apiKey = requireEnv("DEVTO_API_KEY");
  const response = await fetch("https://dev.to/api/articles", {
    method: "POST",
    headers: {
      "api-key": apiKey,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      article: {
        title: article.title,
        body_markdown: body,
        published: true,
        canonical_url: canonicalUrlFor(article.slug),
        tags: article.tags.slice(0, 4).map(normalizeTag).filter(Boolean),
        description: article.summary,
      },
    }),
  });
  if (!response.ok) {
    throw new Error(`dev.to ${response.status}: ${await response.text()}`);
  }
  const data = (await response.json()) as { id: number; url: string };
  return {
    postedAt: new Date().toISOString(),
    url: data.url,
    id: String(data.id),
  };
}

async function postToLinkedIn(article: BlogArticle): Promise<PostRecord> {
  const accessToken = requireEnv("LINKEDIN_ACCESS_TOKEN");
  const authorUrn = requireEnv("LINKEDIN_USER_URN");
  const url = canonicalUrlFor(article.slug);
  const hashtags = article.tags
    .slice(0, 5)
    .map((t) => `#${t.replace(/[^A-Za-z0-9]/g, "")}`)
    .filter((t) => t.length > 1)
    .join(" ");
  const text = `${article.title}\n\n${article.summary}\n\n🔗 ${url}${hashtags ? `\n\n${hashtags}` : ""}`;

  const response = await fetch("https://api.linkedin.com/v2/ugcPosts", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
      "X-Restli-Protocol-Version": "2.0.0",
    },
    body: JSON.stringify({
      author: authorUrn,
      lifecycleState: "PUBLISHED",
      specificContent: {
        "com.linkedin.ugc.ShareContent": {
          shareCommentary: { text },
          shareMediaCategory: "ARTICLE",
          media: [
            {
              status: "READY",
              originalUrl: url,
              title: { text: article.title },
              description: { text: article.summary },
            },
          ],
        },
      },
      visibility: { "com.linkedin.ugc.MemberNetworkVisibility": "PUBLIC" },
    }),
  });
  if (!response.ok) {
    throw new Error(`LinkedIn ${response.status}: ${await response.text()}`);
  }
  const id = response.headers.get("x-restli-id") ?? undefined;
  return { postedAt: new Date().toISOString(), id };
}

async function notifyMediumViaTelegram(article: BlogArticle): Promise<PostRecord> {
  const botToken = requireEnv("TELEGRAM_BOT_TOKEN");
  const chatId = requireEnv("TELEGRAM_CHAT_ID");
  const url = canonicalUrlFor(article.slug);
  const tagList = article.tags.slice(0, 5).join(", ");
  const markDone = `npm run crosspost -- --mark-done medium ${article.slug}`;
  const text =
    `📝 <b>Medium import à faire</b>\n\n` +
    `<b>${escapeHtml(article.title)}</b>\n\n` +
    `${escapeHtml(article.summary)}\n\n` +
    `1. <a href="https://medium.com/p/import">medium.com/p/import</a>\n` +
    `2. Colle : <code>${escapeHtml(url)}</code>\n` +
    `3. Tags : ${escapeHtml(tagList)}\n\n` +
    `Quand c'est fait :\n<code>${escapeHtml(markDone)}</code>`;

  const response = await fetch(`https://api.telegram.org/bot${botToken}/sendMessage`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      chat_id: chatId,
      text,
      parse_mode: "HTML",
    }),
  });
  if (!response.ok) {
    throw new Error(`Telegram ${response.status}: ${await response.text()}`);
  }
  return { postedAt: new Date().toISOString() };
}

function escapeHtml(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`Variable d'environnement manquante : ${name}`);
  return value;
}

function parseArgs(argv: string[]): {
  dryRun: boolean;
  markDone?: { platform: Platform; slug: string };
  listQueue: boolean;
} {
  const dryRun = argv.includes("--dry-run");
  const listQueue = argv.includes("--list");
  const markIdx = argv.indexOf("--mark-done");
  let markDone: { platform: Platform; slug: string } | undefined;
  if (markIdx !== -1) {
    const platform = argv[markIdx + 1] as Platform;
    const slug = argv[markIdx + 2];
    if (!PLATFORMS.includes(platform) || !slug) {
      throw new Error("Usage : --mark-done <devto|linkedin|medium> <slug>");
    }
    markDone = { platform, slug };
  }
  return { dryRun, markDone, listQueue };
}

async function main(): Promise<void> {
  const { dryRun, markDone, listQueue } = parseArgs(process.argv.slice(2));
  const state = loadState();

  if (markDone) {
    const entry = state[markDone.slug] ?? {};
    entry[markDone.platform] = { postedAt: new Date().toISOString() };
    state[markDone.slug] = entry;
    saveState(state);
    console.log(`✓ ${markDone.slug}/${markDone.platform} marqué comme fait`);
    return;
  }

  const queue = buildQueue(state);
  if (listQueue) {
    console.log(`Queue : ${queue.length} item(s)`);
    for (const item of queue) {
      console.log(`  - ${item.platform} / ${item.slug}`);
    }
    return;
  }
  if (queue.length === 0) {
    console.log("Queue vide — rien à publier.");
    return;
  }

  const next = queue[0];
  const article = blogArticles.find((a) => a.slug === next.slug);
  if (!article) throw new Error(`Article introuvable : ${next.slug}`);
  console.log(`→ Prochain : ${next.platform} / ${next.slug}`);

  if (dryRun) {
    console.log("[dry-run] pas d'envoi effectif.");
    if (next.platform !== "medium") console.log(`  URL : ${canonicalUrlFor(next.slug)}`);
    return;
  }

  let result: PostRecord;
  if (next.platform === "devto") {
    result = await postToDevto(article, loadArticleBody(next.slug));
  } else if (next.platform === "linkedin") {
    result = await postToLinkedIn(article);
  } else {
    result = await notifyMediumViaTelegram(article);
  }

  const entry = state[next.slug] ?? {};
  entry[next.platform] = result;
  state[next.slug] = entry;
  saveState(state);
  console.log(`✓ ${next.platform}/${next.slug} — ${result.url ?? result.id ?? "ok"}`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
});
