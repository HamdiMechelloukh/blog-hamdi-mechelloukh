import { render, screen, fireEvent } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { describe, it, expect } from 'vitest';
import App from '../src/App';

describe('Navigation Flow', () => {
  it('navigates to different pages', () => {
    render(
      <BrowserRouter>
        <App />
      </BrowserRouter>
    );

    // Default: Home
    expect(screen.getByText(/Développeur Full-Stack/i)).toBeInTheDocument();

    // Click on About
    const aboutLink = screen.getByRole('link', { name: /À propos/i });
    fireEvent.click(aboutLink);
    expect(screen.getByText(/Qui suis-je \?/i)).toBeInTheDocument();

    // Click on Portfolio (Projets in nav)
    const portfolioLink = screen.getByRole('link', { name: /Projets/i });
    fireEvent.click(portfolioLink);
    // Heading should be Portfolio
    const portfolioHeadings = screen.getAllByText(/Portfolio/i);
    expect(portfolioHeadings.length).toBeGreaterThan(0);

    // Click on Blog
    const blogLink = screen.getByRole('link', { name: /Blog/i });
    fireEvent.click(blogLink);
    const blogHeadings = screen.getAllByText(/Blog/i);
    expect(blogHeadings.length).toBeGreaterThan(0);

    // Click on Contact
    const contactLink = screen.getByRole('link', { name: /Contact/i });
    fireEvent.click(contactLink);
    const contactHeadings = screen.getAllByText(/Contact/i);
    expect(contactHeadings.length).toBeGreaterThan(0);
  });
});
