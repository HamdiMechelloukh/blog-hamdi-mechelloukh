Pendant deux mois, j'ai bricolé un petit système qui surveille mon portefeuille et m'envoie, une fois par mois, ce qu'il pense que je devrais faire : acheter, renforcer, alléger, vendre.

Des idées fausses, des bugs qui en cachaient d'autres, des décisions reprises deux ou trois fois. Et au bout du compte, une vision bien plus claire de la façon dont les modèles de langage se comportent réellement, assez loin de ce que j'imaginais au départ.

## Le robot en une phrase

Une fois par mois, un petit script est lancé depuis un serveur. Il récupère la composition de mon portefeuille et les données de marché publiques, demande à une intelligence artificielle d'analyser tout ça, et m'envoie un message Telegram avec ses recommandations.

Le reste de l'article, c'est comment j'en suis arrivé à ce petit script de veille automatisée, en exploitant "correctement" la puissance d'un LLM.

## Acte 1 : l'illusion de l'avantage

À vrai dire, j'avais conscience que je ne pouvais pas avoir de longueur d'avance sur le marché. Mais on veut quand même tester le fantasme du code qui fait gagner à la bourse. Je me disais qu'avec assez de données, un bon modèle et un peu de code, j'allais faire aussi bien que des milliers d'analystes professionnels.

Ça n'a pas tenu longtemps. Tu ne peux battre le consensus avec l'information du consensus, pire, tu feras moins bien qu'un bon vieux "buy and hold sur S&P500".

Du coup, j'ai arrêté de viser l'avantage. La valeur de ce système était ailleurs, rechercher où les signaux sont favorables ou non, voir là où je ne peux pas voir, filtrer ce que je n'ai pas besoin de savoir, me donner des propositions que je pourrais checker, et après me laisser prendre ou ne pas prendre une décision.

L'avantage existe, mais il vient soit de traiter l'information mieux ou plus vite que les autres (tout le métier quant), soit d'en avoir que les autres n'ont pas, et là c'est hors de ma portée, voire un délit d'initié. Moi, à gratter les mêmes chiffres publics que tout le monde, je n'en ai aucun.

Le projet a pris ce virage réaliste après ce constat.

## Acte 2 : le bal des modèles

Petit point de vocabulaire, parce que tout l'article repose dessus.

> **Un LLM (Large Language Model, grand modèle de langage)** est un programme entraîné à prédire le mot suivant. Vous lui donnez un texte, il calcule, pour chaque mot possible, une probabilité d'être la suite, puis il en choisit un, et recommence. ChatGPT, Gemini, Claude sont des LLM. C'est tout ce qu'ils font : prédire la suite, un mot à la fois. Le reste, le raisonnement apparent, les analyses, émerge de cette mécanique répétée des milliards de fois.

Mon robot délègue son jugement à un LLM. J'ai dû changer la réponse 6 fois en 2 mois pour finir par me dire qu'il n'y a pas forcément de bonne réponse, il faut approcher le LLM comme un simple outil comme la plupart des SaaS.

Néanmoins, voici le cheminement que j'ai pris en termes de choix de LLM :

```
avr.    Gemini seul
mai 1   + Claude          (deux modèles en parallèle pour comparer)
mai 9   + Bear            (un 3e modèle, volontairement pessimiste)
                          -> 3 voix, décision au vote
mai 15  STOP. Opus seul   (grand nettoyage, on simplifie tout)
mai 22  retour à Gemini   (raisons de coût et de fonctionnalités)
juin 13 MiMo              (changement des conditions d'utilisation
                           chez Google, et coût)
```

Comme vous pouvez le voir, au démarrage, j'ai commencé à empiler les modèles car j'avais un grand manque de constance dans les réponses du bot, un coup il me disait "achète Microsoft" et un autre "vend Microsoft" même sur 2 runs lancés à la suite.

C'était assez gênant, je cherchais une réponse fiable, la 1ère idée a été, donc, de blinder le bot avec 2 autres runs LLM (Gemini) pour avoir une sorte de consensus et j'ai été jusqu'à ajouter un nouveau modèle Claude.

C'était un peu mieux, mais le bot était agressif, il pouvait me recommander des lignes intéressantes mais avec trop de thèses négatives, j'avais besoin d'un "avocat du diable", d'où l'idée du "Bear", qui était un 3ème agent LLM qui chercherait les thèses qui engendraient des baisses structurelles, et calmerait "l'optimisme" des 2 autres.

C'était bien mais c'était cher, et diablement complexe, quelque chose clochait et c'était lié à l'architecture. J'ai réécrit le bot en me concentrant et en essayant de simplifier les prompts, j'avais encore le problème de constance des réponses.

Après quelques jours, je suis revenu à Gemini car mes coûts sur Claude étaient un peu trop élevés.

J'ai commencé à m'intéresser aux informations que récupérait le LLM, et c'est ce qui m'a fait retirer le grounding search car le LLM est très influencé par le bruit spéculatif.

Au final, je suis passé sur MiMo car très bons résultats sur bench et avec un coût d'utilisation de tokens (prix au token + utilisation de tokens pour arriver à traiter une tâche) battant la concurrence et les changements des CGU pour l'API Gemini m'ont refroidi, quand les 300$ gratuits disparaissent et qu'on reçoit une facture + retrait dans le compte qui équivaut à 3 mois d'utilisation de l'API, ça ne donne plus envie.

## Acte 3 : la mémoire qui rendait le robot paranoïaque

J'avais donné une mémoire au robot. Concrètement, un fichier qui conservait ses analyses passées sur trente jours, réinjectées à chaque nouvelle exécution. L'idée semblait évidente, ajouter du contexte pour que le robot ne me réponde pas de manière aléatoire.

Sauf que la mémoire a créé un biais. Le modèle voyait qu'il avait suggéré de vendre une position la semaine précédente, et cela le poussait à confirmer cette vente, encore et encore, indépendamment des faits nouveaux. Un avis passé devenait une conviction sans prendre en compte tous les fondamentaux. J'ai patché. J'ai ajouté une règle pour retirer les ventes de la mémoire. Ça a déplacé le problème sans le résoudre. J'ai patché encore. Puis j'ai constaté que la mémoire produisait carrément une forme d'auto-censure : le modèle s'alignait sur son passé au lieu de regarder le présent. 

On a eu ce que j'appelle le paradoxe de l'expérience, un peu comme beaucoup de personnes âgées, on se base sur notre expérience pour décider si une décision est bonne ou pas, sauf que le contexte d'une situation change et donc une même décision peut devenir une bonne décision dans un autre contexte, chose que l'expérience efface.

À un moment, j'ai compté les rustines. Quand une fonctionnalité demande correctif sur correctif et que chaque correctif en appelle un autre, **la fonctionnalité elle-même est le bug.** J'ai enlevé la mémoire. Le robot est redevenu *stateless* : sans état, sans mémoire. Chaque mois, il analyse la situation à neuf, comme s'il la découvrait.

Ajouter de l'état (de la mémoire) à un système le rend plus complexe et introduit des dépendances au passé qui peuvent corrompre le présent. L'absence d'état est souvent une fonctionnalité, pas un manque. Pour ceux qui se rappellent leurs cours d'asservissement (automatique continu) : en réinjectant les sorties passées du modèle dans son entrée, je n'avais pas ouvert une boucle, je l'avais refermée, une boucle de rétroaction positive. La sortie se renforçait elle-même, et le système divergeait. Le repasser en stateless, c'est justement revenir en boucle ouverte, chaque run indépendant, sans retour du passé.

Gardez cette histoire de mémoire en tête. Une partie de l'instabilité que je lui attribuais n'était peut-être pas de son fait. On y reviendra un peu plus tard dans l'article.

## Acte 4 : la spéculation n'est pas de l'information

Pour la veille d'actualité, ma première version utilisait ce qu'on appelle le *grounding* (ou recherche augmentée).

> **Le grounding**, c'est quand on autorise le LLM à aller chercher des informations sur le web en temps réel pendant qu'il répond, plutôt que de se fier uniquement à ce qu'il a mémorisé pendant son entraînement.

Sur le papier, parfait : le modèle va lire les dernières nouvelles. En pratique, il rapportait surtout des rumeurs, des spéculations d'analystes, des "la rumeur dit que". Sur un horizon d'un an, ce genre d'information n'en est pas. C'est du bruit déguisé en signal.

Nous sommes encore face à une perturbation, cette fois en entrée : la qualité du signal d'entrée lui-même.

Demi-tour, à nouveau. J'ai coupé le grounding et construit une veille sur des sources vérifiables uniquement : les dépôts réglementaires officiels (aux États-Unis, les documents que les entreprises sont légalement tenues de publier), des flux d'actualité financière établis, et pour le reste du monde, des recherches ciblées. Puis j'ai imposé deux hiérarchies à l'information collectée :

```
HIÉRARCHIE D'AUTORITÉ          ÉCHELLE DE GRAVITÉ
source officielle     >        G3  structurel (change la thèse)
fil de presse établi  >        G2  notable
reste                          G1  anecdotique
```

Le but n'était plus de tout lire, mais de classer. Un dépôt officiel annonçant un changement de direction (G3, source officielle) ne pèse pas comme un billet d'opinion spéculatif (G1, reste). Le filtrage du bruit promis à l'Acte 1 prenait forme.

## Acte 5 : les faits fantômes

Le système collectait désormais des faits vérifiés. Mais les recommandations finales semblaient les ignorer. Pire : elles étaient quasi identiques aux exécutions où la veille avait complètement échoué et n'avait rien remonté du tout. Comme si les faits n'existaient pas.

Ils existaient pourtant. Ils étaient bien présents dans le texte envoyé au modèle. Mais ils étaient regroupés dans un bloc à part, loin de l'endroit du texte où le modèle prenait sa décision sur chaque action. Le modèle les lisait, puis les oubliait au moment de trancher.

Le correctif, une fois le diagnostic posé, était bête : placer chaque fait juste à côté du nom de l'action qu'il concerne, au moment précis où le modèle statue sur cette action. La leçon, elle, est profonde :

**Avec un LLM, la position d'une information compte autant que sa présence.** Une donnée présente dans le contexte mais mal placée par rapport au point de décision est, en pratique, une donnée absente, surtout lorsque le contexte est long.

Corollaire que j'ai inscrit dans le code dans la foulée : **si la couche de faits échoue, le système plante.** Il n'envoie pas un rapport dégradé. Un rapport plausible mais creux est plus dangereux qu'une absence de rapport, parce qu'il a l'apparence d'une vraie analyse. Mieux vaut un plantage visible qu'une fausse certitude bien présentée.

## Acte 6 : le principe qui a tout réorganisé

À force de corriger des dérives de comportement, j'ai fini par formuler la règle qui sous-tend tout le reste, et qui est probablement ma plus grande prise de conscience du projet.

**Les consignes données à un LLM doivent être des principes, jamais des règles chiffrées. Le déterminisme doit vivre dans la donnée, pas dans le texte de la consigne.**

> **Déterministe** veut dire : qui donne toujours le même résultat à partir des mêmes entrées. Un calcul est déterministe. Un jugement humain ne l'est pas.

Concrètement, je m'interdisais désormais d'écrire dans les instructions des choses comme "vise environ 20 % sur cette position" ou "ne fais que ceci". Pourquoi ? Parce qu'un seuil chiffré écrit en langage naturel donne le pire des deux mondes :

- il rend le modèle **rigide** là où je voulais du jugement nuancé ;
- et il lui tend un nombre auquel **s'accrocher** et autour duquel **inventer** (les LLM ont une fâcheuse tendance à broder autour des chiffres qu'on leur donne, car leur réponse est une estimation de la meilleure réponse à donner mais pas la meilleure réponse à donner).

Si je veux du déterminisme, il doit être en amont, dans le tuyau qui prépare la donnée. Le schéma mental est devenu celui-ci :

```
   AMONT : DÉTERMINISTE                AVAL : NON DÉTERMINISTE
   (du code, des chiffres)             (le jugement du LLM)
 ┌────────────────────────────┐     ┌──────────────────────────┐
 │ portefeuille en valeur      │     │ pèse le pour et le contre│
 │ univers filtré et noté      │ ──> │ arbitre entre les noms   │
 │ faits vérifiés et datés     │     │ rédige des cibles        │
 │ fiabilité du consensus      │     │ selon des PRINCIPES      │
 └────────────────────────────┘     └──────────────────────────┘
   les nombres contraignent            aucun nombre magique
   (calculés, vérifiables)             dans la consigne
```

Tout ce qui peut être calculé proprement est calculé en amont, en code, de façon vérifiable, et fourni au modèle comme une contrainte chiffrée. Le modèle, lui, reçoit des principes ("privilégie les convictions fortes", "un consensus fiable vaut mieux qu'un objectif élevé mais incertain") et juge.

Dernière règle du même esprit, sur la rédaction des consignes : **une règle = un énoncé, dit une seule fois.** Deux règles presque identiques sont pires qu'une seule, parce que le lecteur (et un LLM plus encore) cherche la différence entre les deux. Comme elle n'existe pas, il l'invente. Réécrire pour condenser n'est pas cosmétique, c'est réduire la surface d'erreur.

## Acte 7 : la découverte, le bruit était là depuis le début

Arrive le passage à MiMo. Et avec lui, non pas un nouveau problème, mais la révélation d'un problème ancien que je n'avais jamais vu. Pour le comprendre, trois définitions, en escalier.

> **1. La distribution de probabilités.** À chaque mot, le LLM ne choisit pas "le" mot suivant. Il calcule une probabilité pour chaque mot possible. Par exemple, après "le chat boit du", il pourrait évaluer : *lait* 70 %, *eau* 25 %, *café* 4 %, etc.

> **2. La température.** C'est le bouton qui règle le hasard de la sélection. Température haute : le modèle pioche parfois des options peu probables (plus créatif, plus imprévisible). Température zéro : il prend systématiquement l'option la plus probable. *lait*, toujours.

> **3. Les logits.** Ce sont les scores bruts que le modèle calcule pour chaque mot avant de les transformer en probabilités. C'est le matériau de base de la décision.

À température zéro, le modèle prend toujours le plus probable, donc avec les mêmes données en entrée, il devrait produire exactement la même sortie. Déterministe. En réalité, j'avais mal compris ce qu'est un LLM. Il donne l'illusion d'une réponse exacte, alors qu'en fait il donne surtout une réponse qui "fait l'affaire". Quand tu codes en t'aidant d'un LLM, par exemple, il te sort du code fonctionnel, mais pas forcément du bon, ou plutôt : pas à tous les coups.

En changeant de modèle, j'ai voulu pour la première fois mesurer la stabilité avant de bâtir dessus. J'ai lancé **le même prompt, sur les mêmes données figées, plusieurs fois de suite, à température zéro.** J'attendais des sorties identiques.

J'ai eu le contraire. Une variance considérable d'un essai à l'autre. Quelques chiffres réels de ce test sur une trentaine de positions :

- la somme des allocations cibles proposées valait **44 %** au premier essai, **105 %** au deuxième, **101 %** au troisième ;
- sur la trentaine de noms, **6 seulement** recevaient une recommandation stable d'un essai à l'autre ;
- un essai sur trois partait carrément en vrille.

Même entrée, même température zéro, sorties très différentes.

**Ce bruit n'était pas une nouveauté de MiMo.** Il était là depuis le début, avec Gemini, puis Claude, puis tous les modèles que j'avais utilisés. J'ai essayé de le résoudre avec des instructions déterministes car j'avais un besoin de contrôle, mais c'est un non-sens, je ne peux à la fois avoir le contrôle (déterministe) et que l'IA me trouve des insights (non-déterministe). MiMo n'a rien apporté de spécial sur ce plan ; il a juste été l'occasion où j'ai compris la limite entre l'utilisation ou non du LLM, car ce n'est juste qu'un outil, avec, effectivement, un niveau d'abstraction élevé, mais pas une solution ou un humain de remplacement, même si certaines entreprises font un très beau marketing pour dire le contraire.

Et là, l'Acte 3 prend un autre sens. L'instabilité que j'avais mise sur le dos de la mémoire, ces recommandations qui valsaient d'un mois sur l'autre que je combattais à coups de rustines, une bonne partie n'était sans doute pas la mémoire du tout. C'était déjà ce bruit-là, invisible faute d'être mesuré. 

Attention toutefois à ne pas réécrire l'histoire trop proprement : le biais d'ancrage de la mémoire était bien réel, réinjecter ses propres ventes passées crée un vrai biais de confirmation. Il y avait donc deux problèmes superposés, pas un seul mal diagnostiqué. La mémoire n'était pas innocente ; elle avait simplement un complice invisible que je ne mesurais pas.

### Pourquoi un LLM reste bruité même à température zéro

Voici le cœur, et c'est plus subtil qu'il n'y paraît.

À température zéro, le choix du mot n'est **pas** aléatoire. Le modèle prend le maximum, de façon parfaitement déterministe. **Ce n'est donc pas la règle de sélection qui injecte du hasard.**

Le bruit vient d'un cran plus tôt : **les logits eux-mêmes ne tombent pas toujours sur le même chiffre.** Et la cause n'est pas celle qu'on croit. L'explication courante ("les calculs en parallèle sur le processeur graphique s'exécutent dans un ordre aléatoire") est trompeuse, et c'est précisément ce que corrige le travail cité juste après : pour une configuration de calcul donnée, le modèle est en fait reproductible ; relancé à l'identique, il redonne les mêmes logits.

Le vrai coupable, c'est le regroupement par **lot** (batch). Sur un serveur, ta requête n'est jamais traitée seule : elle est regroupée avec d'autres, et la composition de ce groupe (combien de requêtes, de quelles longueurs) change à chaque appel selon la charge. Or, pour aller vite, le processeur graphique découpe et additionne les nombres dans un ordre qui dépend de la forme du lot. Et l'addition en virgule flottante (l'arithmétique approximative des ordinateurs sur les décimales) n'est pas associative : `(a + b) + c` ne donne pas exactement `a + (b + c)`. Donc des voisins de lot différents entraînent un découpage différent, donc un ordre d'addition différent, donc des logits qui bougent d'un cheveu. Chaque calcul pris isolément est déterministe ; c'est le contexte de lot qui varie d'un appel à l'autre.

Ce n'est d'ailleurs pas une fatalité absolue. [Des travaux récents de Thinking Machines Lab](https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/) ont montré qu'en réécrivant ces calculs pour qu'ils additionnent toujours dans le même ordre, quelle que soit la composition du lot, on peut rendre un modèle parfaitement reproductible à température zéro : dans leur démonstration, 1000 générations devenaient bit-pour-bit identiques, là où la version standard en produisait 80 différentes. Le prix est un ralentissement (de l'ordre de 1,6 à 2 fois selon les noyaux), et c'est pour ça que les API grand public ne l'activent pas par défaut. Le bruit à température zéro est donc une fatalité en pratique, sur les API courantes, pas en principe.

Tant que le meilleur candidat gagne largement, aucune importance. Mais quand deux candidats sont quasi à égalité, ce cheveu fait basculer le classement :

```
Scores bruts (logits) pour le mot suivant

  alléger   ████████████████████  8.40
  vendre    ███████████████████▉  8.39   <- quasi à égalité !
  garder    ██████████            4.10

Essai 1 :  alléger 8.401 , vendre 8.399  ->  on choisit ALLÉGER
Essai 2 :  alléger 8.398 , vendre 8.402  ->  on choisit VENDRE
                          ^ micro-écart flottant, et tout bascule
```

Et sur un modèle qui "raisonne" (qui génère une longue chaîne de réflexion avant de conclure), un seul basculement précoce se propage et s'amplifie tout au long du raisonnement. Une virgule de différence au début, une conclusion opposée à la fin.

La bonne formulation est donc :

> Ce n'est pas bruité parce que le choix est aléatoire, mais parce que **les valeurs estimées sur lesquelles s'appuie un choix déterministe sont elles-mêmes instables.** Un choix déterministe sur des estimations instables redevient instable près des égalités.

Et ce détail change toute l'interprétation. **Le bruit n'est pas aveugle.** Il se concentre exactement sur les cas serrés, ceux où le modèle est lui-même indécis. Une position où la conviction est nette ne bascule jamais (le meilleur candidat gagne largement). Les positions qui valsent d'un essai à l'autre sont précisément celles où deux options se valent presque.

**Le bruit marque les zones d'incertitude réelle.** Ce n'est pas un défaut à masquer. C'est une information.

## Acte 8 : ne pas combattre le bruit, le faire voter

Si le bruit est une information sur l'incertitude, la bonne réponse n'est pas de l'éliminer. C'est de l'agréger.

Plutôt qu'un seul essai, j'en lance plusieurs sur les mêmes données, puis je fais voter les résultats. C'est exactement l'idée du **théorème du jury de Condorcet**, énoncé par un mathématicien français du XVIIIe siècle.

> **Théorème de Condorcet (en quelques mots).** Si chaque juré a une probabilité supérieure à 50 % de trouver la bonne réponse, et que les jurés se trompent indépendamment les uns des autres, alors plus on ajoute de jurés, plus le vote majoritaire tend vers la certitude d'avoir raison.

En formule, la probabilité que la majorité de N jurés ait raison, chacun correct avec probabilité p :

```
                N
P(majorité) =   Σ    C(N,k) · p^k · (1 − p)^(N−k)
              k=⌊N/2⌋+1
```

Ce qu'il faut en retenir sans les symboles :

```
  p (qualité d'un juré)      vote majoritaire quand N grandit
  ─────────────────────      ───────────────────────────────────────
  p > 0,5  (mieux que pile)  ──> tend vers 1   (certitude d'avoir raison)
  p = 0,5  (pile ou face)    ──> reste à 0,5   (le vote n'aide pas)
  p < 0,5  (pire que pile)   ──> tend vers 0   (le vote empire tout !)
```

Attention au piège que la formule rend visible : le vote n'améliore les choses **que si chaque juré est déjà meilleur que le hasard.** Si le modèle est mauvais sur une question, multiplier les essais ne fait qu'amplifier l'erreur. Le vote fiabilise un juré correct mais bruité ; il ne sauve pas un juré incompétent.

Et il y a un second piège, plus pernicieux. Le théorème de Condorcet a **deux** hypothèses, pas une : des jurés meilleurs que le hasard (je viens d'en parler), et des erreurs **indépendantes**. Or relancer le même modèle cinq fois, c'est cinq fois le même réseau, les mêmes biais, le même raisonnement type. Le bruit flottant ne décorrèle les sorties que près des égalités, justement là où je veux qu'elles votent. Mais sur une erreur systématique (le modèle ne comprend pas un secteur, surévalue une thèse), les cinq essais se trompent ensemble, et pire : ils se trompent **à l'unanimité**. Car un vote unanime n'est qu'une **réponse constante**, et une réponse constante n'est que le **renfort de la thèse du modèle**, pas une preuve qu'elle est juste. Le 5/5 mesure la stabilité, jamais la vérité. Pour trancher un consensus, il faut donc une source **hors du modèle** ; un même modèle relancé ne fera que répéter sa thèse avec aplomb. Le vote neutralise le bruit d'échantillonnage ; il ne corrige pas le biais du modèle.

Mon modèle, sur la plupart des positions, est bien meilleur que le hasard, juste bruité près des cas serrés. Cas d'usage idéal pour Condorcet. J'ai affiné le vote sur deux niveaux : d'abord la **direction** (faut-il monter ou baisser cette position ?), puis seulement le **degré**. Sans ça, un consensus clair sur la direction pouvait être enterré par des étiquettes d'action légèrement différentes ("alléger" et "vendre" disent tous deux : baisser).

Le résultat est exactement ce que je cherchais depuis l'Acte 1 sans le savoir :

```
  Action A   vendre 5/5    ->  stable : en tête de la carte (à valider sur les faits)
  Action B   renforcer 4/5 ->  stable : consensus
  Action C   acheter 2 / alléger 2 / garder 1
                           ->  PAS de consensus : affiché comme "partagé,
                               à ton jugement de trancher"
```

Les cas francs ressortent par consensus, les cas serrés apparaissent **honnêtement** comme partagés, et c'est moi qui tranche. Mais attention à ne pas relire ce tableau comme un palmarès de bonnes recommandations : après tout ce qui précède, un "5/5" ne certifie pas que vendre soit le bon coup, il certifie que le modèle est **stable** là-dessus. Ce que produit vraiment l'ensemble, ce n'est pas une liste d'ordres, c'est une **carte de stabilité** : voici où mon jugement est le moins requis, et voici où il l'est le plus. Le consensus me dit où regarder en confiance ; ce sont les faits primaires et moi qui validons le fond. Le système cesse de feindre une certitude qu'il n'a pas.

**J'avais commencé par faire voter trois modèles différents, puis j'avais tout supprimé pour simplifier. Je finis par faire voter un seul modèle plusieurs fois.** La structure est la même (un vote), mais la raison a changé du tout au tout : au début je votais pour combiner des points de vue différents ; à la fin je vote pour neutraliser le bruit d'un même modèle. Il m'aura fallu deux mois et un détour par toute la chaîne pour comprendre la vraie raison d'être de ce vote.

Reste l'objection : la version vraiment fidèle à Condorcet, ce serait plusieurs modèles différents, chacun lancé plusieurs fois, parce que des architectures différentes se trompent de façon plus décorrélée. Et soyons honnête : le gain est **réel**, pas incertain. Un ensemble de modèles différents décorrèle vraiment les erreurs, c'est la raison d'être des ensembles depuis toujours. Simplement, il n'est probablement pas **rentable**. Chaque modèle de plus, c'est un fournisseur à maintenir, à payer, à surveiller, des formats et des prompts à garder synchrones, pour un bénéfice qui devient marginal au regard de ce coût d'exploitation. On quitte les 80 % utiles de Pareto. C'est un arbitrage de coût, pas un déni du bénéfice. J'ai donc tranché autrement. Le vote d'un seul modèle tue le bruit, ce qui est l'essentiel et presque gratuit. Et pour le biais, la source hors du modèle dont j'ai besoin, je l'ai déjà : les faits primaires auxquels je confronte le modèle (Actes 4 et 5), et mon propre jugement sur les cas partagés. Le 5/5 me dit où le modèle est stable ; les faits et moi disons s'il a raison.

## La question que cet article esquive

Tout ce qui précède parle de **cohérence** : le système est-il stable, honnête sur ses doutes ? Mais cohérence n'est pas **justesse**. Un système peut être parfaitement stable, parfaitement lucide sur ses zones d'incertitude, et médiocre en rendement. La cohérence parfaite, c'est même, on vient de le voir, exactement ce à quoi ressemble un préjugé répété sans broncher.

J'ai moi-même démonté l'idée d'avantage dès l'Acte 1 : je n'ai aucune raison sérieuse de battre le marché. Reste alors une tension que je ne résous pas vraiment : à quoi bon des allocations si soignées si rien ne garantit qu'elles valent mieux qu'un simple fonds indiciel ? Ma réponse honnête : ce n'est pas un outil de performance, c'est un outil de veille et d'aide à la décision. Les cibles qu'il produit sont un point de départ pour mon jugement, pas un pilote automatique. Il est encore trop tôt pour mesurer si je bats un indice sur la durée. Pour le moment, je suis très proche du S&P 500 et sous le NASDAQ, sur 2-3 mois de développement, ce qui ne prouve rien, ni dans un sens ni dans l'autre : sur ce laps de temps, tout est noyé sous le bruit de marché. Distinguer la compétence de la chance en allocation actions demande des années, et idéalement de traverser une vraie baisse. Je n'aurai donc pas la réponse avant longtemps, maintenant que le bot est stable. 

J'ai écrit cet article pour expliquer comment j'ai rendu une machine cohérente dans ses réponses et comment j'ai gagné en lucidité sur ses limites, sans prétendre qu'elle a raison.

## Ce que je cherchais vraiment

J'ai commencé en voulant une machine qui ait raison. J'ai fini avec une machine honnête sur ce qu'elle ne sait pas. Et c'est bien plus utile.

Après avoir démonté mes illusions une à une, voici ce qui reste, et qui suffit à justifier la machine : une veille, avec un tamis fiable contre le bruit de l'information, pour voir ce qui se passe d'important sur le marché et sur mon portefeuille. Aucune promesse de battre le marché. Mais à la seule tâche pour laquelle je l'ai bâtie, récolter la bonne information et écarter le reste, elle est sans hésiter meilleure que moi.

La leçon la plus transférable dépasse de loin la finance :

> **Un LLM n'est pas un oracle, c'est un échantillonneur.** Il tire ses réponses dans une distribution de probabilités. Sa variance n'est pas un défaut à cacher, c'est une mesure de sa propre confiance. Les bons systèmes construits autour des LLM ne masquent pas l'incertitude, ils la font remonter à la surface.

Le robot tourne une fois par mois et m'envoie ses conclusions. Mais ce qu'il m'a vraiment donné, ce n'est pas une liste d'achats. C'est une façon plus nette de penser la décision sous incertitude, et un grand respect pour la différence entre un système qui répond, et un système qui sait quand il ne sait pas.
