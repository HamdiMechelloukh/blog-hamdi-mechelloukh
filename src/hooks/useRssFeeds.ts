import { useState, useEffect } from 'react';
import { RssFeed, RssArticle } from '../types';

const RSS2JSON = 'https://api.rss2json.com/v1/api.json';
const ARTICLES_PER_FEED = 2;

interface FeedState {
  articles: RssArticle[];
  loading: boolean;
  error: string | null;
}

async function fetchFeed(feed: RssFeed): Promise<RssArticle[]> {
  const url = `${RSS2JSON}?rss_url=${encodeURIComponent(feed.url)}`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const data = await res.json();
  if (data.status !== 'ok') throw new Error(data.message ?? 'Feed error');

  return data.items.slice(0, ARTICLES_PER_FEED).map((item: Record<string, string>) => ({
    title: item.title,
    link: item.link,
    pubDate: item.pubDate,
    description: item.description
      ? item.description.replace(/<[^>]+>/g, '').slice(0, 160).trimEnd() + '…'
      : '',
    thumbnail: item.thumbnail || undefined,
    source: feed.name,
  }));
}

export function useRssFeeds(feeds: RssFeed[]): FeedState {
  const [state, setState] = useState<FeedState>({
    articles: [],
    loading: true,
    error: null,
  });

  useEffect(() => {
    let cancelled = false;

    const fetchAll = async () => {
      const articles: RssArticle[] = [];
      for (const feed of feeds) {
        if (cancelled) return;
        try {
          const items = await fetchFeed(feed);
          articles.push(...items);
        } catch {
          // skip failing feeds silently
        }
        await new Promise((r) => setTimeout(r, 400)); // respect rss2json rate limit
      }
      if (!cancelled) setState({ articles, loading: false, error: null });
    };

    fetchAll();

    return () => { cancelled = true; };
  }, []);  // feeds list is static, no need to re-fetch

  return state;
}
