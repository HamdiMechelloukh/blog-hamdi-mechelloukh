import { Article } from '../types';

interface ArticleCardProps {
  article: Article;
}

const ArticleCard = ({ article }: ArticleCardProps) => {
  return (
    <article className="card">
      <img src={article.imageUrl} alt={article.title} className="card-img" />
      <div className="card-body">
        <h3 className="card-title">{article.title}</h3>
        <p className="card-text">{article.summary}</p>
        <span className="tag" style={{ backgroundColor: 'var(--bg-color)', color: 'var(--secondary)' }}>
          {new Date(article.date).toLocaleDateString()}
        </span>
        <div style={{ marginTop: '1rem' }}>
          <a href="#" className="btn">Lire la suite</a>
        </div>
      </div>
    </article>
  );
};

export default ArticleCard;
