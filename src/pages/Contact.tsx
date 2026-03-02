import { useState } from 'react';
import { assetPaths } from '../data';

type Status = 'idle' | 'sending' | 'success' | 'error';

const Contact = () => {
  const [status, setStatus] = useState<Status>('idle');

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setStatus('sending');
    const form = e.currentTarget;
    try {
      const res = await fetch('https://formspree.io/f/xlgwydgk', {
        method: 'POST',
        body: new FormData(form),
        headers: { Accept: 'application/json' },
      });
      if (res.ok) {
        setStatus('success');
        form.reset();
      } else {
        setStatus('error');
      }
    } catch {
      setStatus('error');
    }
  };

  return (
    <div className="container">
      <section className="section">
        <h1 className="section-title">
          <img src={assetPaths.contact} alt="" />
          Contact
        </h1>
        <p className="card-text" style={{ textAlign: 'center', marginBottom: '3rem' }}>
          Vous avez un projet en tête ou vous souhaitez simplement discuter ?
          N'hésitez pas à me contacter via le formulaire ci-dessous.
        </p>
        <form className="contact-form" onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="name">Nom</label>
            <input type="text" id="name" name="name" className="form-control" required />
          </div>
          <div className="form-group">
            <label htmlFor="email">Email</label>
            <input type="email" id="email" name="email" className="form-control" required />
          </div>
          <div className="form-group">
            <label htmlFor="message">Message</label>
            <textarea id="message" name="message" rows={5} className="form-control" required></textarea>
          </div>
          <button type="submit" className="btn" style={{ width: '100%', marginTop: '1rem' }} disabled={status === 'sending'}>
            {status === 'sending' ? 'Envoi en cours…' : 'Envoyer'}
          </button>
          {status === 'success' && (
            <p style={{ marginTop: '1rem', textAlign: 'center', color: 'var(--primary)' }}>
              Message envoyé, merci !
            </p>
          )}
          {status === 'error' && (
            <p style={{ marginTop: '1rem', textAlign: 'center', color: 'red' }}>
              Une erreur est survenue, réessaie plus tard.
            </p>
          )}
        </form>
      </section>
    </div>
  );
};

export default Contact;
