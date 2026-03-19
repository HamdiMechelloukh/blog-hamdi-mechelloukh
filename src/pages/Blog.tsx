import { useTranslation } from 'react-i18next';
import { Helmet } from 'react-helmet-async';
import { assetPaths, rssFeeds } from '../data';
import { useRssFeeds } from '../hooks/useRssFeeds';

const BASE_URL = import.meta.env.VITE_BASE_URL ?? '';

const Blog = () => {
  const { t, i18n } = useTranslation();
  const isFr = i18n.language.startsWith('fr');
  const { articles, loading, error } = useRssFeeds(rssFeeds);

  return (
    <div className="container">
      <Helmet>
        <title>{isFr ? 'Veille – Hamdi Mechelloukh' : 'Tech watch – Hamdi Mechelloukh'}</title>
        <meta name="description" content={isFr
          ? 'Veille technologique de Hamdi Mechelloukh : data engineering, cloud, architecture. Articles issus de Towards Data Science, AWS Blog, Databricks et plus.'
          : 'Tech watch by Hamdi Mechelloukh: data engineering, cloud and architecture articles from Towards Data Science, AWS Blog, Databricks and more.'} />
        <link rel="canonical" href={BASE_URL + '/veille'} />
        <meta property="og:url" content={BASE_URL + '/veille'} />
        <meta property="og:title" content={isFr ? 'Veille – Hamdi Mechelloukh' : 'Tech watch – Hamdi Mechelloukh'} />
        <meta property="og:description" content={isFr
          ? 'Veille technologique data engineering, cloud et architecture.'
          : 'Tech watch on data engineering, cloud and architecture.'} />
      </Helmet>
      <section className="section">
        <h1 className="section-title">
          <img src={assetPaths.blog} alt="" />
          {t('blog.title')}
        </h1>
        <p className="card-text" style={{ textAlign: 'center', marginBottom: '3rem' }}>
          {t('blog.description')}
        </p>

        {error && (
          <p style={{ textAlign: 'center', color: 'red' }}>{error}</p>
        )}

        <div className="card-grid">
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
          {loading && Array.from({ length: articles.length === 0 ? 6 : 2 }).map((_, i) => (
            <div key={`sk-${i}`} className="card skeleton-card">
              <div className="skeleton skeleton-img" />
              <div className="card-body">
                <div className="skeleton skeleton-tag" />
                <div className="skeleton skeleton-title" />
                <div className="skeleton skeleton-text" />
                <div className="skeleton skeleton-text skeleton-text--short" />
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
};

export default Blog;
