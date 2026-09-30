# Portfolio et blog de Hamdi Mechelloukh

Site statique généré en Rust, avec un rendu WebGPU (wgpu compilé en WebAssembly) derrière le contenu.
Le texte reste du HTML classique (SEO, accessibilité) ; le GPU dessine le fond, les particules, les panneaux et les halos.

## Structure

- `crates/site` : générateur statique maison (askama, pulldown-cmark, syntect). `content/` + `static/` → `dist/`.
- `crates/gpu` : module WebGPU. Un canvas plein écran lit la position des ancres `data-gpu="panel|card|title|target"`
  et dessine autour. Mini-jeu sur la page 404. Sans WebGPU, rien n'est chargé et le CSS suffit.
- `crates/crossposter` : publication des articles anglais sur dev.to et LinkedIn, notification Telegram pour Medium.
- `content/articles/*.md` : articles, avec un frontmatter TOML (`+++`). Chaque article a un jumeau dans l'autre langue (`translation_slug`).
- `content/data/*.toml` : projets, expériences, formation, flux de la veille.
- `content/i18n/{fr,en}.toml` : textes de l'interface. Une clé manquante ou en trop fait échouer le build.
- `static/` : CSS (ossature et repli sans WebGPU), images, favicon.
- `devto/` : versions dev.to des articles.

## URLs

Français à la racine (`/`, `/about`, `/blog`…), anglais sous `/en/`. Les articles sont en `/blog/<slug>` dans les deux langues.
Pages 404 par langue (`/404.html`, `/en/404.html`), choisies par préfixe dans `vercel.json`.

## Développement

Prérequis : Rust stable, la cible `wasm32-unknown-unknown` et `wasm-bindgen-cli` à la version de `Cargo.lock`.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <version de wasm-bindgen dans Cargo.lock>

./build.sh              # build complet dans dist/ (la veille est récupérée en ligne)
./build.sh --offline    # sans la veille
python3 -m http.server -d dist 8000
cargo test              # site + crossposter (gpu ne compile que pour wasm32)
```

Le rendu GPU demande un navigateur avec WebGPU. Sous Linux, il faut souvent l'activer à la main
(`chrome://flags/#enable-unsafe-webgpu`, ou `dom.webgpu.enabled` dans Firefox).

## Déploiement

Vercel : `vercel-install.sh` ajoute la cible wasm32 à la Rust préinstallée et télécharge `wasm-bindgen`,
puis `build.sh` génère `dist/`.

## Crossposter

1 élément par run, 3 runs par semaine (mar/mer/jeu, 06:00 UTC) via GitHub Actions, pour étaler la publication des articles × plateformes.
L'état est dans `.crossposter-state.json`, committé par le workflow après chaque run.

```bash
cargo run -p crossposter -- --list                        # file d'attente
cargo run -p crossposter -- --dry-run                     # simuler le prochain run
cargo run -p crossposter                                  # publier 1 élément
cargo run -p crossposter -- --mark-done medium <slug>     # après un import Medium manuel
```

### Variables d'environnement

Copier `.env.example` vers `.env` pour un usage local. En prod, les mêmes valeurs sont dans les GitHub Secrets :

- `DEVTO_API_KEY` : dev.to → Settings → Extensions → API Keys
- `LINKEDIN_USER_URN`, `LINKEDIN_REFRESH_TOKEN`, `LINKEDIN_CLIENT_ID`, `LINKEDIN_CLIENT_SECRET` : voir ci-dessous
  (`LINKEDIN_ACCESS_TOKEN` seul reste accepté si l'app n'a pas de refresh token)
- `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID` : bot perso pour les notifications Medium

### Setup LinkedIn (une fois, puis à l'expiration du refresh token, ~1 an)

1. Créer une app sur https://www.linkedin.com/developers/apps avec les produits « Share on LinkedIn » et
   « Sign In with LinkedIn using OpenID Connect », et la redirect URL `http://localhost:5555/callback`.
2. Mettre `LINKEDIN_CLIENT_ID` et `LINKEDIN_CLIENT_SECRET` dans `.env`.
3. Lancer `cargo run -p crossposter -- linkedin-auth`, ouvrir l'URL affichée, puis copier les valeurs finales dans les GitHub Secrets.
