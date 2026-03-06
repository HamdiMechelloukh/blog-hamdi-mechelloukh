import { useTranslation } from 'react-i18next';
import { Project } from '../types';
import { assetPaths } from '../data';

interface ProjectCardProps {
  project: Project;
}

const ProjectCard = ({ project }: ProjectCardProps) => {
  const { t } = useTranslation();

  return (
    <article className="card">
      <img src={project.imageUrl} alt={project.title} className="card-img" />
      <div className="card-body">
        <h3 className="card-title">{t(`portfolio.project_${project.id}_title`, project.title)}</h3>
        <p className="card-text">{t(`portfolio.project_${project.id}_description`, project.description)}</p>
        <div className="tag-list">
          {project.technologies.map((tech) => (
            <span key={tech} className="tag">{tech}</span>
          ))}
        </div>
        <div style={{ display: 'flex', gap: '1rem', marginTop: '1rem' }}>
          <a href={project.githubUrl} className="btn" target="_blank" rel="noopener noreferrer">
            <img src={assetPaths.github} alt="" style={{ width: '16px', height: '16px', verticalAlign: 'middle', marginRight: '0.5rem' }} />
            {t('portfolio.code')}
          </a>
          <a href={project.demoUrl} className="btn" style={{ backgroundColor: 'var(--secondary)' }} target="_blank" rel="noopener noreferrer">
            {t('portfolio.demo')}
          </a>
        </div>
      </div>
    </article>
  );
};

export default ProjectCard;
