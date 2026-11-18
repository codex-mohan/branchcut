<div align="center">

<img src="docs/brand/branchcut-icon.svg" alt="Branchcut icon" width="112">

# Branchcut

**Compila la consulta. Poda el árbol.**

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
  <strong>Español</strong> ·
  <a href="README.fr.md">Français</a> ·
  <a href="README.zh-CN.md">简体中文</a> ·
  <a href="README.ja.md">日本語</a> ·
  <a href="README.hi.md">हिन्दी</a>
</p>

</div>

Branchcut es un motor nativo de consultas al sistema de archivos para búsquedas cotidianas, flujos de trabajo de desarrollo y scripts. Compila patrones, exclusiones, reglas de omisión, filtros de entradas y condiciones de parada en un único plan de recorrido. Puede podar directorios que no pueden aportar resultados, emitir las rutas coincidentes a medida que las encuentra y explicar el trabajo realizado.

La biblioteca y la CLI se dividen en módulos de Rust con responsabilidades específicas. El motor central y la CLI estándar solo usan la biblioteca estándar de Rust. El grafo de dependencias en tiempo de ejecución no incluye crates de terceros.

[Sitio web y documentación](https://codex-mohan.github.io/branchcut/) · [Versiones](https://github.com/codex-mohan/branchcut/releases) · [Inventario de funcionalidades](docs/reference/feature-status.md) · [Informar de un error](https://github.com/codex-mohan/branchcut/issues)

## Instalación

Descarga el archivo comprimido para tu sistema desde [GitHub Releases](https://github.com/codex-mohan/branchcut/releases/latest), verifícalo con `SHA256SUMS`, extráelo y coloca el ejecutable en una carpeta de tu `PATH`. Los binarios precompilados no requieren Rust ni Cargo. Consulta las [instrucciones para archivos comprimidos](docs/contributing/releases.md#verify-and-install-an-archive).

Los scripts de instalación pueden instalar Rust si falta Cargo. La instalación directa con Cargo requiere que las herramientas de Rust ya estén instaladas. Cada método compila Branchcut a partir del código fuente.

**Windows PowerShell**

```powershell
irm https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.ps1 | iex
```

**Linux y macOS**

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.sh | sh
```

**Cargo**

```sh
cargo install --locked --git https://github.com/codex-mohan/branchcut.git
```

Comprueba que el comando esté disponible.

```sh
branchcut --version
branchcut --help
```

Los scripts instalan una compilación en modo release. Consulta la [guía de instalación](docs/getting-started/installation.md) para elegir una ubicación, desinstalar Branchcut o resolver problemas con `PATH`.

## Uso

Busca archivos de código fuente de Rust dentro del directorio actual.

```sh
branchcut --glob 'src/**/*.rs'
```

El texto simple se busca literalmente en los nombres de archivo.

```sh
branchcut config
```

Combina patrones, exclusiones, filtros de archivos y un límite de resultados.

```sh
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --type file \
  --limit 100
```

Consulta el plan y los contadores del recorrido efectivo.

```sh
branchcut --glob '**/*.rs' --explain
branchcut --glob '**/*.rs' --stats
```

### Búsquedas cotidianas y consultas guardadas

Busca archivos habituales de código fuente y configuración, respetando las fuentes de reglas de omisión e incluyendo las rutas ocultas.

```sh
branchcut --glob '**/*.{rs,toml}' --ignore --hidden
```

Busca por tamaño y fecha de modificación.

```sh
branchcut --glob '**/*' --size '>=1.5MiB' --modified-within 30d
branchcut --glob '**/*.log' --modified-before '2025-01-01T12:00:00+05:30'
```

Guarda las selecciones reutilizables de un proyecto en `.branchcut-queries`.

```text
[workspace]
glob packages/**/src/**/*.{rs,ts}
exclude **/target/**
ignore true
max-depth 8
```

Luego ejecuta el perfil.

```sh
branchcut --config .branchcut-queries --profile workspace --explain
```

Encontrarás más ejemplos en [Búsquedas cotidianas](docs/guides/everyday-search.md), [reglas de omisión y rutas ocultas](docs/guides/ignore-and-hidden.md) y [consultas de proyecto guardadas](docs/guides/project-queries.md).

### Canalizaciones y comandos

Usa rutas delimitadas por NUL para pasar nombres de archivo arbitrarios de Unix a otro programa.

```sh
branchcut --glob '**/*.rs' --print0 | xargs -0 rustfmt
```

Usa JSON Lines cuando las herramientas que reciben la salida necesiten rutas estructuradas, o añade datos del sistema de archivos con `--json-metadata`.

```sh
branchcut --glob '**/*.log' --json-metadata
```

Ejecuta un comando con cada resultado como argumento, sin pasar la ruta por un intérprete de comandos.

```sh
branchcut --glob '**/*.tmp' --exec 'rm {}'
```

Para selecciones grandes, `--exec-batch` agrupa las rutas en invocaciones de tamaño limitado. Usa `--exec-preview` para inspeccionar los argumentos (`argv`) sin ejecutar los comandos. Consulta [salida y ejecución](docs/guides/output-and-exec.md) para conocer los marcadores de posición, los detalles de cada plataforma y el comportamiento en caso de error.

## Funcionalidades

- **Búsqueda con globs.** Admite `*`, `?`, `**`, clases de caracteres, llaves anidadas, rangos acotados y extglobs limitados a un componente de ruta.
- **Consultas compuestas.** Combina varios patrones positivos, exclusiones, selección por extensión y tipo, profundidad, términos literales y filtros de metadatos en una sola consulta.
- **Reglas de omisión.** Las rutas ocultas se excluyen por defecto. Puedes habilitar las reglas de omisión de Git, `.ignore` o archivos de reglas explícitos.
- **Controles de recorrido.** Salida en flujo continuo, `--first`, `--limit`, ordenación, selección del número de hilos de trabajo, seguimiento de enlaces simbólicos, recorrido limitado a un sistema de archivos en Unix y límite de directorios abiertos.
- **Filtros de metadatos.** Umbrales exactos de tamaño de archivo en unidades decimales o binarias e intervalos de fecha de modificación. Se evita consultar metadatos cuando la consulta no los requiere.
- **Consultas reutilizables.** Perfiles con nombre para las consultas de un proyecto, en un pequeño archivo de configuración de texto.
- **Diagnóstico.** `--explain`, `--why PATH` y `--stats` muestran el plan, la decisión sobre una ruta candidata y el trabajo medido durante el recorrido.
- **Salida y flujos de trabajo.** Rutas en texto plano, JSON Lines con metadatos opcionales, rutas delimitadas por NUL, formato, color, hipervínculos, ejecución de comandos sin intérprete, lotes de tamaño limitado y autocompletado en el intérprete de comandos.
- **API de la biblioteca.** Compila una `Query` una vez y visita los resultados a través de la biblioteca de Rust sin invocar la CLI.

Consulta la [referencia de la CLI](docs/reference/cli.md), la [sintaxis glob](docs/reference/glob-syntax.md) y el [estado de las funcionalidades](docs/reference/feature-status.md) para conocer la sintaxis, los límites y el estado de implementación.

## Cómo funciona

Una búsqueda típica con varios filtros puede implementarse con un recorrido completo seguido de comprobaciones de globs, exclusiones, lecturas de metadatos y recopilación de resultados. Branchcut compila todas estas partes en un plan de consulta y propaga por el árbol los estados de los patrones que aún pueden coincidir.

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

El diseño busca evitar trabajo innecesario en el sistema de archivos. La ventaja depende de la consulta y de la herramienta con la que se compare. Consulta los resultados de la comparación y la metodología documentada.

## Rendimiento

La tabla recoge las **invocaciones de la CLI con la configuración predeterminada** de las pruebas del 18 de noviembre de 2026 en Windows/NTFS sobre la implementación de búsqueda del commit `8035ad6`. Cada herramienta busca en el mismo conjunto de prueba de 100 000 archivos. Los conjuntos de resultados se comprueban antes y después de medir los tiempos. Los valores son las medianas de nueve ejecuciones en procesos nuevos, tras tres ejecuciones de calentamiento, e incluyen el arranque y la captura de la salida.

`n/t` indica que el programa de pruebas no midió una invocación equivalente. La funcionalidad puede estar disponible. Las mediciones de ripgrep corresponden al modo `--files`, sin búsqueda en el contenido de los archivos.

| Carga de trabajo | Branchcut | zlob | ripgrep (`rg --files`) | fd | fast-glob | tinyglobby |
|---|---:|---:|---:|---:|---:|---:|
| Búsqueda amplia de archivos | **68.79 ms** | 123.94 ms | 109.17 ms | 117.14 ms | 370.45 ms | 242.30 ms |
| Archivos Rust con prefijo fijo | 40.34 ms | **35.46 ms** | 75.23 ms | 40.91 ms | 210.08 ms | 153.56 ms |
| Archivos Rust/TOML con exclusiones | **59.27 ms** | n/t | 79.27 ms | 92.57 ms | 333.23 ms | 208.11 ms |
| Archivos Rust con rutas ocultas | **64.51 ms** | 98.29 ms | 84.53 ms | 94.45 ms | 314.34 ms | 195.53 ms |
| Archivos Rust con `.gitignore` | **68.29 ms** | 80.07 ms | 77.18 ms | 113.49 ms | n/t | n/t |
| Directorios | **53.76 ms** | 80.35 ms | n/t | 84.66 ms | 251.20 ms | 152.86 ms |
| Archivos Rust ordenados | **101.96 ms** | 103.99 ms | 260.04 ms | n/t | n/t | n/t |
| Primeros 10 archivos Rust | **37.50 ms** | 98.46 ms | n/t | 46.00 ms | n/t | n/t |
| Búsqueda amplia con salida delimitada por NUL | **72.61 ms** | n/t | 105.54 ms | 109.36 ms | n/t | n/t |
| Búsqueda recursiva de archivos Rust por nombre | **64.26 ms** | 96.60 ms | 76.10 ms | 94.15 ms | 318.07 ms | 208.26 ms |

Zlob gana en la consulta con prefijo fijo con la configuración predeterminada. Branchcut registra 33.39 ms para esa consulta con cuatro hilos de trabajo. Branchcut lidera las demás filas con la configuración predeterminada en este conjunto de prueba y obtiene tiempos cercanos a los de zlob cuando se ordenan los resultados. Las mediciones se limitan a estas cargas de trabajo. El pico observado del conjunto de trabajo en memoria de Branchcut con la configuración predeterminada fue de 6.38 MiB en la búsqueda amplia de archivos. Se conservan las muestras de memoria de todas las herramientas y de todas las configuraciones de hilos probadas.

- [Resultados completos con la configuración predeterminada y con ajustes, memoria y método](docs/evidence/current-cli-comparison.md)
- [Registro de resultados sin procesar del 18 de noviembre](benchmarks/results/2026-11-18-gap-closure-cli.json)
- [Comparación histórica del 12 de noviembre](docs/evidence/cli-comparison.md)
- [Metodología y límites de las pruebas de rendimiento](BENCHMARKS.md)

## Biblioteca de Rust

La biblioteca de Rust permite que una aplicación compile una consulta una vez y entregue las rutas a una función de devolución de llamada a medida que las encuentra.

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

La función de devolución de llamada puede detener el recorrido con `ControlFlow::Break`. Consulta el [tutorial de la biblioteca de Rust](docs/tutorials/rust-library.md) para conocer el manejo de errores, la ordenación y ejemplos de funciones de visita.

## Compatibilidad y límites

Branchcut se centra en las consultas al sistema de archivos. La compatibilidad con fast-glob, tinyglobby, zlob, fd o ripgrep se limita a los comportamientos documentados y comprobados, por lo que sus API completas no son intercambiables.

- Las reglas de omisión se habilitan explícitamente. `--gitignore` habilita las fuentes de reglas de Git y los archivos `.gitignore` anidados. `--ignore` añade los archivos `.ignore` anidados. Las reglas explícitas de `--ignore-file` tienen la máxima prioridad. Algunas condiciones de configuración de Git y formas de la sintaxis de omisión siguen sin estar disponibles.
- La búsqueda de coincidencias opera sobre bytes de forma predeterminada. Para las rutas con UTF-8 válido, `--unicode` hace que los comodines operen sobre valores escalares Unicode y que la comparación de mayúsculas y minúsculas use una conversión simple a minúsculas. La opción carece de plegado completo de mayúsculas y minúsculas, normalización y coincidencia por grafemas.
- Los umbrales de tamaño decimales son exactos y deben corresponder a un número entero de bytes. Las fechas de modificación admiten fechas en UTC, desfases horarios y fracciones según RFC3339 e instantes Unix con el formato `@seconds`.
- El seguimiento de enlaces simbólicos a directorios se habilita explícitamente. Al seguir enlaces, el recorrido es secuencial y detecta ciclos. `--one-file-system` solo está disponible en Unix.
- Un valor finito de `--max-open` limita el número de directorios abiertos dejando las entradas pendientes en memoria y retomando los directorios almacenados. Esto aumenta el uso de memoria a cambio de mantener menos directorios abiertos.
- El proyecto se limita a consultar el sistema de archivos. Quedan fuera la búsqueda en el contenido, el mantenimiento de índices, la vigilancia de cambios y la interfaz gráfica.

- Sin `--strict`, se informa de los errores del sistema de archivos y el recorrido continúa. La ordenación completa el recorrido antes de emitir resultados. Las consultas ordinarias sin ordenación emiten las rutas a medida que las encuentran.

## Documentación

| Primeros pasos | Referencia y flujos de trabajo avanzados |
|---|---|
| [Inicio rápido](docs/getting-started/quick-start.md) | [Referencia de la CLI](docs/reference/cli.md) |
| [Búsquedas cotidianas](docs/guides/everyday-search.md) | [Sintaxis glob](docs/reference/glob-syntax.md) |
| [Consultas guardadas](docs/guides/project-queries.md) | [Reglas de omisión y rutas ocultas](docs/guides/ignore-and-hidden.md) |
| [Tutorial de la biblioteca de Rust](docs/tutorials/rust-library.md) | [Salida y ejecución](docs/guides/output-and-exec.md) |
| [Estado de las funcionalidades](docs/reference/feature-status.md) | [Referencia de compatibilidad](docs/reference/compatibility.md) |

Los enlaces de idioma de arriba llevan a las traducciones de este README. La documentación detallada está disponible actualmente en inglés. La documentación oficial también alimenta el sitio de [website/](website/README.md).

## Contribuciones

Se agradecen los informes de errores, las solicitudes de cambio de alcance acotado y las correcciones de traducción. Cada regresión de comportamiento confirmada requiere una prueba. Las afirmaciones de rendimiento requieren mediciones reproducibles que conserven también los resultados desfavorables.

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo tree -e normal
```

Lee la [guía de verificación para colaboradores](docs/contributing/verification.md), las [reglas de arquitectura](AGENTS.md), la [hoja de ruta](ROADMAP.md), la [guía de documentación](docs/contributing/documentation.md) y la [metodología de las pruebas de rendimiento](BENCHMARKS.md). El sitio web es independiente del grafo de dependencias de Rust, que carece de crates de terceros.

## Origen y licencia

La [versión presentada al concurso](https://github.com/codex-mohan/branchcut/tree/submission-v1) se conserva como registro histórico. Branchcut nació en el Zero Dependency 72-Hour Hackathon. El desarrollo actual sigue las reglas de producto y arquitectura posteriores al concurso que figuran en [AGENTS.md](AGENTS.md) y conserva intacta la etiqueta de la entrega.

Branchcut se distribuye con [licencia MIT](LICENSE). Consulta [STDLIB.md](STDLIB.md) para ver el registro de dependencias.
