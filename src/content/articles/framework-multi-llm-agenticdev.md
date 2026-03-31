Fin 2025, après avoir passé des heures à prompter des LLMs un par un pour générer du code, une question me trottait en tête : **et si plusieurs agents LLM pouvaient collaborer entre eux pour produire un projet complet ?** Non pas un seul agent qui fait tout, mais une équipe spécialisée (un architecte, un développeur, un testeur), chacun avec son rôle, ses outils, et ses contraintes.

C'est comme ça qu'est né **AgenticDev**, un framework Python qui orchestre 4 agents LLM pour transformer une simple demande en texte libre en code testé et documenté.

Dans cet article, je partage les choix d'architecture, les problèmes rencontrés, et les leçons apprises.

## Le point de départ : tester les limites de la collaboration multi-agents

Mon objectif initial était simple : explorer jusqu'où des agents LLM peuvent collaborer de manière autonome. Pas un POC jetable, mais un vrai pipeline où chaque agent a une responsabilité claire :

- **Architect** — analyse la demande et produit une spécification technique (`spec.md`)
- **Designer** — génère des assets SVG à partir de la spec
- **Developer** — implémente le code en suivant la spec et en intégrant les assets
- **Tester** — écrit et exécute les tests, puis renvoie les échecs au Developer

L'idée, c'est le pattern **Agent as Tool** : chaque agent est un nœud dans un graphe d'exécution, pas un LLM qui appelle d'autres LLMs de manière chaotique.

## Architecture : pourquoi LangGraph plutôt qu'un orchestrateur LLM

Ma première approche était de laisser un agent orchestrateur (Gemini) décider dynamiquement quel sous-agent appeler, via des function calls. Ça fonctionnait, mais j'ai vite identifié un problème : **plus le système était générique, plus il devenait imprévisible.**

L'orchestrateur LLM pouvait décider de sauter le Designer, d'appeler le Tester avant le Developer, ou de boucler indéfiniment. Pour un framework qui doit produire du code fiable, c'est rédhibitoire.

J'ai donc fait le choix de **déléguer l'orchestration à LangGraph**, un framework de graphes déterministes. Le pipeline devient explicite :

```
Architect → Designer → Developer → Tester
                                      │
                                      ▼ (tests échouent ?)
                                   Developer ← fix loop (max 3×)
```

Chaque nœud est un agent autonome, mais **l'ordre d'exécution et la logique de retry sont déterministes**. Le LLM garde le contrôle sur le *quoi* (le contenu généré), mais pas sur le *quand* (le flux d'exécution).

```python
_builder = StateGraph(PipelineState)
_builder.add_edge(START, "architect")
_builder.add_edge("architect", "designer")
_builder.add_edge("designer", "developer")
_builder.add_edge("developer", "tester")
_builder.add_conditional_edges(
    "tester",
    should_fix_or_end,
    {"fix": "fix_developer", "end": END},
)
_builder.add_edge("fix_developer", "tester")
```

La fonction `should_fix_or_end` est pure Python : elle parse la sortie du Tester et décide si on relance le Developer ou si on termine. Pas de LLM dans la boucle de décision.

## Le problème du prompt caching et le choix full Gemini

En phase de découverte, j'ai très rapidement atteint les **limites de taux d'utilisation** de l'API Gemini. Chaque appel d'agent envoyait le system prompt complet, les définitions d'outils, le contexte du projet, des milliers de tokens à chaque requête.

La solution : le **prompt caching**. Mais Gemini et Claude le gèrent de manière très différente.

### Gemini : caching implicite

Gemini applique automatiquement un cache sur les préfixes répétés. Si le system prompt et les premières instructions sont identiques entre deux appels, Google réutilise le contexte en cache. Côté code, il n'y a rien à faire : le cache est transparent.

```python
# Les économies apparaissent dans les métadonnées
cached = getattr(meta, "cached_content_token_count", 0)
total = getattr(meta, "prompt_token_count", 0)
logger.info("cache hit: %d/%d tokens (%d%%)", cached, total, cached * 100 // total)
```

### Claude : caching explicite

Claude demande des marqueurs `cache_control: ephemeral` explicites sur les blocs qu'on veut cacher : le system prompt, les définitions d'outils, et le premier message utilisateur.

```python
system = [{
    "type": "text",
    "text": self.instructions,
    "cache_control": {"type": "ephemeral"}
}]

claude_tools = [self._fn_to_claude_tool(fn) for fn in self.tools]
if claude_tools:
    claude_tools[-1]["cache_control"] = {"type": "ephemeral"}
```

### Pourquoi j'ai basculé sur du full Gemini

J'ai commencé avec une architecture multi-LLM : Gemini pour l'Architect et le Tester, Claude pour le Developer. L'idée était séduisante : utiliser chaque LLM là où il excelle.

En pratique, le **coût de l'API Claude a vite rendu cette approche insoutenable**. Un pipeline complet avec Claude comme Developer consommait significativement plus qu'avec Gemini, surtout sur les itérations de fix où le contexte grossit à chaque tour. J'ai donc décidé de passer sur du **full Gemini** pour le pipeline par défaut, tout en gardant le `ClaudeAgent` dans le framework comme option configurable.

Ce choix pragmatique m'a aussi permis de bénéficier pleinement du caching implicite de Gemini sur l'ensemble du pipeline, sans avoir à gérer deux stratégies de caching différentes en production.

Ce contraste entre les deux approches m'a néanmoins poussé à concevoir la hiérarchie de classes pour isoler ces différences :

```
BaseAgent (ABC)
├── GeminiAgent    → caching implicite, google-genai SDK
│   ├── ArchitectAgent
│   ├── DesignerAgent
│   ├── DeveloperAgent
│   └── TesterAgent
└── ClaudeAgent    → caching explicite, anthropic SDK
    └── DeveloperAgent
```

Chaque agent hérite de la stratégie de caching de son backend, sans avoir à s'en soucier.

## Hiérarchie d'agents : ABC et spécialisation

Le cœur du framework repose sur une hiérarchie simple :

- **`BaseAgent`** (ABC) — définit le contrat : `run(context) → AgentResult`, gestion des outils
- **`GeminiAgent`** — implémente la boucle agentique pour Gemini (chat + tool calls)
- **`ClaudeAgent`** — implémente la boucle agentique pour Claude (messages + tool_use blocks)

Les agents spécialisés (Architect, Developer, Tester) héritent de `GeminiAgent` et ne définissent que leurs **instructions** et leurs **outils** :

```python
class ArchitectAgent(GeminiAgent):
    def __init__(self):
        super().__init__(
            name="Architect",
            instructions="You are a software architect...",
            tools=[web_search, write_file],
            model_name="gemini-3.1-pro-preview",
        )
```

Pour ajouter un nouvel agent, il suffit de créer une classe, de définir ses instructions, et de l'ajouter comme nœud dans le graphe LangGraph. Pas besoin de toucher à la logique de chat, de tool calling, ou de caching.

## Le cas particulier du Designer

Le `DesignerAgent` est un cas intéressant. Contrairement aux autres agents qui utilisent la boucle agentique standard (chat → tool call → réponse → tool call → ...), le Designer fait des **appels directs à l'API** pour générer du SVG.

Pourquoi ? Parce que la génération de SVG est un workflow en deux étapes bien défini :

1. **Planning** — "quels assets ce projet a-t-il besoin ?" → retourne un JSON
2. **Génération** — "génère ces N sprites SVG" → retourne du texte parsable

Pas besoin de boucle agentique avec des outils ici. Le Designer hérite quand même de `GeminiAgent` (pour le client API et la validation de clé), mais il **override `run()`** avec sa propre logique.

```python
class DesignerAgent(GeminiAgent):
    def __init__(self, model_name: str = "nano-banana-pro-preview"):
        super().__init__(name="Designer", instructions="", tools=[], model_name=model_name)

    def run(self, context: TaskContext) -> AgentResult:
        # 1. Plan assets from spec
        sprites = self._plan_assets(spec)
        # 2. Generate SVGs
        written = self._generate_sprites(sprites, assets_dir)
        # 3. Write manifest
        ...
```

## La boucle de correction automatique

Un des aspects les plus utiles du pipeline est la **fix loop**. Quand le Tester détecte des échecs, le Developer est relancé en **FIX MODE** :

```python
def should_fix_or_end(state: PipelineState) -> Literal["fix", "end"]:
    if (
        _has_test_failures(state.get("test_results", ""))
        and state.get("fix_iterations", 0) < MAX_FIX_ITERATIONS
    ):
        return "fix"
    return "end"
```

Le Developer reçoit alors la sortie des tests dans son contexte, avec une instruction claire :

> *"You are in FIX MODE — read existing files and fix these. Do NOT rewrite all files from scratch."*

En pratique, 3 itérations suffisent dans la majorité des cas pour passer de 60-70% de tests verts à 100%.

## Les outils partagés

Les agents interagissent avec le système de fichiers via 4 outils simples :

| Outil | Rôle |
|---|---|
| `write_file(path, content)` | Écrire un fichier (crée les dossiers parents) |
| `read_file(path)` | Lire un fichier existant |
| `execute_code(command)` | Exécuter une commande shell |
| `web_search(query)` | Recherche web via DuckDuckGo |

Ces outils sont des fonctions Python classiques, passées aux agents via leur constructeur. Le framework se charge de les exposer au LLM au bon format (Gemini function declarations ou Claude tool definitions).

## Les limites : une base solide, pas un produit fini

Il faut être honnête sur ce que le framework peut et ne peut pas faire. **AgenticDev excelle pour générer une base de projet fonctionnelle** : structure de fichiers, code initial, tests, documentation. Sur des projets simples (CLI tools, bibliothèques, petites APIs), le résultat est souvent utilisable tel quel.

Mais dès que la complexité augmente (logique métier pointue, intégrations multiples, contraintes de performance), **le code généré sera une base de départ, pas le produit final**. Il y aura des limites techniques (architectures trop naïves, edge cases non couverts) et fonctionnelles (le LLM ne connaît pas votre contexte métier) qu'il faudra corriger manuellement ou en vibe-codant avec un outil comme Claude Code ou Cursor.

C'est d'ailleurs le workflow que je recommande : laisser AgenticDev générer le squelette, puis itérer dessus avec un assistant de code pour affiner les détails. Le framework vous fait gagner les premières heures de setup, pas les dernières heures de polish.

## Ce que j'ai appris

### La spécialisation bat la généricité

Un agent "qui fait tout" est moins fiable qu'une équipe d'agents spécialisés. L'Architect ne sait pas coder, le Developer ne sait pas tester, et c'est voulu. Chaque agent a des instructions précises et un périmètre limité.

### L'orchestration déterministe est non négociable

Laisser un LLM décider du flux d'exécution, c'est accepter que le pipeline se comporte différemment à chaque run. Pour un outil de génération de code, c'est inacceptable. LangGraph m'a permis de garder la créativité des LLMs tout en imposant un ordre d'exécution prévisible.

### Le prompt caching est indispensable en multi-agents

Sans caching, un pipeline de 4 agents consomme facilement 100k+ tokens par run, dont 80% sont du contexte répété. Le caching réduit les coûts et les latences de manière significative.

### Le coût dicte l'architecture

Commencer en multi-LLM était intellectuellement satisfaisant, mais la réalité économique m'a rattrapé. Garder l'abstraction multi-backend tout en utilisant un seul provider par défaut est le bon compromis : on ne paie que ce qu'on utilise, sans sacrifier la flexibilité.

### Les instructions d'agent sont du code

Les prompts des agents ne sont pas des phrases vagues : ce sont des spécifications précises avec des règles, des exemples, et des cas limites. Par exemple, le prompt du Developer inclut des règles sur les conventions Python vs TypeScript, la gestion des placeholders, et un audit de complétion obligatoire avant de retourner sa réponse.

## Pour aller plus loin

Le code source est disponible sur [GitHub](https://github.com/HamdiMechelloukh/AgenticDev). Le framework est conçu pour être étendu : ajouter un nouvel agent prend une dizaine de lignes de code.

Les prochaines étapes que j'envisage :
- Support de nouveaux backends LLM (Mistral, Llama)
- Métriques de qualité sur le code généré
- Mode interactif avec validation humaine entre chaque étape
