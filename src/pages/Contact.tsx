import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Helmet } from 'react-helmet-async';
import { assetPaths } from '../data';

const BASE_URL = import.meta.env.VITE_BASE_URL ?? '';

type Status = 'idle' | 'sending' | 'success' | 'error';

const Contact = () => {
  const { t, i18n } = useTranslation();
  const isFr = i18n.language.startsWith('fr');
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
      <Helmet>
        <title>{isFr ? 'Contact – Hamdi Mechelloukh' : 'Contact – Hamdi Mechelloukh'}</title>
        <meta name="description" content={isFr
          ? 'Contactez Hamdi Mechelloukh, Data Engineer à Lille, pour discuter d\'un projet ou d\'une opportunité.'
          : 'Contact Hamdi Mechelloukh, Data Engineer in Lille, to discuss a project or opportunity.'} />
        <link rel="canonical" href={BASE_URL + '/contact'} />
        <meta property="og:url" content={BASE_URL + '/contact'} />
        <meta property="og:title" content={isFr ? 'Contact – Hamdi Mechelloukh' : 'Contact – Hamdi Mechelloukh'} />
      </Helmet>
      <section className="section">
        <h1 className="section-title">
          <img src={assetPaths.contact} alt="" />
          {t('contact.title')}
        </h1>
        <p className="card-text" style={{ textAlign: 'center', marginBottom: '3rem' }}>
          {t('contact.description')}
        </p>
        <form className="contact-form" onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="name">{t('contact.name')}</label>
            <input type="text" id="name" name="name" className="form-control" required />
          </div>
          <div className="form-group">
            <label htmlFor="email">{t('contact.email')}</label>
            <input type="email" id="email" name="email" className="form-control" required />
          </div>
          <div className="form-group">
            <label htmlFor="message">{t('contact.message')}</label>
            <textarea id="message" name="message" rows={5} className="form-control" required></textarea>
          </div>
          <button type="submit" className="btn" style={{ width: '100%', marginTop: '1rem' }} disabled={status === 'sending'}>
            {status === 'sending' ? t('contact.sending') : t('contact.send')}
          </button>
          {status === 'success' && (
            <p style={{ marginTop: '1rem', textAlign: 'center', color: 'var(--primary)' }}>{t('contact.success')}</p>
          )}
          {status === 'error' && (
            <p style={{ marginTop: '1rem', textAlign: 'center', color: 'red' }}>{t('contact.error')}</p>
          )}
        </form>
      </section>
    </div>
  );
};

export default Contact;
