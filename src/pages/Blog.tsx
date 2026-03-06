import { useTranslation } from 'react-i18next';
import { assetPaths, rssFeeds } from '../data';
import { useRssFeeds } from '../hooks/useRssFeeds';

const Blog = () => {
  const { t } = useTranslation();
  const { articles, loading, error } = useRssFeeds(rssFeeds);

  return (
    <div className="container">
      <section className="section">
        <h1 className="section-title">
          <img src={assetPaths.blog} alt="" />
          {t('blog.title')}
        </h1>
        <p className="card-text" style={{ textAlign: 'center', marginBottom: '3rem' }}>
          {t('blog.description')}
        </p>

        {loading && (
          <p style={{ textAlign: 'center', color: 'var(--secondary)' }}>{t('blog.loading')}</p>
        )}

        {error && (
          <p style={{ textAlign: 'center', color: 'red' }}>{error}</p>
        )}

        {!loading && articles.length > 0 && (
          <div className="card-grid" style={{ gridTemplateColumns: 'repeat(2, 1fr)' }}>
            {articles.map((article, i) => (
              <a
                key={i}
                href={article.link}
                target="_blank"
                rel="noopener noreferrer"
                style={{ textDecoration: 'none', color: 'inherit' }}
              >
                <article className="card" style={{ height: '100%' }}>
                  {article.thumbnail && (
                    <img src={article.thumbnail} alt="" className="card-img" style={{ objectFit: 'cover', height: '160px' }} />
                  )}
                  <div className="card-body">
                    <span className="tag" style={{ marginBottom: '0.5rem', display: 'inline-block', fontSize: '0.75rem' }}>
                      {article.source}
                    </span>
                    <h3 className="card-title" style={{ fontSize: '1rem' }}>{article.title}</h3>
                    <p className="card-text" style={{ fontSize: '0.9rem' }}>{article.description}</p>
                    <span style={{ fontSize: '0.8rem', color: 'var(--secondary)' }}>
                      {new Date(article.pubDate).toLocaleDateString(t('blog.title') === 'Tech watch' ? 'en-GB' : 'fr-FR')}
                    </span>
                  </div>
                </article>
              </a>
            ))}
          </div>
        )}
      </section>
    </div>
  );
};

export default Blog;
