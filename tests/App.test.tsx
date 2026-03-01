import { render, screen, fireEvent } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { describe, it, expect, vi } from 'vitest';
import App from '../src/App';
import Home from '../src/pages/Home';
import About from '../src/pages/About';
import Portfolio from '../src/pages/Portfolio';
import Blog from '../src/pages/Blog';
import Contact from '../src/pages/Contact';

describe('Smoke and Navigation Tests', () => {
  it('renders the App and Home page by default', () => {
    render(
      <BrowserRouter>
        <App />
      </BrowserRouter>
    );
    const headings = screen.getAllByText(/Hamdi Mechelloukh/i);
    expect(headings.length).toBeGreaterThan(0);
    expect(screen.getByText(/Développeur Full-Stack/i)).toBeInTheDocument();
  });

  it('renders navigation links', () => {
    render(
      <BrowserRouter>
        <App />
      </BrowserRouter>
    );
    expect(screen.getByText('Accueil')).toBeInTheDocument();
    expect(screen.getByText('À propos')).toBeInTheDocument();
    expect(screen.getByText('Projets')).toBeInTheDocument();
    expect(screen.getByText('Blog')).toBeInTheDocument();
    expect(screen.getByText('Contact')).toBeInTheDocument();
  });
});

describe('Page Component Rendering', () => {
  it('renders the About page', () => {
    render(
      <BrowserRouter>
        <About />
      </BrowserRouter>
    );
    expect(screen.getByText(/Qui suis-je/i)).toBeInTheDocument();
    expect(screen.getByText(/Bonjour ! Je suis Hamdi Mechelloukh/i)).toBeInTheDocument();
  });

  it('renders the Portfolio page with projects', () => {
    render(
      <BrowserRouter>
        <Portfolio />
      </BrowserRouter>
    );
    // Use getAllByText for Portfolio since it's also in the nav
    const portfolioHeadings = screen.getAllByText(/Portfolio/i);
    expect(portfolioHeadings.length).toBeGreaterThan(0);
    expect(screen.getByText(/E-Commerce Dashboard/i)).toBeInTheDocument();
    expect(screen.getByText(/Task Manager API/i)).toBeInTheDocument();
  });

  it('renders the Blog page with articles', () => {
    render(
      <BrowserRouter>
        <Blog />
      </BrowserRouter>
    );
    const blogHeadings = screen.getAllByText(/Blog/i);
    expect(blogHeadings.length).toBeGreaterThan(0);
    expect(screen.getByText(/Comprendre TypeScript en 2024/i)).toBeInTheDocument();
    expect(screen.getByText(/Optimiser les performances React/i)).toBeInTheDocument();
  });

  it('renders the Contact page and handle form submission', () => {
    // Mock window.alert
    const alertMock = vi.spyOn(window, 'alert').mockImplementation(() => {});
    
    render(
      <BrowserRouter>
        <Contact />
      </BrowserRouter>
    );
    
    const contactHeadings = screen.getAllByText(/Contact/i);
    expect(contactHeadings.length).toBeGreaterThan(0);
    expect(screen.getByLabelText(/Nom/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Email/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Message/i)).toBeInTheDocument();

    // Fill form
    fireEvent.change(screen.getByLabelText(/Nom/i), { target: { value: 'John Doe' } });
    fireEvent.change(screen.getByLabelText(/Email/i), { target: { value: 'john@example.com' } });
    fireEvent.change(screen.getByLabelText(/Message/i), { target: { value: 'Bonjour !' } });

    // Submit form
    fireEvent.click(screen.getByRole('button', { name: /Envoyer/i }));

    expect(alertMock).toHaveBeenCalledWith('Merci pour votre message ! Je vous répondrai dès que possible.');
    
    alertMock.mockRestore();
  });
});
