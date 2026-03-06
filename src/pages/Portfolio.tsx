import { useTranslation } from 'react-i18next';
import { projects, assetPaths } from '../data';
import ProjectCard from '../components/ProjectCard';

const Portfolio = () => {
  const { t } = useTranslation();

  return (
    <div className="container">
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
