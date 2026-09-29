//! Obtient les tokens LinkedIn via le flow OAuth 2.0 « authorization code ».
//! À exécuter une seule fois en local, pour récupérer les valeurs à coller dans les secrets GitHub.
//!
//! Prérequis : une app sur https://www.linkedin.com/developers/apps avec
//!   - Product « Share on LinkedIn » (w_member_social)
//!   - Product « Sign In with LinkedIn using OpenID Connect » (openid + profile)
//!   - Redirect URL : http://localhost:5555/callback
//! et LINKEDIN_CLIENT_ID, LINKEDIN_CLIENT_SECRET dans .env ou l'environnement.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};
use reqwest::Url;
use reqwest::blocking::Client;
use serde::Deserialize;
use tiny_http::{Header, Response, Server};

use crate::require_env;

const REDIRECT_URI: &str = "http://localhost:5555/callback";
const LISTEN_ADDRESS: &str = "127.0.0.1:5555";
const SCOPE: &str = "openid profile w_member_social";
const SECONDS_PER_DAY: u64 = 86_400;

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
    refresh_token_expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct UserInfo {
    sub: String,
    name: String,
}

pub fn run() -> Result<()> {
    let (Ok(client_id), Ok(client_secret)) = (require_env("LINKEDIN_CLIENT_ID"), require_env("LINKEDIN_CLIENT_SECRET"))
    else {
        bail!("Renseigne LINKEDIN_CLIENT_ID et LINKEDIN_CLIENT_SECRET dans .env");
    };

    // Valeur anti-CSRF : RandomState est initialisé aléatoirement par la bibliothèque standard.
    let state = format!("{:x}", RandomState::new().build_hasher().finish());
    let authorize_url = Url::parse_with_params(
        "https://www.linkedin.com/oauth/v2/authorization",
        &[
            ("response_type", "code"),
            ("client_id", client_id.as_str()),
            ("redirect_uri", REDIRECT_URI),
            ("state", state.as_str()),
            ("scope", SCOPE),
        ],
    )?;
    println!("\n1. Ouvre cette URL dans ton navigateur :\n\n{authorize_url}");
    println!("\n2. Accepte les permissions. Tu seras redirigé vers localhost.\n");

    let server = Server::http(LISTEN_ADDRESS).map_err(|error| anyhow::anyhow!("{error}"))?;
    println!("Serveur d'écoute du callback : http://localhost:5555\n");

    for request in server.incoming_requests() {
        if !request.url().starts_with("/callback") {
            request.respond(Response::empty(404))?;
            continue;
        }
        let callback = Url::parse(&format!("http://localhost{}", request.url()))?;
        let param = |name: &str| callback.query_pairs().find(|(key, _)| key == name).map(|(_, value)| value.into_owned());

        if let Some(error) = param("error") {
            request.respond(Response::from_string(format!("Erreur LinkedIn : {error}")).with_status_code(400))?;
            bail!("Erreur : {error} — {}", param("error_description").unwrap_or_default());
        }
        let (Some(code), true) = (param("code"), param("state").as_deref() == Some(state.as_str())) else {
            request.respond(Response::from_string("State mismatch ou code manquant").with_status_code(400))?;
            bail!("State mismatch");
        };

        return match exchange_code(&code, &client_id, &client_secret) {
            Ok((token, user)) => {
                let html = Header::from_bytes("Content-Type", "text/html; charset=utf-8").expect("en-tête valide");
                request.respond(
                    Response::from_string("<h1>OK</h1><p>Tu peux fermer cette page, retourne au terminal.</p>")
                        .with_header(html),
                )?;
                print_secrets(&token, &user);
                Ok(())
            }
            Err(error) => {
                request.respond(Response::from_string(format!("Erreur : {error:#}")).with_status_code(500))?;
                Err(error)
            }
        };
    }
    bail!("serveur de callback arrêté sans réponse")
}

fn exchange_code(code: &str, client_id: &str, client_secret: &str) -> Result<(TokenResponse, UserInfo)> {
    let client = Client::new();
    let response = client
        .post("https://www.linkedin.com/oauth/v2/accessToken")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ])
        .send()?;
    if !response.status().is_success() {
        bail!("token {}: {}", response.status().as_u16(), response.text().unwrap_or_default());
    }
    let token: TokenResponse = response.json().context("réponse token illisible")?;

    let response = client.get("https://api.linkedin.com/v2/userinfo").bearer_auth(&token.access_token).send()?;
    if !response.status().is_success() {
        bail!("userinfo {}: {}", response.status().as_u16(), response.text().unwrap_or_default());
    }
    let user: UserInfo = response.json().context("réponse userinfo illisible")?;
    Ok((token, user))
}

fn print_secrets(token: &TokenResponse, user: &UserInfo) {
    let expires_at = SystemTime::now() + Duration::from_secs(token.expires_in);
    let expires_at = site::date::format_iso8601(expires_at.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default());
    println!("\n✓ Token obtenu avec succès\n");
    println!("Utilisateur : {}", user.name);
    println!("Expire le   : {expires_at}");
    println!("\n--- À copier dans les secrets GitHub ---\n");
    println!("LINKEDIN_ACCESS_TOKEN={}", token.access_token);
    println!("LINKEDIN_USER_URN=urn:li:person:{}", user.sub);
    match &token.refresh_token {
        Some(refresh_token) => {
            let refresh_days = token.refresh_token_expires_in.unwrap_or(0) / SECONDS_PER_DAY;
            println!("LINKEDIN_REFRESH_TOKEN={refresh_token}");
            println!("\n(+ LINKEDIN_CLIENT_ID et LINKEDIN_CLIENT_SECRET dans les secrets)");
            println!("\n----------------------------------------\n");
            println!("✓ Refresh token obtenu (valide ~{refresh_days} jours).");
            println!("   Le crossposter renouvellera l'access token tout seul à chaque run.");
            println!("   Plus rien à faire avant ~1 an (relance cette commande à l'expiration du refresh).\n");
        }
        None => {
            println!("\n----------------------------------------\n");
            println!("⚠️  Aucun refresh_token renvoyé : ton app LinkedIn n'est pas habilitée aux refresh tokens.");
            println!(
                "   → renouvellement MANUEL de LINKEDIN_ACCESS_TOKEN tous les ~{} jours (relance cette commande).\n",
                token.expires_in / SECONDS_PER_DAY
            );
        }
    }
}
