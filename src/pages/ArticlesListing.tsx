import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Helmet } from 'react-helmet-async';
import { blogArticles, assetPaths } from '../data';

const BASE_URL = import.meta.env.VITE_BASE_URL ?? '';

const ArticlesListing = () => {
  const { t, i18n } = useTranslation();
  const isFr = i18n.language.startsWith('fr');
  const sorted = [...blogArticles].sort(
    (a, b) => new Date(b.date).getTime() - new Date(a.date).getTime()
  );

  return (
    <div className="container">
      <Helmet>
        <title>{isFr ? 'Blog – Hamdi Mechelloukh' : 'Blog – Hamdi Mechelloukh'}</title>
        <meta
          name="description"
          content={t('articles.description')}
        />
        <link rel="canonical" href={BASE_URL + '/blog'} />
        <meta property="og:url" content={BASE_URL + '/blog'} />
        <meta property="og:title" content="Blog – Hamdi Mechelloukh" />
        <meta property="og:description" content={t('articles.description')} />
      </Helmet>
      <section className="section">
        <h1 className="section-title">
          <img src={assetPaths.blog} alt="" />
          {t('articles.title')}
        </h1>
        <p className="card-text" style={{ textAlign: 'center', marginBottom: '3rem' }}>
          {t('articles.description')}
        </p>

        {sorted.length === 0 ? (
          <p style={{ textAlign: 'center', color: 'var(--secondary)' }}>
            {t('articles.empty')}
          </p>
        ) : (
          <div className="card-grid">
            {sorted.map((article) => (
              <Link
                key={article.slug}
                to={`/blog/${article.slug}`}
                style={{ textDecoration: 'none', color: 'inherit' }}
              >
                <article className="card" style={{ height: '100%' }}>
                  <div className="card-body">
                    <span
                      style={{
                        fontSize: '0.8rem',
                        color: 'var(--secondary)',
                        marginBottom: '0.5rem',
                        display: 'block',
                      }}
                    >
                      {new Date(article.date).toLocaleDateString(
                        isFr ? 'fr-FR' : 'en-GB',
                        { year: 'numeric', month: 'long', day: 'numeric' }
                      )}{' '}
                      · {article.readingTimeMinutes} {t('articles.readingTime')}
                    </span>
                    <h3 className="card-title">{article.title}</h3>
                    <p className="card-text">{article.summary}</p>
                    <div className="tag-list">
                      {article.tags.map((tag) => (
                        <span key={tag} className="tag">
                          {tag}
                        </span>
                      ))}
                    </div>
                  </div>
                </article>
              </Link>
            ))}
          </div>
        )}
      </section>
    </div>
  );
};

export default ArticlesListing;
