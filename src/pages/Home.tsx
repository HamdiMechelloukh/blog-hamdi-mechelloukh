import { Link } from 'react-router-dom';
import { assetPaths } from '../data';

const Home = () => {
  return (
    <div className="container">
      <section className="hero">
        <img src={assetPaths.avatar} alt="Hamdi Mechelloukh" className="hero-avatar" />
        <h1 className="hero-title">Hamdi Mechelloukh</h1>
        <p className="hero-subtitle">Data Engineer @ Decathlon Digital</p>
        <p style={{ maxWidth: '600px', margin: '0 auto 2rem', fontSize: '1.1rem' }}>
          Ingénieur passionné par la data, basé à Lille. Spécialisé dans le traitement
          de données à grande échelle avec Spark, et le développement backend Java/Node.js.
        </p>
        <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
          <Link to="/portfolio" className="btn">
            Voir mes projets
          </Link>
          <Link to="/contact" className="btn" style={{ backgroundColor: 'var(--secondary)' }}>
            Me contacter
          </Link>
        </div>
      </section>

      <section className="section">
        <h2 className="section-title">
          <img src={assetPaths.fullstack} alt="" />
          Compétences
        </h2>
        <div className="card-grid" style={{ maxWidth: '900px', margin: '0 auto' }}>
          <div className="card" style={{ padding: '2rem', textAlign: 'center' }}>
            <h3>Data Engineering</h3>
            <p className="card-text">Python, Scala, Apache Spark, Databricks, AWS, Azure, Hadoop, Hive</p>
          </div>
          <div className="card" style={{ padding: '2rem', textAlign: 'center' }}>
            <h3>Backend</h3>
            <p className="card-text">Java, Spring Boot, Node.js, TypeScript, Apache Kafka, REST APIs</p>
          </div>
          <div className="card" style={{ padding: '2rem', textAlign: 'center' }}>
            <h3>Bases de données</h3>
            <p className="card-text">MongoDB, PostgreSQL, MySQL, Apache Hive, BigQuery</p>
          </div>
          <div className="card" style={{ padding: '2rem', textAlign: 'center' }}>
            <h3>Cloud & Outils</h3>
            <p className="card-text">AWS, Azure, GCP, Docker, Git, Jenkins</p>
          </div>
        </div>
      </section>
    </div>
  );
};

export default Home;
