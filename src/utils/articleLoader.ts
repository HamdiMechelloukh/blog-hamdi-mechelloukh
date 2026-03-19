const modules = import.meta.glob('/src/content/articles/*.md', {
  query: '?raw',
  import: 'default',
});

export async function loadArticleContent(
  slug: string
): Promise<string | null> {
  const path = `/src/content/articles/${slug}.md`;
  const loader = modules[path];
  if (!loader) return null;
  return (await loader()) as string;
}
