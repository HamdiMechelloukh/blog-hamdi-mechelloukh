import { projects, assetPaths } from '../data';
import ProjectCard from '../components/ProjectCard';

const Portfolio = () => {
  return (
    <div className="container">
      <section className="section">
        <h1 className="section-title">
          <img src={assetPaths.portfolio} alt="" />
          Portfolio
        </h1>
        <p className="card-text" style={{ textAlign: 'center', marginBottom: '3rem' }}>
          Une sélection de mes réalisations récentes, allant d'applications web complexes 
          à des outils simples mais efficaces.
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
