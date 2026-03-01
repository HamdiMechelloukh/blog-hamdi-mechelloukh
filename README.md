# Portfolio Hamdi Mechelloukh

Ce projet est le frontend du portfolio et blog de Hamdi Mechelloukh, développeur Full-Stack.
Il est réalisé avec React, TypeScript et Vite.

## Structure du projet

- `src/components`: Composants réutilisables (Navbar, Footer, Cards).
- `src/pages`: Pages de l'application (Home, About, Portfolio, Blog, Contact).
- `src/data.ts`: Données en dur (projets, articles) et configuration des assets.
- `src/types.ts`: Définitions TypeScript.
- `src/index.css`: Styles globaux.

## Installation

1. Assurez-vous d'avoir Node.js installé.
2. Installez les dépendances :

```bash
npm install
```

## Lancement

Pour lancer le serveur de développement :

```bash
npm run dev
```

L'application sera accessible sur `http://localhost:5173`.

## Build

Pour construire l'application pour la production :

```bash
npm run build
```

## Assets

Les icônes et images sont situées dans le dossier `assets` à la racine (servi comme racine par Vite).
Le manifest des assets se trouve dans `assets/manifest.json`.
