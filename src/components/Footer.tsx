import { assetPaths, socialLinks } from '../data';

const Footer = () => {
  return (
    <footer className="footer">
      <div className="container">
        <div className="social-links">
          <a href={socialLinks.github} target="_blank" rel="noopener noreferrer">
            <img src={assetPaths.github} alt="GitHub" className="social-icon" />
          </a>
          <a href={socialLinks.linkedin} target="_blank" rel="noopener noreferrer">
            <img src={assetPaths.linkedin} alt="LinkedIn" className="social-icon" />
          </a>
        </div>
        <p>&copy; {new Date().getFullYear()} Hamdi Mechelloukh. Tous droits réservés.</p>
      </div>
    </footer>
  );
};

export default Footer;
