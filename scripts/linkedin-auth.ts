/**
 * Obtient un access token LinkedIn via OAuth 2.0 authorization code flow.
 * À exécuter une seule fois en local pour récupérer le token à coller dans
 * les secrets GitHub (LINKEDIN_ACCESS_TOKEN, LINKEDIN_USER_URN).
 *
 * Prérequis : avoir créé une app sur https://www.linkedin.com/developers/apps
 * avec :
 *   - Product : "Share on LinkedIn" (ajoute w_member_social)
 *   - Product : "Sign In with LinkedIn using OpenID Connect" (ajoute openid + profile)
 *   - Redirect URL : http://localhost:5555/callback
 *
 * Variables d'environnement nécessaires (via .env ou export) :
 *   LINKEDIN_CLIENT_ID, LINKEDIN_CLIENT_SECRET
 *
 * Usage : npm run linkedin-auth
 */

import { createServer } from "node:http";

const CLIENT_ID = process.env.LINKEDIN_CLIENT_ID;
const CLIENT_SECRET = process.env.LINKEDIN_CLIENT_SECRET;
const REDIRECT_URI = "http://localhost:5555/callback";
const PORT = 5555;
const SCOPE = "openid profile w_member_social";

if (!CLIENT_ID || !CLIENT_SECRET) {
  console.error("Renseigne LINKEDIN_CLIENT_ID et LINKEDIN_CLIENT_SECRET dans .env");
  process.exit(1);
}

const state = Math.random().toString(36).slice(2);
const authorizeUrl =
  "https://www.linkedin.com/oauth/v2/authorization?" +
  new URLSearchParams({
    response_type: "code",
    client_id: CLIENT_ID,
    redirect_uri: REDIRECT_URI,
    state,
    scope: SCOPE,
  }).toString();

console.log("\n1. Ouvre cette URL dans ton navigateur :\n");
console.log(authorizeUrl);
console.log("\n2. Accepte les permissions. Tu seras redirigé vers localhost.\n");

const server = createServer(async (req, res) => {
  if (!req.url?.startsWith("/callback")) {
    res.writeHead(404);
    res.end();
    return;
  }
  const url = new URL(req.url, `http://localhost:${PORT}`);
  const code = url.searchParams.get("code");
  const returnedState = url.searchParams.get("state");
  const error = url.searchParams.get("error");

  if (error) {
    res.writeHead(400, { "Content-Type": "text/plain" });
    res.end(`Erreur LinkedIn : ${error}`);
    console.error(`Erreur : ${error} — ${url.searchParams.get("error_description")}`);
    server.close();
    process.exit(1);
  }
  if (!code || returnedState !== state) {
    res.writeHead(400, { "Content-Type": "text/plain" });
    res.end("State mismatch ou code manquant");
    console.error("State mismatch");
    server.close();
    process.exit(1);
  }

  try {
    const tokenResponse = await fetch("https://www.linkedin.com/oauth/v2/accessToken", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        grant_type: "authorization_code",
        code,
        redirect_uri: REDIRECT_URI,
        client_id: CLIENT_ID,
        client_secret: CLIENT_SECRET,
      }).toString(),
    });
    if (!tokenResponse.ok) {
      throw new Error(`token ${tokenResponse.status}: ${await tokenResponse.text()}`);
    }
    const tokenData = (await tokenResponse.json()) as {
      access_token: string;
      expires_in: number;
    };

    const userResponse = await fetch("https://api.linkedin.com/v2/userinfo", {
      headers: { Authorization: `Bearer ${tokenData.access_token}` },
    });
    if (!userResponse.ok) {
      throw new Error(`userinfo ${userResponse.status}: ${await userResponse.text()}`);
    }
    const userData = (await userResponse.json()) as { sub: string; name: string };
    const userUrn = `urn:li:person:${userData.sub}`;

    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    res.end(`<h1>OK</h1><p>Tu peux fermer cette page — retourne au terminal.</p>`);

    const expiryDate = new Date(Date.now() + tokenData.expires_in * 1000);
    console.log("\n✓ Token obtenu avec succès\n");
    console.log(`Utilisateur : ${userData.name}`);
    console.log(`Expire le   : ${expiryDate.toISOString()}`);
    console.log("\n--- À copier dans les secrets GitHub ---\n");
    console.log(`LINKEDIN_ACCESS_TOKEN=${tokenData.access_token}`);
    console.log(`LINKEDIN_USER_URN=${userUrn}`);
    console.log("\n----------------------------------------\n");
    console.log("⚠️  Le token expire dans", Math.round(tokenData.expires_in / 86400), "jours.");
    console.log("   Pense à le renouveler avant cette date (rerun ce script).\n");

    server.close();
    process.exit(0);
  } catch (err) {
    res.writeHead(500, { "Content-Type": "text/plain" });
    res.end(`Erreur : ${err instanceof Error ? err.message : err}`);
    console.error(err);
    server.close();
    process.exit(1);
  }
});

server.listen(PORT, () => {
  console.log(`Serveur d'écoute du callback : http://localhost:${PORT}\n`);
});
