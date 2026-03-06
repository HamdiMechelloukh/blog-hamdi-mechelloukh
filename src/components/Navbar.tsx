import { NavLink } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { assetPaths } from '../data';

const Navbar = () => {
  const { t, i18n } = useTranslation();

  const toggleLang = () => {
    const next = i18n.language === 'fr' ? 'en' : 'fr';
    i18n.changeLanguage(next);
    localStorage.setItem('lang', next);
  };

  return (
    <nav className="navbar">
      <div className="container navbar-content">
        <NavLink to="/" className="nav-logo">
          <img src={assetPaths.fullstack} alt="Logo" style={{ width: '32px', height: '32px' }} />
          Hamdi
        </NavLink>
        <div className="nav-links">
          <NavLink to="/" className={({ isActive }) => isActive ? "nav-link active" : "nav-link"} end>
            <img src={assetPaths.home} alt="Home" />
            <span>{t('nav.home')}</span>
          </NavLink>
          <NavLink to="/about" className={({ isActive }) => isActive ? "nav-link active" : "nav-link"}>
            <img src={assetPaths.about} alt="About" />
            <span>{t('nav.about')}</span>
          </NavLink>
          <NavLink to="/portfolio" className={({ isActive }) => isActive ? "nav-link active" : "nav-link"}>
            <img src={assetPaths.portfolio} alt="Portfolio" />
            <span>{t('nav.projects')}</span>
          </NavLink>
          <NavLink to="/blog" className={({ isActive }) => isActive ? "nav-link active" : "nav-link"}>
            <img src={assetPaths.blog} alt="Blog" />
            <span>{t('nav.blog')}</span>
          </NavLink>
          <NavLink to="/contact" className={({ isActive }) => isActive ? "nav-link active" : "nav-link"}>
            <img src={assetPaths.contact} alt="Contact" />
            <span>{t('nav.contact')}</span>
          </NavLink>
          <button onClick={toggleLang} style={{
            background: 'none',
            border: '1px solid var(--primary)',
            color: 'var(--primary)',
            borderRadius: '4px',
            padding: '0.25rem 0.6rem',
            cursor: 'pointer',
            fontWeight: 600,
            fontSize: '0.85rem'
          }}>
            {i18n.language === 'fr' ? 'FR' : 'EN'}
          </button>
        </div>
      </div>
    </nav>
  );
};

export default Navbar;
