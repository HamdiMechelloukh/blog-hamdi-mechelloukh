import { useTranslation } from 'react-i18next';
import { Helmet } from 'react-helmet-async';
import { projects, assetPaths } from '../data';
import ProjectCard from '../components/ProjectCard';

const BASE_URL = import.meta.env.VITE_BASE_URL ?? '';

const Portfolio = () => {
  const { t, i18n } = useTranslation();
  const isFr = i18n.language.startsWith('fr');

  return (
    <div className="container">
      <Helmet>
        <title>{isFr ? 'Portfolio – Hamdi Mechelloukh' : 'Portfolio – Hamdi Mechelloukh'}</title>
        <meta name="description" content={isFr
          ? 'Projets data engineering de Hamdi Mechelloukh : lakehouse Olist (Kafka, Spark, Iceberg), pipeline MLOps (XGBoost, MLflow, Airflow), portfolio React/TypeScript.'
          : 'Data engineering projects by Hamdi Mechelloukh: Olist lakehouse (Kafka, Spark, Iceberg), MLOps pipeline (XGBoost, MLflow, Airflow), React/TypeScript portfolio.'} />
        <link rel="canonical" href={BASE_URL + '/portfolio'} />
        <meta property="og:url" content={BASE_URL + '/portfolio'} />
        <meta property="og:title" content={isFr ? 'Portfolio – Hamdi Mechelloukh' : 'Portfolio – Hamdi Mechelloukh'} />
        <meta property="og:description" content={isFr
          ? 'Projets data engineering : Olist lakehouse, MLOps pipeline, Kafka, Spark, XGBoost, Airflow.'
          : 'Data engineering projects: Olist lakehouse, MLOps pipeline, Kafka, Spark, XGBoost, Airflow.'} />
      </Helmet>
      <section className="section">
        <h1 className="section-title">
          <img src={assetPaths.portfolio} alt="" />
          {t('portfolio.title')}
        </h1>
        <p className="card-text" style={{ textAlign: 'center', marginBottom: '3rem' }}>
          {t('portfolio.description')}
        </p>
        <div className="card-grid">
          {projects.map((project) => (
            <ProjectCard key={project.id} project={project} />
          ))}
        </div>
      </section>
    </div>
  );
};

export default Portfolio;
