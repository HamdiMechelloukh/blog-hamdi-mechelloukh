import { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Helmet } from 'react-helmet-async';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneLight } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { blogArticles } from '../data';
import { loadArticleContent } from '../utils/articleLoader';

const BASE_URL = 'https://hamdimechelloukh.com';

const ArticlePage = () => {
  const { slug } = useParams<{ slug: string }>();
  const { t, i18n } = useTranslation();
  const isFr = i18n.language.startsWith('fr');
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const article = blogArticles.find((a) => a.slug === slug);
  const counterpart = article
    ? blogArticles.find((a) => a.date === article.date && a.lang !== article.lang)
    : undefined;

  useEffect(() => {
    if (!slug) return;
    setLoading(true);
    loadArticleContent(slug).then((md) => {
      setContent(md);
      setLoading(false);
    });
  }, [slug]);

  if (!article) {
    return (
      <div className="container">
        <section className="section" style={{ textAlign: 'center' }}>
          <h1>{t('articles.notFound')}</h1>
          <Link to="/blog" className="btn" style={{ marginTop: '1rem', display: 'inline-flex' }}>
            {t('articles.backToList')}
          </Link>
        </section>
      </div>
    );
  }

  return (
    <div className="container">
      <Helmet>
        <title>{article.title} – Hamdi Mechelloukh</title>
        <meta name="description" content={article.summary} />
        <link rel="canonical" href={`${BASE_URL}/blog/${article.slug}`} />
        <link rel="alternate" hreflang={article.lang} href={`${BASE_URL}/blog/${article.slug}`} />
        {counterpart && (
          <link rel="alternate" hreflang={counterpart.lang} href={`${BASE_URL}/blog/${counterpart.slug}`} />
        )}
        <link rel="alternate" hreflang="x-default" href={`${BASE_URL}/blog/${article.lang === 'fr' ? article.slug : (counterpart?.slug ?? article.slug)}`} />
        <meta property="og:url" content={`${BASE_URL}/blog/${article.slug}`} />
        <meta property="og:title" content={article.title} />
        <meta property="og:description" content={article.summary} />
        <meta property="og:type" content="article" />
        <meta property="og:locale" content={article.lang === 'fr' ? 'fr_FR' : 'en_US'} />
      </Helmet>
      <article className="article-container">
        <Link to="/blog" className="article-back">
          &larr; {t('articles.backToList')}
        </Link>
        <header className="article-header">
          <h1 className="article-title">{article.title}</h1>
          <div className="article-meta">
            <span>
              {t('articles.publishedOn')}{' '}
              {new Date(article.date).toLocaleDateString(
                isFr ? 'fr-FR' : 'en-GB',
                { year: 'numeric', month: 'long', day: 'numeric' }
              )}
            </span>
            <span>·</span>
            <span>{article.readingTimeMinutes} {t('articles.readingTime')}</span>
          </div>
          <div className="tag-list" style={{ justifyContent: 'center' }}>
            {article.tags.map((tag) => (
              <span key={tag} className="tag">{tag}</span>
            ))}
          </div>
        </header>

        {loading ? (
          <div style={{ textAlign: 'center', padding: '3rem', color: 'var(--secondary)' }}>
            {t('articles.loading')}
          </div>
        ) : content ? (
          <div className="article-prose">
            <ReactMarkdown
              components={{
                code({ className, children, ...props }) {
                  const match = /language-(\w+)/.exec(className || '');
                  const codeString = String(children).replace(/\n$/, '');
                  if (match) {
                    return (
                      <SyntaxHighlighter
                        style={oneLight}
                        language={match[1]}
                        PreTag="div"
                      >
                        {codeString}
                      </SyntaxHighlighter>
                    );
                  }
                  return (
                    <code className={className} {...props}>
                      {children}
                    </code>
                  );
                },
              }}
            >
              {content}
            </ReactMarkdown>
          </div>
        ) : (
          <p style={{ textAlign: 'center', color: 'var(--secondary)' }}>
            {t('articles.contentNotFound')}
          </p>
        )}
      </article>
    </div>
  );
};

export default ArticlePage;
