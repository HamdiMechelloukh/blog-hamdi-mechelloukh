import { NavLink } from 'react-router-dom';
import { assetPaths } from '../data';

const Navbar = () => {
  return (
    <nav className="navbar">
      <div className="container navbar-content">
        <NavLink to="/" className="nav-logo">
          <img src={assetPaths.fullstack} alt="Logo" style={{ width: '32px', height: '32px' }} />
          Hamdi.dev
        </NavLink>
        <div className="nav-links">
          <NavLink 
            to="/" 
            className={({ isActive }) => isActive ? "nav-link active" : "nav-link"}
            end
          >
            <img src={assetPaths.home} alt="Home" />
            <span>Accueil</span>
          </NavLink>
          <NavLink 
            to="/about" 
            className={({ isActive }) => isActive ? "nav-link active" : "nav-link"}
          >
            <img src={assetPaths.about} alt="About" />
            <span>À propos</span>
          </NavLink>
          <NavLink 
            to="/portfolio" 
            className={({ isActive }) => isActive ? "nav-link active" : "nav-link"}
          >
            <img src={assetPaths.portfolio} alt="Portfolio" />
            <span>Projets</span>
          </NavLink>
          <NavLink 
            to="/blog" 
            className={({ isActive }) => isActive ? "nav-link active" : "nav-link"}
          >
            <img src={assetPaths.blog} alt="Blog" />
            <span>Blog</span>
          </NavLink>
          <NavLink 
            to="/contact" 
            className={({ isActive }) => isActive ? "nav-link active" : "nav-link"}
          >
            <img src={assetPaths.contact} alt="Contact" />
            <span>Contact</span>
          </NavLink>
        </div>
      </div>
    </nav>
  );
};

export default Navbar;
