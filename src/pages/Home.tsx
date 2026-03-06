import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { assetPaths } from '../data';

const Home = () => {
  const { t } = useTranslation();

  return (
    <div className="container">
      <section className="hero">
        <img src={assetPaths.avatar} alt="Hamdi Mechelloukh" className="hero-avatar" />
        <h1 className="hero-title">Hamdi Mechelloukh</h1>
        <p className="hero-subtitle">{t('home.subtitle')}</p>
        <p style={{ maxWidth: '600px', margin: '0 auto 2rem', fontSize: '1.1rem' }}>
          {t('home.description')}
        </p>
        <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
          <Link to="/portfolio" className="btn">{t('home.cta_projects')}</Link>
          <Link to="/contact" className="btn" style={{ backgroundColor: 'var(--secondary)' }}>{t('home.cta_contact')}</Link>
        </div>
      </section>

      <section className="section">
        <h2 className="section-title">
          <img src={assetPaths.fullstack} alt="" />
          {t('home.skills_title')}
        </h2>
        <div className="card-grid" style={{ maxWidth: '900px', margin: '0 auto' }}>
          <div className="card" style={{ padding: '2rem', textAlign: 'center' }}>
            <h3>{t('home.skill_data')}</h3>
            <p className="card-text">Python, Scala, Apache Spark, Databricks, AWS, Azure, Hadoop, Hive</p>
          </div>
          <div className="card" style={{ padding: '2rem', textAlign: 'center' }}>
            <h3>{t('home.skill_backend')}</h3>
            <p className="card-text">Java, Spring Boot, Node.js, TypeScript, Apache Kafka, REST APIs</p>
          </div>
          <div className="card" style={{ padding: '2rem', textAlign: 'center' }}>
            <h3>{t('home.skill_db')}</h3>
            <p className="card-text">MongoDB, PostgreSQL, MySQL, Apache Hive, BigQuery</p>
          </div>
          <div className="card" style={{ padding: '2rem', textAlign: 'center' }}>
            <h3>{t('home.skill_cloud')}</h3>
            <p className="card-text">AWS, Azure, GCP, Docker, Git, Jenkins</p>
          </div>
        </div>
      </section>
    </div>
  );
};

export default Home;
