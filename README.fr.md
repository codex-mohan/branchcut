<div align="center">

<img src="docs/brand/branchcut-icon.svg" alt="Branchcut icon" width="112">

# Branchcut

**Compilez la requête. Élaguez l’arborescence.**

<p>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2024-DEA584?style=for-the-badge&amp;logo=rust&amp;logoColor=white&amp;labelColor=0A1220" alt="Rust 2024" /></a>
  <img src="https://img.shields.io/badge/core_crates-zero-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="Zero third-party crates in the core" />
  <a href="https://github.com/codex-mohan/branchcut/actions/workflows/install-matrix.yml"><img src="https://img.shields.io/github/actions/workflow/status/codex-mohan/branchcut/install-matrix.yml?style=for-the-badge&amp;label=installers&amp;labelColor=0A1220" alt="Installer checks" /></a>
  <a href="https://github.com/codex-mohan/branchcut/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-BB86FC?style=for-the-badge&amp;labelColor=0A1220" alt="MIT license" /></a>
  <a href="https://github.com/codex-mohan/branchcut/stargazers"><img src="https://img.shields.io/github/stars/codex-mohan/branchcut?style=for-the-badge&amp;labelColor=0A1220&amp;color=FFD700" alt="GitHub stars" /></a>
  <a href="https://github.com/codex-mohan/branchcut/pulls"><img src="https://img.shields.io/badge/PRs-welcome-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="PRs welcome" /></a>
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="README.es.md">Español</a> ·
  <strong>Français</strong> ·
  <a href="README.zh-CN.md">简体中文</a> ·
  <a href="README.ja.md">日本語</a> ·
  <a href="README.hi.md">हिन्दी</a>
</p>

</div>

Branchcut est un moteur natif de requêtes sur le système de fichiers, conçu pour les recherches courantes, le développement et les scripts. Il compile les motifs, les exclusions, les règles qui indiquent les chemins à ignorer, les filtres d’entrées et les conditions d’arrêt en un seul plan de parcours. Il peut écarter les répertoires qui ne peuvent fournir aucun résultat, émettre les chemins correspondants à mesure qu’il les trouve et expliquer le travail effectué.

La bibliothèque et l’interface en ligne de commande (CLI) sont organisées en modules Rust aux rôles distincts. Le moteur et la CLI standard utilisent uniquement la bibliothèque standard de Rust. Le graphe des dépendances à l’exécution ne contient aucune crate tierce.

[Site et documentation](https://codex-mohan.github.io/branchcut/) · [Versions](https://github.com/codex-mohan/branchcut/releases) · [Liste des fonctionnalités](docs/reference/feature-status.md) · [Signaler un bogue](https://github.com/codex-mohan/branchcut/issues)

## Installation

Téléchargez l’archive correspondant à votre système depuis les [versions sur GitHub](https://github.com/codex-mohan/branchcut/releases/latest), vérifiez-la à l’aide de `SHA256SUMS`, extrayez-la et placez l’exécutable dans un répertoire de votre `PATH`. Les binaires précompilés fonctionnent sans Rust ni Cargo. Consultez les [instructions d’installation à partir d’une archive](docs/contributing/releases.md#verify-and-install-an-archive).

Les scripts d’installation peuvent installer Rust si Cargo est absent. L’installation directe avec Cargo nécessite une chaîne d’outils Rust déjà installée. Chacune de ces méthodes compile Branchcut à partir du code source.

**Windows PowerShell**

```powershell
irm https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.ps1 | iex
```

**Linux et macOS**

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.sh | sh
```

**Cargo**

```sh
cargo install --locked --git https://github.com/codex-mohan/branchcut.git
```

Vérifiez que la commande est disponible.

```sh
branchcut --version
branchcut --help
```

Les scripts installent un binaire compilé en mode release. Consultez le [guide d’installation](docs/getting-started/installation.md) pour choisir un emplacement, désinstaller Branchcut ou résoudre les problèmes de `PATH`.

## Utilisation

Recherchez les fichiers source Rust dans l’arborescence du répertoire courant.

```sh
branchcut --glob 'src/**/*.rs'
```

Le texte simple est recherché littéralement dans les noms de fichiers.

```sh
branchcut config
```

Combinez des motifs, des exclusions, des filtres de fichiers et une limite de résultats.

```sh
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --type file \
  --limit 100
```

Consultez le plan et les compteurs relevés pendant le parcours.

```sh
branchcut --glob '**/*.rs' --explain
branchcut --glob '**/*.rs' --stats
```

### Recherches courantes et requêtes enregistrées

Recherchez les fichiers source et de configuration courants en appliquant les règles qui indiquent les chemins à ignorer et en incluant les chemins masqués.

```sh
branchcut --glob '**/*.{rs,toml}' --ignore --hidden
```

Recherchez par taille et date de modification.

```sh
branchcut --glob '**/*' --size '>=1.5MiB' --modified-within 30d
branchcut --glob '**/*.log' --modified-before '2025-01-01T12:00:00+05:30'
```

Enregistrez les sélections réutilisables d’un projet dans `.branchcut-queries`.

```text
[workspace]
glob packages/**/src/**/*.{rs,ts}
exclude **/target/**
ignore true
max-depth 8
```

Exécutez ensuite le profil.

```sh
branchcut --config .branchcut-queries --profile workspace --explain
```

Vous trouverez d’autres exemples dans les guides [Recherches courantes](docs/guides/everyday-search.md), [Chemins à ignorer et chemins masqués](docs/guides/ignore-and-hidden.md) et [Requêtes de projet enregistrées](docs/guides/project-queries.md).

### Pipelines et commandes

Utilisez des chemins délimités par NUL pour transmettre des noms de fichiers Unix quelconques à un autre programme.

```sh
branchcut --glob '**/*.rs' --print0 | xargs -0 rustfmt
```

Utilisez JSON Lines si les outils en aval ont besoin de chemins structurés, ou ajoutez des informations sur le système de fichiers avec `--json-metadata`.

```sh
branchcut --glob '**/*.log' --json-metadata
```

Exécutez une commande en passant chaque résultat comme argument, sans passer le chemin par un interpréteur de commandes.

```sh
branchcut --glob '**/*.tmp' --exec 'rm {}'
```

Pour les grandes sélections, `--exec-batch` regroupe les chemins en lots de taille limitée. Utilisez `--exec-preview` pour consulter les arguments (`argv`) sans exécuter les commandes. La page [Sortie et exécution](docs/guides/output-and-exec.md) décrit les paramètres de substitution, les particularités des plateformes et le comportement en cas d’échec.

## Fonctionnalités

- **Recherche par motifs glob.** `*`, `?`, `**`, classes de caractères, accolades imbriquées, plages bornées et extglobs limités à un composant du chemin.
- **Requêtes composées.** Plusieurs motifs positifs, exclusions, sélection par extension et par type, profondeur, termes littéraux et filtres de métadonnées dans une même requête.
- **Recherche dans le contenu.** Recherche optionnelle de texte littéral dans les fichiers sélectionnés avec `--content` (alias `--grep`), plusieurs motifs combinés avec OR, sortie `chemin:ligne:colonne:texte`, `-l`, `--count`, modes de casse, omission automatique des binaires avec `--text` et arrêt anticipé. Pas d’expressions régulières ni de modes multilignes.
- **Chemins à ignorer.** Les chemins masqués sont exclus par défaut. Vous pouvez activer les sources de règles Git pour les chemins à ignorer, les fichiers `.ignore` ou des fichiers de règles indiqués explicitement.
- **Options de parcours.** Sortie au fil du parcours, `--first`, `--limit`, tri, choix du nombre de threads de travail, suivi des liens symboliques, parcours limité à un seul système de fichiers sous Unix et limitation du nombre de répertoires ouverts.
- **Filtres de métadonnées.** Seuils exacts de taille de fichier en unités décimales ou binaires et intervalles de date de modification. Les métadonnées sont lues uniquement si la requête l’exige.
- **Requêtes réutilisables.** Profils nommés pour les requêtes d’un projet, dans un petit fichier de configuration texte.
- **Diagnostic.** `--explain`, `--why PATH` et `--stats` affichent le plan, la décision prise pour un chemin donné et les mesures du parcours.
- **Sortie et automatisation.** Chemins en texte brut, JSON Lines avec métadonnées facultatives, chemins délimités par NUL, formatage, couleurs, liens hypertextes, exécution de commandes sans interpréteur, lots de taille limitée et complétion dans le shell.
- **API de la bibliothèque.** Compilez une `Query` une seule fois et parcourez les résultats avec la bibliothèque Rust sans invoquer la CLI.

Consultez la [référence de la CLI](docs/reference/cli.md), la [syntaxe des motifs glob](docs/reference/glob-syntax.md) et l’[état des fonctionnalités](docs/reference/feature-status.md) pour connaître la syntaxe, les limites et l’état de l’implémentation.

## Fonctionnement

Une recherche à plusieurs filtres peut parcourir toute l’arborescence, puis vérifier les motifs glob et les exclusions, lire les métadonnées et collecter les résultats. Branchcut compile ces opérations en un plan de requête et propage les états des motifs encore susceptibles de correspondre au fil du parcours.

```text
patterns + exclusions + ignores + filters + termination
                          │
                          ▼
             compiled traversal plan
       narrowed root · active pattern states
       pruning rules · metadata requirements
                          │
                          ▼
             stream matching filesystem paths
```

Cette conception vise à éviter des opérations inutiles sur le système de fichiers. Les gains dépendent de la requête et de l’outil de comparaison. Consultez les mesures ci-dessous et la méthodologie consignée.

## Performances

Le tableau présente les **invocations de la CLI avec les réglages par défaut** mesurées le 18 novembre 2026 sous Windows/NTFS, sur l’implémentation de la recherche du commit `8035ad6`. Chaque outil effectue sa recherche dans le même jeu de test de 100 000 fichiers. Les ensembles de résultats sont vérifiés avant et après le chronométrage. Les valeurs sont les médianes de neuf exécutions dans des processus neufs, après trois exécutions de préchauffage. Elles comprennent le démarrage et la capture de la sortie.

`n/t` signifie que le programme de mesure n’a pas testé d’invocation équivalente. La fonctionnalité peut tout de même être disponible. Les mesures de ripgrep sont réalisées en mode `--files`, sans recherche dans le contenu des fichiers.

| Scénario | Branchcut | zlob | ripgrep (`rg --files`) | fd | fast-glob | tinyglobby |
|---|---:|---:|---:|---:|---:|---:|
| Recherche large de fichiers | **68.79 ms** | 123.94 ms | 109.17 ms | 117.14 ms | 370.45 ms | 242.30 ms |
| Fichiers Rust avec préfixe fixe | 40.34 ms | **35.46 ms** | 75.23 ms | 40.91 ms | 210.08 ms | 153.56 ms |
| Fichiers Rust/TOML avec exclusions | **59.27 ms** | n/t | 79.27 ms | 92.57 ms | 333.23 ms | 208.11 ms |
| Fichiers Rust et chemins masqués | **64.51 ms** | 98.29 ms | 84.53 ms | 94.45 ms | 314.34 ms | 195.53 ms |
| Fichiers Rust avec `.gitignore` | **68.29 ms** | 80.07 ms | 77.18 ms | 113.49 ms | n/t | n/t |
| Répertoires | **53.76 ms** | 80.35 ms | n/t | 84.66 ms | 251.20 ms | 152.86 ms |
| Fichiers Rust triés | **101.96 ms** | 103.99 ms | 260.04 ms | n/t | n/t | n/t |
| 10 premiers fichiers Rust | **37.50 ms** | 98.46 ms | n/t | 46.00 ms | n/t | n/t |
| Recherche large avec sortie délimitée par NUL | **72.61 ms** | n/t | 105.54 ms | 109.36 ms | n/t | n/t |
| Recherche récursive de fichiers Rust par nom | **64.26 ms** | 96.60 ms | 76.10 ms | 94.15 ms | 318.07 ms | 208.26 ms |

Zlob arrive en tête sur la requête à préfixe fixe avec les réglages par défaut. Branchcut réalise cette même requête en 33.39 ms avec quatre threads de travail. Avec les réglages par défaut, Branchcut arrive en tête sur les autres scénarios de ce jeu de test, avec un temps proche de celui de zlob lorsque les résultats sont triés. Ces mesures valent pour les scénarios testés. Le pic de mémoire résidente observé avec Branchcut en configuration par défaut était de 6.38 MiB lors de la recherche large de fichiers. Les mesures de mémoire de chaque outil et de chaque configuration de threads testée sont conservées.

- [Résultats complets avec les réglages par défaut et ajustés, mémoire et méthode](docs/evidence/current-cli-comparison.md)
- [Résultats bruts du 18 novembre](benchmarks/results/2026-11-18-gap-closure-cli.json)
- [Comparaison historique du 12 novembre](docs/evidence/cli-comparison.md)
- [Méthodologie et limites des mesures de performance](BENCHMARKS.md)

## Bibliothèque Rust

La bibliothèque Rust permet à une application hôte de compiler une requête une seule fois et de transmettre chaque chemin à une fonction de rappel à mesure qu’il est trouvé.

```rust
use branchcut::{Completion, Query};
use std::{io, ops::ControlFlow, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = Query {
        patterns: vec!["src/**/*.rs".into()],
        ..Query::default()
    }
    .compile()?;

    let report = query.visit(&mut |path: &Path| -> io::Result<ControlFlow<()>> {
        println!("{}", path.display());
        Ok(ControlFlow::Continue(()))
    })?;

    assert_eq!(report.completion, Completion::Complete);
    Ok(())
}
```

La fonction de rappel peut interrompre le parcours avec `ControlFlow::Break`. Le [tutoriel de la bibliothèque Rust](docs/tutorials/rust-library.md) détaille la gestion des erreurs et le tri et donne des exemples de fonctions de visite.

## Compatibilité et limites

Branchcut se concentre sur les requêtes sur le système de fichiers. La compatibilité avec fast-glob, tinyglobby, zlob, fd ou ripgrep se limite aux comportements documentés et vérifiés, et leurs API complètes ne sont donc pas interchangeables.

- La prise en compte des règles qui indiquent les chemins à ignorer s’active explicitement. `--gitignore` active les sources de règles Git et les fichiers `.gitignore` imbriqués. `--ignore` ajoute les fichiers `.ignore` imbriqués. Les règles indiquées explicitement avec `--ignore-file` ont la priorité la plus élevée. Certaines conditions de configuration de Git et certaines formes de syntaxe de ces règles restent non prises en charge.
- Par défaut, la recherche de correspondances s’effectue sur les octets. Pour les chemins en UTF-8 valide, `--unicode` applique les jokers aux valeurs scalaires Unicode et compare la casse par conversion simple en minuscules. L’option laisse de côté le pliage de casse complet, la normalisation et la recherche par graphèmes.
- Les seuils de taille décimaux sont exacts et doivent correspondre à un nombre entier d’octets. Les dates de modification acceptent les dates UTC, les décalages horaires et les fractions de seconde au format RFC3339, ainsi que les instants Unix au format `@seconds`.
- Le suivi des liens symboliques vers des répertoires s’active explicitement. Lorsqu’il est activé, le parcours est séquentiel et détecte les cycles. `--one-file-system` est réservé à Unix.
- Une valeur finie pour `--max-open` limite le nombre de répertoires ouverts simultanément en stockant les entrées restantes en mémoire pour reprendre leur parcours plus tard. Cela augmente la consommation de mémoire en échange d’un nombre plus faible de répertoires ouverts.
- Branchcut sélectionne des chemins et lit les métadonnées du système de fichiers. La recherche optionnelle de texte littéral dans les fichiers sélectionnés est disponible avec `--content` (alias `--grep`) : plusieurs motifs se combinent avec OR, la sortie est `chemin:ligne:colonne:texte`, les fichiers binaires sont ignorés sauf avec `--text`, et les requêtes qui ne la demandent pas ne lisent aucun contenu. Elle n’implémente ni expressions régulières, ni captures, ni correspondances multilignes. Branchcut ne gère pas non plus d’index, ne surveille pas les changements et ne fournit pas d’interface graphique.

- Sans `--strict`, le parcours se poursuit après le signalement des erreurs du système de fichiers. Le tri parcourt l’arborescence avant d’émettre les résultats. Les requêtes ordinaires sans tri émettent les résultats au fil du parcours.

## Documentation

| Pour commencer | Référence et usages avancés |
|---|---|
| [Démarrage rapide](docs/getting-started/quick-start.md) | [Référence de la CLI](docs/reference/cli.md) |
| [Recherches courantes](docs/guides/everyday-search.md) | [Syntaxe des motifs glob](docs/reference/glob-syntax.md) |
| [Requêtes enregistrées](docs/guides/project-queries.md) | [Chemins à ignorer et chemins masqués](docs/guides/ignore-and-hidden.md) |
| [Tutoriel de la bibliothèque Rust](docs/tutorials/rust-library.md) | [Sortie et exécution](docs/guides/output-and-exec.md) |
| [État des fonctionnalités](docs/reference/feature-status.md) | [Référence de compatibilité](docs/reference/compatibility.md) |

Les liens de langue en haut de page mènent aux traductions de ce README. La documentation détaillée est actuellement en anglais. La documentation de référence alimente également le site dans [website/](website/README.md).

## Contribuer

Les signalements de bogues, les pull requests ciblées et les corrections de traduction sont les bienvenus. Toute régression de comportement confirmée doit faire l’objet d’un test. Toute affirmation de performance exige des mesures reproductibles qui conservent aussi les résultats défavorables.

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo tree -e normal
```

Consultez le [guide de vérification pour les contributeurs](docs/contributing/verification.md), les [règles d’architecture](AGENTS.md), la [feuille de route](ROADMAP.md), le [guide de documentation](docs/contributing/documentation.md) et la [méthodologie des mesures de performance](BENCHMARKS.md). Le site web est indépendant du graphe de dépendances Rust, qui ne comporte aucune crate tierce.

## Origines et licence

La [version soumise au concours](https://github.com/codex-mohan/branchcut/tree/submission-v1) est conservée comme trace historique. Branchcut a été créé à l’occasion du Zero Dependency 72-Hour Hackathon. Le développement actuel suit les règles de produit et d’architecture d’après-concours définies dans [AGENTS.md](AGENTS.md) et conserve intacte l’étiquette Git de la soumission.

Branchcut est distribué sous [licence MIT](LICENSE). Consultez [STDLIB.md](STDLIB.md) pour le registre des dépendances.
