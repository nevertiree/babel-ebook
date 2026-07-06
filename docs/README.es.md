# BabelEbook

[![CI][ci-badge]][ci-url]
[![License: MIT][license-badge]][license-url]
[![Rust Version][rust-badge]][rust-url]
[![Release][release-badge]][release-url]

**BabelEbook** es un traductor de EPUB impulsado por grandes modelos de lenguaje.
Genera libros electrónicos bilingües (idioma de origen + idioma de destino)
donde cada párrafo traducido va seguido del texto original.

Lee esto en otros idiomas:
[English](README.en.md) ·
[简体中文](README.md) ·
[日本語](README.ja.md) ·
[한국어](README.ko.md) ·
[Русский](README.ru.md) ·
**Español**

[ci-badge]: https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml
[license-badge]: https://img.shields.io/badge/License-MIT-yellow.svg
[license-url]: ../LICENSE
[rust-badge]: https://img.shields.io/badge/rust-1.88%2B-blue.svg
[rust-url]: https://www.rust-lang.org/
[release-badge]: https://img.shields.io/github/v/release/nevertiree/babel-ebook
[release-url]: https://github.com/nevertiree/babel-ebook/releases

> Tu contenido EPUB y tus claves de API solo se procesan en tu propio equipo
> y nunca se envían a los servidores de los mantenedores del proyecto.
>
> Leer en otros idiomas:
> [中文](README.md) · [English](README.en.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Русский](README.ru.md)
> [Español](README.es.md)

<p align="center">
  <img src="assets/screenshots/01-translate.png" alt="Ventana principal de BabelEbook" width="800">
</p>

<p align="center">
  <a href="https://github.com/nevertiree/babel-ebook/releases/latest/download/BabelEbook_0.1.0_x64-setup.exe">
    <img alt="Descargar para Windows"
      src="https://img.shields.io/badge/Windows-Descargar-blue?logo=windows&logoColor=white">
  </a>
  <a href="https://github.com/nevertiree/babel-ebook/releases/latest/download/BabelEbook_0.1.0_amd64.AppImage">
    <img alt="Descargar para Linux"
      src="https://img.shields.io/badge/Linux-Descargar-orange?logo=linux&logoColor=white">
  </a>
</p>

---

## ¿Por qué BabelEbook?

| Característica | BabelEbook | Traductores en línea | Plugins de Calibre |
|----------------|------------|----------------------|--------------------|
| Totalmente local: el EPUB nunca se sube | ✅ | ❌ | ✅ |
| Diseño bilingüe lado a lado | ✅ | Parcial | Requiere ajuste manual |
| Instalador de escritorio con un clic | ✅ | No requiere instalación | Requiere Calibre |
| DeepSeek / OpenAI / Anthropic / Ollama | ✅ | Proveedor fijo | Depende del plugin |
| Glosario, selectores de exclusión, concurrencia | ✅ | Parcial | Depende del plugin |

---

## Capturas de pantalla

| Ventana principal | Configuración de cálculo | Opciones de traducción |
|-------------------|--------------------------|------------------------|
| ![Ventana principal][scr01] | ![Configuración de cálculo][scr02] | ![Opciones de traducción][scr03] |

| Progreso de la traducción | Registros |
|---------------------------|-----------|
| ![Progreso de la traducción][scr06] | ![Registros][scr07] |

[scr01]: assets/screenshots/01-translate.png
[scr02]: assets/screenshots/02-settings-compute.png
[scr03]: assets/screenshots/03-settings-translation.png
[scr06]: assets/screenshots/06-translate-progress.png
[scr07]: assets/screenshots/07-logs-progress.png

---

## Plataformas compatibles

La aplicación de escritorio está disponible en:

- **Windows** (recomendado): instaladores `.exe` (NSIS) y `.msi`.
- **Linux**: paquete `.AppImage` (portátil, se ejecuta con doble clic) y paquetes `.deb`
  para distribuciones basadas en Debian/Ubuntu.

macOS actualmente **no** tiene un instalador de escritorio oficial.
Los usuarios de macOS pueden compilar y ejecutar la versión de línea de comandos desde el código fuente.

---

## Guía de usuario

### Descarga e instalación

1. Abre la página de [Releases](https://github.com/nevertiree/babel-ebook/releases).
2. Descarga el instalador para tu sistema:

   **Windows**
   - **Recomendado para la mayoría de usuarios**: `BabelEbook_<version>_x64-setup.exe`
     (instalador NSIS, detecta automáticamente el idioma del sistema).
   - **Administradores de TI o despliegue silencioso**: `BabelEbook_<version>_x64_en-US.msi`
     (instalador MSI).

   **Linux**
   - **Recomendado para la mayoría de distribuciones**: `BabelEbook_<version>_amd64.AppImage`
     (no requiere instalación; ejecuta `chmod +x` y luego haz doble clic).
   - **Debian / Ubuntu**: `BabelEbook_<version>_amd64.deb`
     (haz doble clic para instalar, o ejecuta `sudo dpkg -i BabelEbook_<version>_amd64.deb`).

3. Haz doble clic en el instalador y sigue las instrucciones.

> **Visualización de caracteres chinos en Linux:** si tu sistema Linux no tiene instalada una fuente
> china, los caracteres chinos de la interfaz pueden aparecer como cuadrados.
> Instala una fuente china recomendada por el sistema, como `fonts-noto-cjk` en Debian/Ubuntu:
>
> ```bash
> sudo apt-get install fonts-noto-cjk
> ```

### Primer uso

#### 1. Preparar una clave de API

BabelEbook necesita llamar a la API de un gran modelo de lenguaje de terceros.
Actualmente admite DeepSeek, OpenAI, Anthropic y Ollama alojado localmente.

Usando DeepSeek como ejemplo:

1. Visita la [plataforma DeepSeek](https://platform.deepseek.com/), regístrate y crea una clave de API.
2. Abre BabelEbook y ve a **Configuración** → **Cálculo**.
3. Selecciona el proveedor `DeepSeek` e introduce tu clave de API.
4. Haz clic en **Probar conexión** para verificar la conexión.

> Si usas Ollama local, no necesitas clave de API; solo completa la URL base
> (por ejemplo `http://localhost:11434`).

### Traducir un libro

1. En la pantalla principal, haz clic en **Seleccionar EPUB** para elegir el libro que quieres traducir.
2. Selecciona el idioma de destino (de forma predeterminada `zh-CN` para chino simplificado).
3. Haz clic en **Iniciar traducción**.
4. El archivo de salida se guardará en la ubicación que hayas especificado.

### Configuración común

| Configuración | Descripción |
|---------------|-------------|
| Proveedor / API | Selecciona el proveedor de LLM e introduce la clave de API. |
| Idioma de destino | Idioma de traducción de destino, p. ej. `zh-CN`, `en`, `ja`, etc. |
| Modo de salida | `bilingual` (origen + destino), `translation_only` (solo destino),<br>`interleaved` (alternado). |
| Concurrencia | Número de capítulos traducidos en paralelo. Mayor valor es más rápido pero cuesta más. |
| Tokens máximos de entrada/salida | Tokens máximos por solicitud. Los valores predeterminados son adecuados. |
| Selectores de exclusión | Elementos a omitir, p. ej. `.code`, `pre`. |
| Glosario | Tabla de terminología para fijar la traducción de nombres propios. |

### Modos de salida

- **Bilingual**: cada párrafo traduido va seguido del texto original. Ideal para aprender idiomas.
- **Translation only**: solo se conserva el contenido traducido.
- **Interleaved**: los párrafos de origen y destino se alternan.

### Idioma de la interfaz

La aplicación de escritorio admite English, Español, 日本語, 한국어, Русский y 简体中文.
El idioma de la interfaz se selecciona automáticamente en el primer inicio según el idioma del sistema
y se puede cambiar en Configuración.

### Preguntas frecuentes

**P: ¿Por qué la traducción está vacía o faltan capítulos?**
R: Comprueba si el contenido del EPUB es una imagen escaneada; si es así, ejecuta OCR primero.
También puedes ajustar `Exclude Selectors` para omitir elementos que no deban traducirse.

**P: ¿Cuántos tokens consumirá la traducción?**
R: Usa el modo **Dry Run** en la pantalla principal o la CLI para contar tokens sin llamar realmente a la API.

**P: ¿Mi clave de API está segura?**
R: Sí. Las claves de API se almacenan en el Administrador de credenciales de Windows de forma
predeterminada y no se guardan en archivos de configuración en texto plano.

---

## Guía para desarrolladores

### Introducción al proyecto

BabelEbook utiliza una arquitectura Rust + TypeScript:

- **Rust core** (`crates/babel-ebook`): análisis de EPUB, fragmentación, caché, llamadas a LLM.
- **Rust CLI** (`crates/babel-ebook-cli`): punto de entrada de línea de comandos.
- **Aplicación de escritorio Tauri** (`desktop/`): backend Rust + frontend React/TypeScript.

### Requisitos

- [Rust](https://rustup.rs/) 1.88 o posterior
- [pnpm](https://pnpm.io/) 9+ (desarrollo de escritorio)
- Windows 10/11 (para el desarrollo de la GUI de escritorio)
- Una clave de API para el proveedor elegido

### Inicio rápido

```bash
# Clonar el repositorio
git clone https://github.com/nevertiree/babel-ebook.git
cd babel-ebook

# Compilar y probar el workspace de Rust
cargo build --workspace
cargo test --workspace

# Instalar dependencias del frontend de escritorio
cd desktop
pnpm install

# Iniciar el servidor de desarrollo de escritorio
pnpm tauri dev
```

### Estructura del proyecto

```text
├── Cargo.toml              # workspace version (single source of truth)
├── crates/
│   ├── babel-ebook/        # core translation library (Rust)
│   └── babel-ebook-cli/    # command-line interface (Rust)
├── desktop/
│   ├── src/                # React + i18next frontend (TypeScript)
│   ├── src-tauri/          # Tauri Rust backend
│   ├── e2e/                # Playwright GUI tests
│   └── scripts/            # build & release helpers
└── release/v<x.y.z>/       # final distributable installers (generated)
```

### Comandos de compilación

#### CLI

```bash
cargo build --release -p babel-ebook-cli
# Output: target/release/babel-ebook
```

#### Instalador de escritorio para Windows

```bash
cd desktop
pnpm install
pnpm tauri build
```

Resultados:

- MSI: `target/release/bundle/msi/BabelEbook_<version>_x64_en-US.msi`
- NSIS: `target/release/bundle/nsis/BabelEbook_<version>_x64-setup.exe`

#### Instalador de escritorio para Linux

En distribuciones Debian/Ubuntu o compatibles, instala primero las dependencias de Tauri:

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev xdg-utils
```

Luego compila:

```bash
cd desktop
pnpm install
pnpm tauri build
```

Resultados:

- AppImage: `target/release/bundle/appimage/BabelEbook_<version>_amd64.AppImage`
- deb: `target/release/bundle/deb/BabelEbook_<version>_amd64.deb`

> **Fuente china de la interfaz en Linux:** si tu sistema Linux no tiene instalada una fuente china,
> los caracteres chinos de la interfaz pueden aparecer como cuadrados.
> Instala `fonts-noto-cjk` (Debian/Ubuntu: `sudo apt-get install fonts-noto-cjk`)
> u otra fuente china del sistema.

### Controles de calidad

Antes de abrir una PR, asegúrate de que lo siguiente se cumpla:

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cd desktop
pnpm exec tsc --noEmit
pnpm build
```

### Directrices de contribución

¡Las contribuciones son bienvenidas!
Lee [.github/CONTRIBUTING.md](.github/CONTRIBUTING.md),
[.github/CODE_OF_CONDUCT.md](.github/CODE_OF_CONDUCT.md) y
[.github/SECURITY.md](.github/SECURITY.md) primero.

#### Modelo de ramas

Este proyecto sigue **Git Flow**:

- `master`: código de producción publicado.
- `develop`: línea base de integración diaria.
- `release/<version>`: rama de estabilización de la publicación.
- `feature/<name>`: rama de funcionalidad.

#### Estilo de commits

- Usa [Conventional Commits](https://www.conventionalcommits.org/):
  - `feat:` nueva funcionalidad
  - `fix:` corrección de errores
  - `docs:` actualización de documentación
  - `refactor:` refactorización
  - `chore:` compilación/herramientas/varios
- Mantén los commits pequeños y enfocados.
- No incluyas claves de API, rutas personales ni documentos de planificación interna.

#### Requisitos de PR

1. Todos los controles de CI deben pasar.
2. Actualiza `docs/README.md` y `CHANGELOG.md` si cambia el comportamiento visible para el usuario.
3. Mantén el diff limitado a la funcionalidad o corrección.
4. Los cambios de escritorio deben incluir o actualizar pruebas E2E de Playwright.

### Flujo de publicación

```bash
cd desktop

# 1. Incrementar versión (patch / minor / major), sincronizar Cargo.toml/package.json/tauri.conf.json
#    y crear una etiqueta
pnpm version:bump minor

# 2. Ejecutar la compilación completa en el commit de la etiqueta
pnpm release:build
```

Los artefactos finales se copian a `release/v<version>/`.

### Uso avanzado de la CLI

```bash
export DEEPSEEK_API_KEY=sk-...

cargo run --release -p babel-ebook-cli -- input.epub -o output.epub \
  --provider deepseek \
  --model deepseek-chat \
  --concurrency 3 \
  --max-input-tokens 4000 \
  --max-output-tokens 2000

# Estimar tokens únicamente, sin llamar a la API
cargo run --release -p babel-ebook-cli -- input.epub -o output.epub --dry-run

# Usar un archivo de configuración JSON
cargo run --release -p babel-ebook-cli -- input.epub -o output.epub --config config.json
```

Ejecuta `babel-ebook --help` para ver la lista completa de argumentos de la CLI.

### Proveedores de LLM compatibles

| Provider | `--provider` | Default model | Base URL | Notes |
|----------|--------------|---------------|----------|-------|
| DeepSeek | `deepseek` | `deepseek-chat` | `https://api.deepseek.com` | Recomendado por defecto |
| OpenAI | `openai` | — | `https://api.openai.com/v1` | Requiere `--model` explícito |
| Anthropic | `anthropic` | `claude-3-5-sonnet-20241022` | `https://api.anthropic.com` | — |
| Ollama | `ollama` | `llama3` | local | No requiere clave de API |
| OpenAI-compatible | `openai-compatible` | — | Se establece mediante `base_url` | Para endpoints propios o proxy |

### Seguridad

- **Nunca incluyas claves de API:**
  - Usa variables de entorno, el llavero del sistema operativo o archivos de configuración locales
    ignorados por `.gitignore`.
  - No escribas claves de API en el código ni las incluyas en Git.
- Informa de vulnerabilidades de seguridad de forma privada mediante [.github/SECURITY.md](.github/SECURITY.md).

### Agradecimientos

Construido con Rust, Tauri, React e i18next.

## Licencia

MIT
