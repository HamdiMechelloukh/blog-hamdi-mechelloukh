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

## Crossposter

Script qui publie automatiquement les articles anglais sur dev.to et LinkedIn, et envoie une notification Telegram pour l'import manuel sur Medium (Medium n'offre plus de token d'intégration).

### Cadence

1 article par run, 3 runs par semaine (mar/mer/jeu 09:00 Paris) via GitHub Actions. Cela étale la publication des N articles × 3 plateformes dans le temps et évite de flooder les feeds.

### Variables d'environnement

Copier `.env.example` vers `.env` pour tester en local. En prod, les mêmes valeurs sont à mettre dans GitHub Secrets :

- `DEVTO_API_KEY` — settings dev.to → Extensions → API Keys
- `LINKEDIN_ACCESS_TOKEN`, `LINKEDIN_USER_URN` — obtenus via `npm run linkedin-auth` (voir ci-dessous)
- `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID` — bot perso pour les notifications Medium

### Setup LinkedIn (une seule fois, tous les ~60 jours)

1. Créer une app sur https://www.linkedin.com/developers/apps avec les produits "Share on LinkedIn" + "Sign In with LinkedIn using OpenID Connect" et la redirect URL `http://localhost:5555/callback`.
2. Mettre `LINKEDIN_CLIENT_ID` et `LINKEDIN_CLIENT_SECRET` dans `.env`.
3. Lancer `npm run linkedin-auth`, suivre l'URL affichée, copier les valeurs finales dans `.env` (et les GitHub Secrets pour la prod).
4. Le token expire au bout de ~60 jours — re-run la commande avant expiration.

### Scripts

```bash
npm run crosspost:list   # voir la file d'attente (articles × plateformes non publiés)
npm run crosspost:dry    # simuler le prochain run sans publier
npm run crosspost        # publier 1 élément de la file
```

### État

L'état des publications est persisté dans `.crossposter-state.json` à la racine (committé par GitHub Actions après chaque run).

### Workflow Medium

Quand un article est sélectionné pour Medium, le script envoie un message Telegram avec le lien vers `https://medium.com/p/import` et la commande à lancer une fois l'import fait :

```bash
npm run crosspost -- --mark-done medium <slug>
```
