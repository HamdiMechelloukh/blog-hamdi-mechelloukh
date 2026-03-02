import { assetPaths, experiences, education } from '../data';

const About = () => {
  return (
    <div className="container">
      <section className="section">
        <h1 className="section-title">
          <img src={assetPaths.about} alt="" />
          À propos
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
            <h2>Qui suis-je ?</h2>
            <p className="card-text" style={{ fontSize: '1.1rem', marginBottom: '1.5rem' }}>
              Bonjour ! Je suis Hamdi Mechelloukh, Data Engineer chez Decathlon Digital à Lille.
              Ingénieur de formation (HEI Lille), j'ai commencé ma carrière dans le développement
              backend avant de me spécialiser dans la data engineering. Je conçois et maintiens
              des pipelines de données à grande échelle avec Spark, AWS et Databricks.
            </p>
            <h3>Mon parcours</h3>
            <p className="card-text">
              Après un diplôme d'ingénieur à HEI Lille et une reconversion via l'AFPA,
              j'ai évolué du développement Node.js/Java vers la data engineering, en passant
              par des missions chez Leansys, Boulanger et PROMOD. Depuis 2022, je suis chez
              Decathlon Digital où j'ai assuré la stabilité des données de performance économique
              avant de rejoindre l'équipe Data Engineering.
            </p>
          </div>
        </div>
      </section>

      <section className="section">
        <h2 className="section-title">Expériences</h2>
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
        <h2 className="section-title">Centres d'intérêt</h2>
        <div style={{ maxWidth: '800px', margin: '0 auto' }}>
          <div className="card-grid">
            <div className="card" style={{ padding: '1.5rem', textAlign: 'center' }}>
              <h3>Karaté Kyokushinkai</h3>
              <p className="card-text">
                Pratiquant depuis mai 2013, 2ème Dan. Également professeur depuis 2025.
              </p>
            </div>
            <div className="card" style={{ padding: '1.5rem', textAlign: 'center' }}>
              <h3>Jeux vidéo</h3>
              <p className="card-text">
                Passionné de RPGs et de culture geek en général.
              </p>
            </div>
            <div className="card" style={{ padding: '1.5rem', textAlign: 'center' }}>
              <h3>Culture pop</h3>
              <p className="card-text">
                Manga, BD, mèmes et tout ce qui fait partie de la culture geek.
              </p>
            </div>
          </div>
        </div>
      </section>

      <section className="section">
        <h2 className="section-title">Formation</h2>
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
