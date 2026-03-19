export interface Project {
  id: number;
  title: string;
  description: string;
  technologies: string[];
  imageUrl: string; // Path to SVG or placeholder
  githubUrl: string;
  demoUrl?: string;
}

export interface Article {
  id: number;
  title: string;
  summary: string;
  date: string;
  imageUrl: string;
  slug: string; // for potential routing (though we list them)
}

export interface Experience {
  id: number;
  company: string;
  role: string;
  period: string;
  location: string;
  description: string;
  technologies: string[];
}

export interface Education {
  id: number;
  school: string;
  degree: string;
  period: string;
  description?: string;
}

export interface RssFeed {
  name: string;
  url: string;
}

export interface RssArticle {
  title: string;
  link: string;
  pubDate: string;
  description: string;
  thumbnail?: string;
  source: string;
}

export interface BlogArticle {
  slug: string;
  title: string;
  summary: string;
  date: string;
  tags: string[];
  readingTimeMinutes: number;
}
