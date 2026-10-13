# The Incident at Galley House — Traducción al Español (España)

Traducción comunitaria no oficial al castellano peninsular para **The Incident at Galley House**, adaptada a la arquitectura de Godot Engine 4 utilizada por el juego.

---

> [!IMPORTANT]
> **Aviso sobre el uso de Inteligencia Artificial:**
> Esta traducción ha sido generada de manera asistida mediante **Inteligencia Artificial (modelos de lenguaje / LLM)**, utilizando flujos de trabajo especializados en localización de videojuegos para preservar el tono narrativo de misterio detectivesco británico, los registros de tratamiento (*tuteo* entre allegados y cortesía formal entre desconocidos), las pistas de deducción y la estricta integridad sintáctica del motor.

> [!NOTE]
> **Permiso de la desarrolladora:**
> De acuerdo con las [declaraciones oficiales de los desarrolladores en el foro de Steam](https://steamcommunity.com/app/3641000/discussions/0/570417860284882417/#c589559079513483475), la comunidad tiene permiso expreso para crear y distribuir libremente traducciones del juego de manera independiente:
> 
> > *"You are welcome to create fan translations, but we are unfortunately unable to assist you in any way."*

---

## 🚀 Instalación en otro ordenador

Tienes dos formas de instalar la traducción:

### Opción 1: Instalador (recomendada para jugadores)

El instalador es un único ejecutable. No necesitas instalar Python ni nada más.

1. Descarga el instalador de tu sistema desde la sección de **Releases** de este repositorio:
   - **Windows:** `galley-house-es-windows-x86_64.exe`
   - **Linux / Steam Deck (modo escritorio):** `galley-house-es-linux-x86_64`
   - **macOS:** `galley-house-es-macos`
2. Cierra el juego y ejecuta el instalador:
   - **Windows:** haz doble clic en el `.exe`. Si SmartScreen lo bloquea, pulsa *Más información* > *Ejecutar de todas formas*.
   - **Linux / Steam Deck:** dale permiso de ejecución (*Propiedades > Permisos > Es ejecutable*, o `chmod +x galley-house-es-linux-x86_64`) y haz doble clic.
   - **macOS:** sigue los pasos de [Instalación en macOS](#instalación-en-macos).
3. El instalador busca el juego en todas tus bibliotecas de Steam. Si no lo encuentra, te pide que elijas la carpeta del juego. También puedes pasarla como argumento:
   ```bash
   ./galley-house-es-linux-x86_64 "/ruta/a/The Incident at Galley House"
   ```
4. Abre el juego y en **Options > Game > Language** selecciona **Español**.

El instalador no crea copias de seguridad. Modifica el archivo `.pck` del juego y guarda dentro de él lo necesario para deshacer el cambio. Si lo ejecutas otra vez:
- con la misma versión de la traducción, no modifica nada;
- con una versión más reciente, actualiza la traducción.

Para desinstalar la traducción y dejar el juego exactamente como el original:
```bash
./galley-house-es-linux-x86_64 --desinstalar
```
También sirve **Steam > Propiedades > Archivos instalados > Verificar integridad de los archivos**.

> [!WARNING]
> Si Steam actualiza el juego, la traducción desaparece y tienes que volver a instalarla. Si la actualización cambia los scripts del juego, el instalador se niega a instalar hasta que haya una versión nueva de la traducción.

---

### Instalación en macOS

En macOS hacen falta pasos extra. El juego está firmado por su desarrollador, y esa firma protege todos los archivos de `galleyhouse.app`, incluido el `.pck` que modifica el instalador. Al cambiar el `.pck`, la firma deja de ser válida y macOS dice que el juego «está dañado». Por eso el instalador vuelve a firmar `galleyhouse.app` con una firma local (ad-hoc), y tienes que autorizar esa firma una vez.

1. **Permite que la terminal modifique el juego.** Si ya has abierto el juego alguna vez, macOS solo deja modificarlo a las apps con permiso de **Gestión de apps**. Ve a **Ajustes del Sistema > Privacidad y seguridad > Gestión de apps**, activa la app de terminal que vayas a usar (por ejemplo, Terminal) y ciérrala del todo.
2. **Ejecuta el instalador** desde la terminal. No está firmado por Apple, así que primero quítale la marca de cuarentena:
   ```bash
   cd ~/Downloads
   xattr -d com.apple.quarantine galley-house-es-macos
   chmod +x galley-house-es-macos
   ./galley-house-es-macos
   ```
3. **Autoriza el juego.** Abre el juego desde Steam. macOS lo bloquea y dice que «está dañado» o que no puede comprobar el desarrollador. Cierra el aviso y ve a **Ajustes del Sistema > Privacidad y seguridad**. En la sección **Seguridad**, pulsa **Abrir igualmente** junto al aviso de «galleyhouse» y confirma con tu contraseña o Touch ID.
4. Abre el juego otra vez desde Steam. Si macOS vuelve a preguntar, elige **Abrir**.
5. En **Options > Game > Language**, selecciona **Español**.

Ten en cuenta:
- Cada vez que instalas, actualizas o desinstalas la traducción, la firma cambia y tienes que repetir el paso 3.
- Puedes quitar el permiso de **Gestión de apps** a la terminal cuando termines.
- Para recuperar la firma original del desarrollador, usa **Verificar integridad de los archivos** en Steam. Esto también quita la traducción.

---

### Opción 2: Desde este repositorio (para desarrolladores)

Requisitos: Python 3.9+, Rust (`cargo`) y el juego instalado (para la referencia de textos originales).

1. Clona este repositorio en tu ordenador:
   ```bash
   git clone git@github.com:gortiz/tiagh-es.git
   cd tiagh-es
   ```
2. Copia los textos originales del juego a `data/reference/` (esta carpeta no se sube a git):
   ```bash
   python3 scripts/extract_translations.py "/ruta/al/juego/galleyhouse.pck"
   ```
3. Compila la traducción en `build/`:
   ```bash
   python3 scripts/build_localization.py
   ```
4. Compila el instalador. Incluye `build/galley.es.translation` dentro del ejecutable:
   ```bash
   cd installer
   cargo build --release
   ```
5. Instala la traducción en tu juego:
   ```bash
   ./target/release/galley-house-es
   ```

Al publicar una etiqueta `v*`, el workflow `.github/workflows/release.yml` compila los instaladores de Linux, Windows y macOS y crea la release.

---

## 🛠️ Herramientas y Flujo de Trabajo

El proyecto cuenta con un conjunto de herramientas automatizadas para gestionar la extracción, control de calidad, compilación y parcheo del juego:

| Script | Descripción |
| :--- | :--- |
| `scripts/extract_translations.py` | Copia los textos originales (en/fr/de) del `.pck` de tu juego a `data/reference/`, que git ignora. |
| `scripts/check_status.py` | Muestra el avance de la traducción por archivo y palabras. Necesita `data/reference/`. |
| `scripts/validate_tags.py` | Verifica la integridad de etiquetas BBCode (`[person]`, `[room]`, etc.) y marcadores (`{p0}`, `%s`). Necesita `data/reference/`. |
| `scripts/translation_batches.py` | `export` divide las claves sin traducir en lotes para un agente; `apply` incorpora sus respuestas. |
| `scripts/build_localization.py` | Compila las traducciones a binario Godot 4 (`build/galley.es.translation`). |
| `installer/` | Instalador en Rust. Detecta el juego, inyecta la traducción en el `.pck`, parchea los scripts del juego y permite desinstalarla. |

---

## 📁 Estructura del Repositorio

* `data/translations/`: Traducciones al español, un mapa `{"clave": "texto"}` por categoría. No contienen textos originales del juego:
  * `ui.json`: Interfaz, menús de inicio y pausa, opciones de configuración, terminal SpectralOS y avisos.
  * `rooms.json`: Nombres de salas y códigos abreviados de navegación (`AT`, `BI`, `CH`, etc.).
  * `evidence.json`: Fichas, nombres y descripciones de pruebas y documentos (`evidence.*`, `db.ev.*`).
  * `hints.json`: Pistas y deducciones de la libreta de investigación (`db.hint.*`).
  * `credits.json`: Créditos y roles del equipo.
  * `dialogue.json`: Diálogos principales, descripciones de escenas, líneas narradas y subtítulos de efectos de sonido.
* `data/reference/` (local, ignorada por git): textos originales del juego, para traducir y validar.
* `scripts/`: Herramientas de extracción, validación y compilación.
* `installer/`: Instalador multiplataforma en Rust. `installer/src/scripts.rs` contiene nuestros cambios a los scripts del juego: el instalador los aplica sobre la copia del juego del propio jugador.
* `.agents/skills/game-localization/`: Especificación del skill de localización eficiente de videojuegos.

---

## ⚖️ Licencia y Propiedad Intelectual

* El código fuente de las herramientas, scripts e instaladores se publica bajo la [Licencia MIT](LICENSE).
* Las adaptaciones y textos de la traducción se publican bajo licencia **Creative Commons Atribución-CompartirIgual 4.0 Internacional (CC BY-SA 4.0)**.
* *The Incident at Galley House* es propiedad exclusiva de sus respectivos desarrolladores y titulares de derechos. Este proyecto es una localización comunitaria independiente sin ánimo de lucro.
* Este repositorio y sus instaladores no incluyen textos, código ni otros archivos del juego. El instalador lee lo que necesita de la copia del juego de cada jugador.
