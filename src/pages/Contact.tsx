import { assetPaths } from '../data';

const Contact = () => {
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    alert('Merci pour votre message ! Je vous répondrai dès que possible.');
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
          N'hésitez pas à me contacter via le formulaire ci-dessous ou sur mes réseaux sociaux.
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
          <button type="submit" className="btn" style={{ width: '100%', marginTop: '1rem' }}>Envoyer</button>
        </form>
      </section>
    </div>
  );
};

export default Contact;
