# The Incident at Galley House — Traducción al Español (España)

Traducción comunitaria no oficial al castellano peninsular para **The Incident at Galley House**, adaptada a la arquitectura de Godot Engine 4 utilizada por el juego.

> [!NOTE]
> De acuerdo con las [declaraciones oficiales de los desarrolladores en Steam](https://steamcommunity.com/app/3641000/discussions/0/570417860284882417/#c589559079513483475), la comunidad tiene permiso para crear y distribuir traducciones del juego de manera independiente.

---

## 🛠️ Herramientas y Flujo de Trabajo

El proyecto cuenta con un conjunto de herramientas automatizadas para gestionar la extracción, control de calidad, compilación y parcheo del juego:

| Script | Descripción |
| :--- | :--- |
| `scripts/check_status.py` | Muestra el estado actual y porcentaje de avance de la traducción por archivo y palabras. |
| `scripts/validate_tags.py` | Verifica la integridad de etiquetas BBCode (`[person]`, `[room]`, etc.) y marcadores (`{p0}`, `%s`). |
| `scripts/build_localization.py` | Compila las cadenas traducidas a binario Godot 4 (`galley.es.translation`) y compila los scripts de configuración (`.gdc`). |
| `scripts/sync_to_game.py` | Inyecta la traducción directamente en la instalación local del juego creando una copia de seguridad (`galleyhouse.pck.bak`). |
| `scripts/package_release.py` | Genera el archivo ZIP ligero de distribución con instalador en un clic para la comunidad. |
| `scripts/extract_translations.py` | Extrae y sincroniza las cadenas del juego desde `galleyhouse.pck` conservando el progreso existente. |

---

## 📁 Estructura del Repositorio

* `data/translations/`: Archivos JSON de trabajo organizados por categorías:
  * `ui.json`: Interfaz, menús de inicio y pausa, opciones de configuración, diálogos del sistema y avisos.
  * `rooms.json`: Nombres de salas y códigos abreviados de navegación (`AT`, `BI`, `CH`, etc.).
  * `evidence.json`: Fichas, nombres y descripciones de pruebas y documentos (`evidence.*`, `db.ev.*`).
  * `hints.json`: Pistas y anotaciones de la libreta de investigación (`db.hint.*`).
  * `credits.json`: Créditos y roles del equipo.
  * `dialogue.json`: Diálogos principales, descripciones de escenas, líneas narradas y subtítulos de efectos de sonido.
* `src/scripts/`: Código fuente de scripts de Godot modificados para habilitar el selector de idioma español y el registro dinámico.
* `build/`: Artefactos binarios generados listos para empaquetar o sincronizar.
* `dist/`: Paquetes ZIP preparados para publicación y descarga comunitaria.

---

## 🚀 Instrucciones de Instalación (Para Jugadores)

1. Descarga el archivo `galley_house_es_traduccion.zip` desde la sección de **Releases**.
2. Descomprime el archivo en cualquier carpeta.
3. Ejecuta el instalador:
   ```bash
   python3 install_es.py
   ```
4. Inicia el juego desde Steam y entra en **Opciones > Juego > Idioma** para seleccionar **Español**.

---

## ⚖️ Licencia y Propiedad Intelectual

* El texto de esta traducción se publica bajo licencia **Creative Commons Atribución-CompartirIgual 4.0 Internacional (CC BY-SA 4.0)**.
* *The Incident at Galley House* es propiedad exclusiva de sus desarrolladores. Esta localización es un proyecto comunitario sin ánimo de lucro.
