import { useTranslation } from 'react-i18next';
import { Helmet } from 'react-helmet-async';
import { assetPaths, experiences, education } from '../data';

const BASE_URL = import.meta.env.VITE_BASE_URL ?? '';

const About = () => {
  const { t, i18n } = useTranslation();
  const isFr = i18n.language.startsWith('fr');

  return (
    <div className="container">
      <Helmet>
        <title>{isFr ? 'À propos – Hamdi Mechelloukh' : 'About – Hamdi Mechelloukh'}</title>
        <meta name="description" content={isFr
          ? 'Parcours, expériences et centres d\'intérêt de Hamdi Mechelloukh, Data Engineer chez Decathlon Digital à Lille.'
          : 'Career path, experience and interests of Hamdi Mechelloukh, Data Engineer at Decathlon Digital in Lille.'} />
        <link rel="canonical" href={BASE_URL + '/about'} />
        <meta property="og:url" content={BASE_URL + '/about'} />
        <meta property="og:title" content={isFr ? 'À propos – Hamdi Mechelloukh' : 'About – Hamdi Mechelloukh'} />
        <meta property="og:description" content={isFr
          ? 'Parcours, expériences et centres d\'intérêt de Hamdi Mechelloukh, Data Engineer chez Decathlon Digital.'
          : 'Career path, experience and interests of Hamdi Mechelloukh, Data Engineer at Decathlon Digital.'} />
      </Helmet>
      <section className="section">
        <h1 className="section-title">
          <img src={assetPaths.about} alt="" />
          {t('about.title')}
        </h1>
        <div className="about-content">
          <div style={{ flex: 1, display: 'flex', justifyContent: 'center' }}>
            <img
              src={assetPaths.avatar}
              alt="Hamdi Mechelloukh"
              style={{ width: '250px', height: '250px', borderRadius: '50%', objectFit: 'cover' }}
            />
          </div>
          <div style={{ flex: 2 }}>
            <h2>{t('about.who_title')}</h2>
            <p className="card-text" style={{ fontSize: '1.1rem', marginBottom: '1.5rem' }}>
              {t('about.who_text')}
            </p>
            <h3>{t('about.journey_title')}</h3>
            <p className="card-text">{t('about.journey_text')}</p>
          </div>
        </div>
      </section>

      <section className="section">
        <h2 className="section-title">{t('about.interests_title')}</h2>
        <div style={{ maxWidth: '800px', margin: '0 auto' }}>
          <div className="card-grid">
            <div className="card" style={{ padding: '1.5rem', textAlign: 'center' }}>
              <h3>{t('about.interest_karate_title')}</h3>
              <p className="card-text">{t('about.interest_karate_text')}</p>
            </div>
            <div className="card" style={{ padding: '1.5rem', textAlign: 'center' }}>
              <h3>{t('about.interest_gaming_title')}</h3>
              <p className="card-text">{t('about.interest_gaming_text')}</p>
            </div>
            <div className="card" style={{ padding: '1.5rem', textAlign: 'center' }}>
              <h3>{t('about.interest_pop_title')}</h3>
              <p className="card-text">{t('about.interest_pop_text')}</p>
            </div>
          </div>
        </div>
      </section>

      <section className="section">
        <h2 className="section-title">{t('about.experience_title')}</h2>
        <div style={{ maxWidth: '800px', margin: '0 auto' }}>
          {experiences.map((exp) => (
            <div key={exp.id} className="card" style={{ marginBottom: '1.5rem', padding: '1.5rem' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', flexWrap: 'wrap', gap: '0.5rem' }}>
                <div>
                  <h3 style={{ margin: 0 }}>{exp.role}</h3>
                  <p style={{ margin: '0.25rem 0', fontWeight: 600, color: 'var(--primary)' }}>{exp.company}</p>
                </div>
                <div style={{ textAlign: 'right' }}>
                  <p style={{ margin: 0, fontSize: '0.9rem', color: 'var(--secondary)' }}>{exp.period}</p>
                  <p style={{ margin: 0, fontSize: '0.85rem', color: 'var(--secondary)' }}>{exp.location}</p>
                </div>
              </div>
              <p className="card-text" style={{ marginTop: '0.75rem' }}>{exp.description}</p>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.5rem', marginTop: '0.75rem' }}>
                {exp.technologies.map((tech) => (
                  <span key={tech} style={{
                    background: 'var(--primary)',
                    color: '#fff',
                    padding: '0.2rem 0.6rem',
                    borderRadius: '4px',
                    fontSize: '0.8rem'
                  }}>{tech}</span>
                ))}
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="section">
        <h2 className="section-title">{t('about.education_title')}</h2>
        <div style={{ maxWidth: '800px', margin: '0 auto' }}>
          {education.map((edu) => (
            <div key={edu.id} className="card" style={{ marginBottom: '1.5rem', padding: '1.5rem' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', flexWrap: 'wrap', gap: '0.5rem' }}>
                <div>
                  <h3 style={{ margin: 0 }}>{edu.degree}</h3>
                  <p style={{ margin: '0.25rem 0', fontWeight: 600, color: 'var(--primary)' }}>{edu.school}</p>
                </div>
                <p style={{ margin: 0, fontSize: '0.9rem', color: 'var(--secondary)' }}>{edu.period}</p>
              </div>
              {edu.description && (
                <p className="card-text" style={{ marginTop: '0.75rem' }}>{edu.description}</p>
              )}
            </div>
          ))}
        </div>
      </section>
    </div>
  );
};

export default About;
