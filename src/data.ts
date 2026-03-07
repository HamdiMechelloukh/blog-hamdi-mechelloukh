import { Project, Article, Experience, Education, RssFeed } from './types';

export const projects: Project[] = [
  {
    id: 3,
    title: "Portfolio Personnel",
    description: "Le site sur lequel vous naviguez actuellement, construit avec React et Vite.",
    technologies: ["React", "TypeScript", "Vite"],
    imageUrl: "/assets/project_placeholder.svg",
    githubUrl: "#",
    demoUrl: "#"
  }
];

export const articles: Article[] = [
  {
    id: 1,
    title: "Comprendre TypeScript en 2024",
    summary: "Pourquoi TypeScript est devenu incontournable pour les développeurs JavaScript modernes.",
    date: "2023-10-15",
    imageUrl: "/assets/article_placeholder.svg",
    slug: "comprendre-typescript"
  },
  {
    id: 2,
    title: "Optimiser les performances React",
    summary: "Techniques avancées pour rendre vos applications React plus rapides et réactives.",
    date: "2023-11-02",
    imageUrl: "/assets/article_placeholder.svg",
    slug: "optimiser-react"
  },
  {
    id: 3,
    title: "Docker pour les débutants",
    summary: "Introduction à la conteneurisation et comment démarrer avec Docker.",
    date: "2023-12-10",
    imageUrl: "/assets/article_placeholder.svg",
    slug: "docker-debutant"
  }
];

export const experiences: Experience[] = [
  {
    id: 1,
    company: "Decathlon Digital",
    role: "Data Engineer",
    period: "Juil. 2024 – Aujourd'hui",
    location: "Lille, Hybride",
    description: "Développement de jobs Scala/Python Spark sur environnement AWS/Databricks pour l'équipe des données de prix",
    technologies: ["Scala", "Python", "Apache Spark", "AWS", "Databricks"]
  },
  {
    id: 2,
    company: "Decathlon Digital",
    role: "Production Manager – Data Domain Sales",
    period: "Juil. 2022 – Déc. 2024",
    location: "Croix, Hauts-de-France",
    description: "Garant de la stabilité du produit Perfeco (données de performance économique/ventes). Animation des parties prenantes de la chaîne d'intégration des données, gestion des postmortems, organisation du support et de l'astreinte.",
    technologies: ["Data Quality", "JIRA", "Gestion de crise"]
  },
  {
    id: 3,
    company: "Objectware",
    role: "Developer",
    period: "Janv. 2022 – Juin 2022",
    location: "Lille",
    description: "Développement logiciel en mission client.",
    technologies: ["TypeScript", "Node.js"]
  },
  {
    id: 4,
    company: "Boulanger",
    role: "Back-end Developer",
    period: "Mars 2021 – Juin 2022",
    location: "Lille",
    description: "Développement de REST APIs et de loaders Apache Kafka.",
    technologies: ["Java 11", "Spring Boot", "Apache Kafka", "Kafka Streams", "MongoDB", "Docker"]
  },
  {
    id: 5,
    company: "PROMOD",
    role: "Data Engineer",
    period: "Nov. 2019 – Mars 2021",
    location: "Lille",
    description: "Développement de jobs PySpark dans Azure HDInsight (environnement Hadoop) via Cloudera pour des besoins BI et DataScience.",
    technologies: ["Python", "PySpark", "Apache Spark", "Apache Hadoop", "Apache Hive", "Cloudera Impala", "Microsoft Azure", "Apache Oozie"]
  },
  {
    id: 6,
    company: "Boulanger",
    role: "Back-end Developer",
    period: "Avr. 2019 – Nov. 2019",
    location: "Lille",
    description: "Développement de REST APIs et de loaders Apache Kafka.",
    technologies: ["Java 8", "Spring Boot", "Apache Kafka", "Kafka Streams", "MongoDB", "Docker"]
  },
  {
    id: 7,
    company: "Leansys",
    role: "Developer",
    period: "Juin 2018 – Déc. 2021",
    location: "Villeneuve-d'Ascq",
    description: "Missions variées : APIs REST pour logiciels de gestion interne, plateforme de mise en relation chauffeurs/clients, chatbots cloud natifs multicanaux (Google Assistant, Amazon Alexa).",
    technologies: ["TypeScript", "Node.js", "MongoDB", "TypeORM", "Google Cloud Platform", "AWS", "PostgreSQL", "Docker", "DialogFlow", "Alexa Skill Kit"]
  }
];

export const education: Education[] = [
  {
    id: 1,
    school: "HEI Lille",
    degree: "Cycle Ingénieur",
    period: "2012 – 2014",
    description: "Grande école d'ingénieurs. Activités : AS Kickboxing, Association Aparté (Bande dessinée)."
  },
  {
    id: 2,
    school: "AFPA",
    degree: "Concepteur Développeur Informatique – Bac+4",
    period: "2017 – 2018",
    description: "Java, JPA/Hibernate, Spring, Android, JavaScript, PHP, SQL Server, MySQL."
  },
  {
    id: 3,
    school: "Lycée César Baggio",
    degree: "Classe Préparatoire ATS – Sciences Physiques",
    period: "2011 – 2012",
    description: "9ème de promo sur 39 élèves. Équivalent Bac+3 (60 ECTS validés)."
  },
  {
    id: 4,
    school: "Lycée Saint Rémi",
    degree: "BTS Électrotechnique",
    period: "2009 – 2011",
    description: "Mention Bien."
  }
];

export const rssFeeds: RssFeed[] = [
  {
    name: "Towards Data Science",
    url: "https://medium.com/feed/towards-data-science"
  },
  {
    name: "AWS Big Data Blog",
    url: "https://aws.amazon.com/blogs/big-data/feed/"
  },
  {
    name: "Databricks Blog",
    url: "https://www.databricks.com/blog/rss.xml"
  },
  {
    name: "dev.to – Data Engineering",
    url: "https://dev.to/feed/tag/dataengineering"
  },
  {
    name: "The New Stack – Data",
    url: "https://thenewstack.io/category/data/feed/"
  },
  {
    name: "InfoQ",
    url: "https://www.infoq.com/feed/"
  }
];

export const socialLinks = {
  github: "https://github.com/HamdiMechelloukh",
  linkedin: "https://linkedin.com/in/hamdimechelloukh",
  email: ""
};

export const assetPaths = {
  avatar: "/assets/PXL_20251202_194539824.jpg",
  home: "/assets/home_icon.svg",
  about: "/assets/about_icon.svg",
  portfolio: "/assets/portfolio_icon.svg",
  blog: "/assets/blog_icon.svg",
  contact: "/assets/contact_icon.svg",
  github: "/assets/github_icon.svg",
  linkedin: "/assets/linkedin_icon.svg",
  fullstack: "/assets/fullstack_icon.svg"
};
