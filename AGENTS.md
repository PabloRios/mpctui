# AGENTS.md — mpctui

Base de conocimiento para cualquier agente o persona que trabaje en este repositorio. Leer completo antes de tocar código. El plan y el estado de avance viven en `PLAN.md`.

---

## 1. Proyecto

- **Qué es:** cliente TUI de MPD (Music Player Daemon) para Linux, estilo ncmpcpp, escrito en Rust.
- **Stack:** Rust edition 2024, `tokio` (async), `ratatui` + `crossterm` (TUI), `thiserror`/`anyhow` (errores), `serde` + `toml` (config), `directories`, `tracing` (log a archivo: la terminal es de la TUI), `rustfft` (visualizador), `reqwest` con rustls (LRCLIB).
- **Cliente de MPD propio:** no se usa ningún crate de MPD; el protocolo de texto está implementado en `src/repository/mpd/`.
- **MVP:** cola + controles, biblioteca (Artista → Álbum → Tema y por directorios), búsqueda, playlists guardadas.
- **Estado:** MVP completo, más los extras de `DESIGN_EXTRA.md` (formato de audio, actualización de la base, cuenta regresiva de reconexión y visualizador con seis formas), letras, la interfaz en inglés (por defecto) y español, y la pestaña Salidas (particiones y salidas de audio; usar otra partición está pendiente: fase S2). El detalle por fases está en `PLAN.md`; la guía de uso, en `README.md`; las decisiones e ideas pendientes, en `DECISIONES.md`.

---

## 2. Principios SOLID (obligatorios en todo diseño y refactor)

Todo diseño respeta SOLID. En este proyecto eso significa:

- Las abstracciones son **traits angostos** (ver `src/repository/traits.rs`): `PlayerRepository`, `OptionsRepository`, `QueueRepository`, `LibraryRepository`, `PlaylistRepository`, `OutputRepository`, `ChangesRepository`, `PcmSource`, más `Transport` en `src/repository/mpd/transport.rs`. Se extiende componiendo e implementando traits, no modificando structs existentes.
- Cada consumidor depende **solo del trait que necesita** (p. ej. el vigilante de cambios solo pide `ChangesRepository`), nunca de `MpdClient` ni de `Connection`.
- Las dependencias se **inyectan por constructor** (`MpdClient::new(transport)`) y se arman en **`src/factory.rs`** (composition root). Ninguna struct instancia sus colaboradores.

## 3. Principios adicionales de diseño

- **DRY:** nunca duplicar lógica. Extraer el comportamiento compartido a funciones o helpers. Ejemplos: `MpdClient::run`/`fetch`, `Command::optional_arg`, `filter_expression`, `validate_single_line`, `parse_with`.
- **KISS:** la solución más simple que resuelva correctamente el problema. Sin sobre-diseño.
- **YAGNI:** no agregar funcionalidad, comandos ni campos hasta que el MVP o la fase en curso los necesite. Si un campo de MPD no se muestra ni se usa, no se parsea.
- **Separación de responsabilidades:** la capa de entrada (vista TUI, `src/tui/`), la lógica de negocio (servicios, `src/service/`) y el acceso a datos (repositorios, `src/repository/`) están estrictamente separadas. Ejemplos:
  - "Pausa/reanuda según el estado" es lógica de negocio: va en un servicio, no en el repositorio.
  - El repositorio solo expone `set_pause(bool)`.
- **Composición sobre herencia:** structs chicas y enfocadas que se combinan; un tipo puede implementar varios traits angostos.
- **Fail fast:** validar los inputs en el borde (capa TUI/entrada) antes de ejecutar cualquier lógica de negocio. En Rust el equivalente de Pydantic son los **newtypes con constructor validado** de `src/domain/inputs.rs`:
  - Tipos actuales: `Volume`, `SearchText`, `PlaylistName`, `PartitionName` (ASCII alfanumérico, `-` y `_`), `Password` (este último con `Debug` enmascarado) y `PcmFormat` (16 bits, 1 o 2 canales, 8–192 kHz).
  - Si existe un valor de estos tipos, es válido. Servicios y repositorios los reciben ya validados y no revalidan.
  - Nunca corregir en silencio (nada de `.min(100)`): devolver `InputError`.
  - Las respuestas del servidor también se parsean de forma estricta: un valor inválido es `Error::Protocol`, no `None`.
- **Nombres con significado:** revelar la intención. Nada de abreviaturas, variables de una letra (salvo contadores) ni nombres genéricos (`data`, `obj`, `buf`, `conn`, `k`/`v`).
- **Funciones pequeñas:** cada función hace una sola cosa. Si un bloque necesita un comentario que explique qué hace, se extrae a una función con nombre propio.
- **Sin números ni strings mágicos:** usar constantes o `enum` con nombre. Ejemplos:
  - En `vocabulary.rs` están `Verb` (comandos), `keys::*` (claves de respuesta), `FLAG_ON/OFF`, `SINGLE_ONESHOT` y `VOLUME_UNAVAILABLE`.
  - En `address.rs` están `DEFAULT_HOST/PORT`.
  - En `domain/inputs.rs` están `MAX_VOLUME` y `PATH_SEPARATOR`.

### Equivalencias con la versión Python de estas reglas

| Regla original (Python) | En este proyecto (Rust) |
|---|---|
| `typing.Protocol` en `repository/protocols.py` | traits en `src/repository/traits.rs` |
| armado en `tui/factory.py` | `src/factory.rs` |
| Pydantic `BaseModel` + validators | newtypes con `new() -> Result<_, InputError>` en `src/domain/inputs.rs` |
| `Enum` / constantes | `enum` y `const` (ver `vocabulary.rs`) |

---

## 4. Arquitectura

```
domain  ←  repository  ←  service  ←  tui
                 ↑ factory (composition root) arma todo
i18n (textos del usuario) ← tui, main
```

- **`src/i18n/` (textos del usuario):** es el **único lugar con palabras de la interfaz**, como `glyphs.rs` lo es de los glifos.
  - `Phrase` son los textos fijos y `Message` los que llevan datos (cantidades con plural, nombres, errores). `English` y `Spanish` implementan el trait `Catalog` con un `match` exhaustivo: si falta una traducción, no compila.
  - `catalog(Language)` elige el catálogo; la factory lo inyecta en `Appearance` y las vistas lo piden con `context.phrase(...)`, `context.message(...)` y `context.key(action)`. `App` arma los avisos con `self.phrase`/`self.message`.
  - Las capas internas **no producen texto para el usuario**: devuelven datos o errores tipados (`repository::Error`, `InputError`, `ConfigError`) y el catálogo los explica (`Message::Failure`, `Message::Input`, `Message::Config`). El `Display` de `thiserror` queda en español para el log.
  - Un texto nuevo de la interfaz se agrega como variante de `Phrase` o `Message` y en los dos catálogos; nunca como literal en una vista.
  - No se traducen: el protocolo de MPD (`vocabulary.rs`), las claves del TOML y las variables de entorno, las unidades (`kHz`, `dB`, `ms`), los nombres propios (LRCLIB) ni las teclas que se escriben igual (`Enter`, `Tab`, `Ctrl+`).

- **`src/domain/`:** modelos compartidos (`Song`, `Status`, `AudioFormat`, `PlayState`, `SingleMode`, `Entry`, `Subsystem`, `Tag`, `TagFilter`, `Output`, `Partition`), entradas validadas, la línea de tiempo (`timeline()`: tramos consecutivos del mismo álbum y offsets de cada tema) las bandas del espectro (`band_edges`/`band_centers` logarítmicos de 40 Hz a 16 kHz) y el visualizador (`visual.rs`: `VisualizerStyle` y su ciclo, `VisualRequest`, `VisualFrame` con un `VisualContent` por forma). No depende de ninguna otra capa ni conoce MPD.
- **`src/repository/`:**
  - `traits.rs` define los contratos y `error.rs` el error de acceso a datos. Es `Clone + Eq` (el de E/S se guarda como `IoFailure`, con su tipo y su texto) para que viaje tal cual hasta la TUI en eventos y resultados.
  - `lrclib.rs`: `LrclibProvider` implementa `LyricsProvider` sobre `reqwest` (rustls, User-Agent `mpctui/<versión>`, 10 s de espera). `/api/get` exige álbum y duración; sin ellos, o sin coincidencia, `/api/search` y se elige por duración (±2 s), prefiriendo letras sincronizadas. La URL base se inyecta (los tests usan un servidor HTTP local). Los errores de red son `Error::Http`.
  - `lyrics_files.rs`: `LocalLyricsFiles` implementa `LyricsFiles` (caché `Artista - Título.lrc|txt` y archivo junto al tema). Sanea los nombres y no sale de la biblioteca (`..`, rutas absolutas, streams).
  - `fifo.rs`: `FifoPcmSource` implementa `PcmSource` sobre la salida fifo de MPD. Abre en modo no bloqueante en la primera lectura y, tras un fin de archivo (`Ok(0)`), vuelve a abrir en la siguiente. **No** va en `mpd/`: no es el protocolo de MPD.
  - `mpd/` es la implementación:
    - `vocabulary.rs`: comandos, claves y mapeos dominio ↔ MPD. Es el único lugar con strings del protocolo.
    - `protocol.rs`: sintaxis. Escape, `Command` (y `Command::list`, una lista de comandos que viaja como un solo `Command` de varias líneas), expresiones de filtro y clasificación de líneas (`OK`/`ACK`/par).
    - `parse.rs`: pares → modelos del dominio.
    - `transport.rs`: trait `Transport`, `Connection` real (TCP o socket unix, autentica al conectar) y `MpdConnector`.
    - `reconnect.rs`: trait `Connector` + `ReconnectingTransport`, que reabre la conexión y reintenta **una vez** si se cortó (`Error::is_disconnection`). Un `ACK` no reconecta.
    - `client.rs`: `MpdClient<T: Transport>` implementa todos los traits de repositorio. Lo que depende de una partición (`outputs`, `moveoutput`, `enable/disableoutput`, `delpartition`) va en una lista con `partition X` delante (`fetch_in`/`run_in`).
- **`src/service/`:** lógica de negocio sobre los traits del repositorio.
  - `PlaybackService`: alterna pausa según el estado, pasos de volumen, ciclo de single y `play_from` (salta a un momento y deja sonando: en pausa reanuda, detenido arranca).
  - `QueueService`, `LibraryService` y `PlaylistService`.
  - `OutputService`: particiones con sus salidas (`overview`), crear, borrar (primero devuelve sus salidas a `default`), mover y activar/desactivar (decide según el estado conocido).
  - `ChangeWatcher`: loop de `idle` que emite `ServerEvent`. Es la **única fuente del estado de conexión**: si se corta, emite `Disconnected { attempt, retry_at, delay, reason }`, espera con `Backoff` exponencial (1 s → 30 s), emite `Connecting` y prueba con `ping` (no con `idle`, que bloquearía); al responder, `Connected`. `retry_at` es `tokio::time::Instant` para testear con `start_paused`.
  - `update_transition` (`database_update.rs`): decide si la actualización de la base empezó o terminó comparando dos `Status` sucesivos.
  - `visual/`: análisis en funciones puras. `pcm.rs` (`PcmDecoder` con cuadros partidos, `StereoWindow` de pares L/R), `bands.rs` (Hann, FFT con `rustfft`, bandas, dB, `Smoother`), `levels.rs` (RMS, pico, correlación), `peak.rs` (`HeldPeak`) y un módulo por forma. `Analysis` calcula solo la forma pedida y se reinicia al cambiar de forma; sin señal vacía la ventana, así todas las formas caen a cero.
  - `Visualizer` (`visualizer.rs`): tarea que lee la `PcmSource`, analiza a 20 FPS la forma del último `VisualRequest` y publica el último cuadro por `watch`. `VisualizerLauncher` la lanza con una fuente nueva; `RunningVisualizer` la cancela al soltarse (`Drop` → `abort`).
  - `LyricsService` (`lyrics.rs`): prueba las fuentes de letras en el orden de la configuración (caché, carpeta del tema, tags y LRCLIB); una fuente que falla no corta la búsqueda. Lo descargado se guarda en la caché solo si `cache` es una fuente, y cada respuesta de LRCLIB (también "no hay" o instrumental) se recuerda en memoria durante la sesión. `LyricsLookup` la corre en su propia tarea y atiende solo el último pedido; una búsqueda empezada no se interrumpe, para no dejar a medias un comando de MPD. `parse_lrc` (`lrc.rs`) es pura.
  - `PlaybackClock`: interpola el progreso.
  - Reciben entradas ya validadas y nunca tocan la terminal.
- **`src/tui/`:**
  - `app/` contiene `App<B: Backend>`. Corre el loop `tokio::select!` (teclas, `ServerEvent`, tick de 1 s) y tiene un submódulo por vista (`queue`, `library`, `browse`, `search`, `playlists`, `visual`, `lyrics`, `outputs`), más `playback`, `picker`, `prompt` y `refresh`.
  - `action.rs`: **tabla única `BINDINGS`** de la que salen `action_for` y la ayuda (`help_entries`). Un atajo nuevo se agrega solo ahí.
  - Las acciones de vista (`Activate`, `Add`, `AddAll`, `Delete`, `Back`, `Forward`, `Navigate`) las interpreta la vista activa.
  - `list.rs`: `SelectableList<T>` con `SelectionPolicy::{Keep, Reset}` (Keep para refrescos, Reset para contenido nuevo). La usan todas las listas.
  - `prompt.rs`: texto libre y confirmaciones s/n. Las acciones destructivas (vaciar cola, borrar playlist) siempre confirman; quitar un tema (de la cola o de una playlist) no.
  - `picker.rs`: selectores (`Picker`), con la misma navegación (`choose`). El de playlists (`P`): `PlaylistAddition` lleva las URI y el nombre para el aviso; la primera opción es «Nueva playlist», que abre el prompt `NewPlaylist`. El de particiones (`m`, vista Salidas): las demás particiones como destino de la salida elegida.
  - `views/` solo dibuja a partir de `AppState`, recibiendo un `chrome::Context` (glifos, modo de layout, ancho, instante). Colores en `theme.rs`, textos auxiliares en `format.rs`.
    - **Listas:** todas pasan por `widgets::render_list`, que arma solo las filas visibles a partir de un `RowSource` (cursor, foco, desplazamiento, cabecera fija, barra de desplazamiento). Una lista nueva implementa `RowSource`; no usa `Table`.
    - **Temas agrupados:** `GroupedSongRows` (Cola, Búsqueda, Playlists) intercala separadoras de álbum; la selección sigue siendo un índice de tema y la `Timeline` vive en `state::GroupedSongs`, que la recalcula solo al cambiar los temas.
    - **Marco:** `chrome::rule` dibuja `├──┤` y `chrome::divider` une solo los cruces (`┬ ┼ ┴`) mirando los símbolos ya dibujados.
    - **Barras:** `gauges.rs` devuelve `BarCell`/`MeterCell` y una tabla las traduce a glifo + estilo. Las tres barras del panel son la misma función con distintos pesos.
    - **Tamaño:** `layout::layout_mode` decide completo, compacto o aviso en cada dibujado.
    - **Visualizador:** `pages/visual/` tiene un archivo por forma más lo compartido (`columns.rs` para las barras, `canvas.rs` para los puntos braille o ASCII, `correlation.rs`). Cada forma dice qué resolución necesita (`request`) según su área; al dibujar, la vista lo deja en `state.visual.request` y la app lo usa en `sync_visualizer` para lanzar, ajustar o cancelar el visualizador. Un cuadro de otra forma no se dibuja.
  - `glyphs.rs`: juegos `UNICODE` y `ASCII` (mismo layout). Es el **único lugar con glifos literales**: las vistas piden el glifo por su rol (`glyphs.cursor`, `glyphs.border`) y reciben `&Glyphs` inyectado desde `App`. Los anchos se miden con `Span::width()`/`unicode-width`, nunca con `.len()`, y se trunca con `format::fit_line`. La única excepción a los glifos literales es el braille, que se calcula como `BRAILLE_BASE` + bits en `pages/visual/canvas.rs`; lo propio del modo ASCII de la Onda y el Vectorscopio está en `Plot::Cells`. El diseño visual completo está en `DESIGN.md`.
  - La TUI construye las entradas validadas (`SearchText`, `PlaylistName`) a partir de lo que tipea el usuario.
- **`src/config.rs` (borde de entrada):**
  - `resolve(FileConfig, Environment) -> Settings` es pura y testeable.
  - Precedencia: defecto < `~/.config/mpctui/config.toml` < `MPD_HOST`/`MPD_PORT`/`MPCTUI_GLYPHS`. `MPD_HOST` admite `contraseña@host`.
  - `language` (`en` por defecto, `es`) elige el catálogo de textos. `load()` devuelve `ConfigFailure { language, error }` para que `main` explique el error en el idioma configurado (en inglés si el TOML no se pudo leer).
  - `glyphs = "auto"` se resuelve con `resolve_glyph_mode` (pura): ASCII si el locale no es UTF-8 o `TERM` es `linux`/`dumb`.
  - `visualizer_fifo` (`/tmp/mpd.fifo`), `visualizer_format` (`44100:16:2`, validado como `PcmFormat`) y `visualizer_style` (`bars` por defecto; una forma estéreo con FIFO mono es error). Vacío o inválido no arranca; un FIFO inexistente no es error de configuración (la vista lo explica). La forma inicial viaja en `Appearance`.
  - `[lyrics]`: `enabled`, `sources`, `directory`, `music_directory`, `fetch_in_background` y `now_playing_line` (bool, `false` por defecto). Las dos últimas viajan como `Services.lyrics_in_background` y `Appearance.lyrics_line` (ya cruzada con `enabled`); `now_playing_line` implica buscar la letra siempre.
  - `album_progress` / `queue_progress` (bool, `false` por defecto) muestran las barras opcionales del panel. La factory los convierte en `tui::appearance::ProgressBars` dentro de `Appearance` (junto con los glifos), que se inyecta en `App`; la TUI no conoce `Settings`.
  - El TOML usa `deny_unknown_fields`.
  - `config.toml.dist` (raíz) documenta todas las opciones con sus valores por defecto. Una opción nueva se agrega también ahí: los tests `the_dist_config_*` exigen que resuelva a los valores por defecto y que sus líneas comentadas sean válidas.
- **`src/logging.rs`:** `tracing` a `~/.local/state/mpctui/mpctui.log`, nivel INFO.
- **`src/factory.rs`:**
  - `MpdBackend` fija los tipos concretos del trait `Backend`.
  - `build_app` abre **una conexión por servicio + una para `idle`**. Así ningún servicio espera a otro, e `idle` bloquea su propia conexión.
  - `visualizer_launcher` arma el `VisualizerLauncher<FifoPcmSource>` con un backoff corto (50 ms → 1 s) para que el visualizador reaparezca rápido al reanudar.

### Async en traits

- Los traits declaran `fn metodo(&mut self, ...) -> impl Future<Output = Result<T>> + Send` y las implementaciones usan `async fn`.
- No se usa `async_trait`.
- Estos traits no son dyn-compatible: se consumen con genéricos (`fn new(player: P) where P: PlayerRepository`), no con `Box<dyn ...>`.

### Por qué tokio

- La app tiene varias fuentes de eventos simultáneas: teclado, conexión `idle`, tick de progreso y respuestas a comandos.
- `tokio::select!` las combina en un solo loop sin locks.
- `crossterm` (feature `event-stream`) expone el teclado como stream async.
- Timeouts y reconexión con backoff salen de `tokio::time`.
- La alternativa (hilos + `std::sync::mpsc`) era viable, pero se descartó por esto.

---

## 5. Conocimiento del protocolo MPD

- **Saludo:** al conectar, el servidor manda `OK MPD x.y.z`. `check_greeting` falla si no llega eso.
- **Respuestas:**
  - Un comando responde pares `clave: valor` (separador `": "`, se parte en la primera aparición) y termina en `OK`.
  - Un error llega como `ACK [código@índice] {comando} mensaje`.
  - Después de un `ACK` la conexión sigue usable.
- **Argumentos:** siempre entre comillas dobles, escapando `\` y `"` (`quote`). MPD acepta números entre comillas.
- **Filtros (sintaxis 0.21+):**
  - Forma: `((Artist == "X") AND (Album == "Y"))`.
  - El valor se escapa dentro de la expresión y la expresión entera se vuelve a escapar como argumento: **doble escape**. Hay un test que lo cubre.
  - `find` y `list` usan `==` (exacto). `search` usa `contains` y no distingue mayúsculas.
  - `any` es el pseudo-tag que matchea cualquier tag.
- **`list <tag>`:** devuelve claves con mayúsculas canónicas (`Album: ...`); se comparan sin distinguir mayúsculas.
- **Duración:** `duration` (float, preferido) con fallback al legacy `Time` (entero).
- **`status`:**
  - `volume: -1` significa que no hay mixer (`Volume` = `None`).
  - Mientras no está detenido informa `audio` (`frecuencia:bits:canales`, con bits `f` = flotante, o `dsdN:canales`) y `bitrate` en kbps (`0` = desconocido). En pausa mantiene el último valor. El bitrate varía en VBR sin aviso de `idle`: por eso se relee `status` cada 2 s mientras suena.
  - `single` admite `0`/`1`/`oneshot` (0.21+).
  - `consume` admite `oneshot` recién desde 0.24; para el cliente cuenta como activo.
- **`pause` sin argumento está deprecado:** se usa `pause 0|1` y el toggle se decide en el servicio.
- **`lsinfo`:** sin argumento lista la raíz. Puede devolver `directory`, `file` y `playlist` (`.m3u` dentro de la biblioteca).
- **Nombres de playlist:** no pueden contener `/` ni saltos de línea.
- **`idle`:** bloquea la conexión hasta que cambia un subsistema y responde `changed: <subsistema>`. Se mapean `database`, `stored_playlist`, `playlist` (→ `Subsystem::Queue`), `player`, `mixer`, `options`, `update`, `output` (→ `Subsystem::Outputs`) y `partition` (→ `Subsystem::Partitions`); el resto se ignora.
- **`update` y `updating_db`:** `changed: update` llega cuando una actualización de la base **empieza o termina**; para saber cuál, se relee `status`, que trae `updating_db: <id>` solo mientras hay una en curso. MPD no informa progreso ni hora de inicio.
- **Edición:** `move <desde> <hasta>` corre un tema de la cola. `playlistadd <nombre> <uri>` agrega un archivo o un directorio completo al final de una playlist y la crea si no existe; `playlistdelete <nombre> <pos>` y `playlistmove <nombre> <desde> <hasta>` usan posiciones desde 0. No hay `findaddpl`: para agregar un artista se buscan sus temas con `find` y se agregan de a uno.
- **Listas de comandos:** `command_list_begin`, un comando por línea y `command_list_end`. MPD las ejecuta de corrido sobre la conexión y responde una sola vez: `OK`, o el `ACK` del primero que falla. En el servidor falso, cada línea es un paso del guion y solo la última lleva respuesta.
- **Particiones:** cada una es un reproductor con su cola, su reproducción y sus salidas; cada conexión está en una (al conectar, `default`). `listpartitions` → `partition: <nombre>`; `partition X` cambia la conexión; `newpartition X` (nombre ASCII alfanumérico, `-`, `_`); `delpartition X` solo si no tiene salidas ni clientes; `moveoutput <nombre>` trae la salida a la partición **de la conexión**. No sobreviven a un reinicio de MPD. Requieren el permiso `admin`.
- **Salidas:** `outputs` lista `outputid`, `outputname`, `plugin`, `outputenabled` (y `attribute`, que no se parsea). El id es la posición en la lista **de esa partición**. Las salidas de otra partición aparecen con `plugin: dummy` (se descartan). Se activan con `enableoutput <id>` / `disableoutput <id>` en su partición.
- **`ping`:** responde `OK` sin efectos. Lo usa el vigilante para confirmar la reconexión.
- **`readcomments <uri>`:** todos los comentarios del archivo, con la clave tal como está en él (comparar sin distinguir mayúsculas). MPD imprime el valor **sin escapar**: una letra embebida multilínea llega partida en varias líneas, incluso vacías. Por eso una línea que no es `OK`, `ACK` ni un par con clave válida para MPD (letras, `_` y `-`) es continuación del valor anterior (`ResponseLine::Continuation`). En MP3 solo expone frames `TXXX`, no `USLT`.
- **Salida fifo:** MPD escribe PCM crudo, entrelazado, con signo y en el orden de bytes nativo, en el `format` de su `audio_output` fifo. Un FIFO abierto en no bloqueante sin escritor no despierta al lector hasta que alguien escribe; cuando el escritor cierra, la lectura devuelve 0 (fin de archivo).
- **`connection_timeout`:** MPD cierra las conexiones sin actividad después de 60 s por defecto. Por eso las conexiones de comandos van envueltas en `ReconnectingTransport`. Durante `idle` no corre el timeout.
- **Inyección de protocolo:** un salto de línea en un argumento cortaría el comando. Por eso todo texto del usuario pasa por `SearchText` o `PlaylistName`, que rechazan caracteres de control.

---

## 6. Entorno de desarrollo

- **MPD:**
  - Corre como **servicio de sistema** (`systemctl is-active mpd`), no de usuario.
  - Escucha en `localhost:6600` (bind `0.0.0.0`), sin password.
  - Protocolo informado: **0.23.5**; el binario instalado es 0.23.14.
- **Config de MPD:** `~/.config/mpd/mpd.conf`.
  - `music_directory ~/Music`; playlists en `~/Music/playlists`.
  - Salidas: pulse, alsa `hw:1,3`, alsa dmix NVidia y **FIFO `/tmp/mpd.fifo` (44100:16:2)**, que lee la vista Visual. Leerla es solo lectura y se puede probar contra el servidor real (`cargo run --example spectrum_smoke`).
  - La salida `alsa hw:1,3 BLOCKING` apunta a un dispositivo que no existe: MPD registra una excepción en `~/Music/.mpd/log` cada vez que arranca un tema. Es ruido del entorno, no un problema de mpctui.
- **Biblioteca real (referencia):** unos 1229 temas en cola, 223 artistas, 43 entradas en la raíz y 0 playlists guardadas.
- **`mpc`** está instalado: sirve para comparar resultados (`mpc status`, `mpc lsplaylists`).
- **Rust:**
  - Instalado con rustup en `~/.cargo/bin` (Rust 1.99).
  - En shells no interactivos puede faltar en el `PATH`: anteponer `export PATH="$HOME/.cargo/bin:$PATH"`.

> ⚠️ **El usuario usa este MPD para escuchar música.** Contra el servidor real solo se hacen operaciones de **lectura** (`examples/smoke.rs`). Todo lo que modifique estado (play, volumen, cola, playlists) se prueba contra el servidor falso de `tests/fake_server.rs`, salvo pedido explícito.

---

## 7. Comandos y verificación

```bash
cargo fmt                                   # formato
cargo clippy --all-targets -- -D warnings   # lint, cero warnings
cargo test                                  # unitarios + integración
cargo run --example smoke                   # solo lectura contra el MPD real
cargo run --example spectrum_smoke          # lee el FIFO 3 s (solo lectura)
cargo run --example lyrics_smoke            # consulta LRCLIB con el tema actual (solo lectura; usa la red)
```

### Probar la TUI sin intervención

Correr la app en tmux, enviar teclas y leer la pantalla como texto:

```bash
tmux new-session -d -s prueba -x 140 -y 30 ./target/debug/mpctui
tmux new-session -d -s ascii -x 100 -y 30 "MPCTUI_GLYPHS=ascii ./target/debug/mpctui"
tmux send-keys -t prueba 2 j l        # teclas
tmux send-keys -t prueba -l "texto"   # texto literal
tmux capture-pane -t prueba -p        # pantalla
tmux kill-session -t prueba
```

Contra el MPD real solo se envían teclas de lectura: `1`–`8`, `Tab`, `j`/`k`, `g`/`G`, `h`/`l`, `/` + búsqueda, `o` y `?`. **Nunca** `Enter` en la cola, `p`, `s`, `<`, `>`, `+`, `-`, `f`, `b`, `z`, `r`, `y`, `R`, `a`, `A`, `d`, `c`, `S`, `u`, `J`, `K`, `n`, `m` ni `Enter` en el selector de playlists (`P` + `Esc` sí: solo lo abre y lo cierra) ni en la vista Salidas (activa o desactiva una salida).

Checklist antes de dar un cambio por terminado:

1. `cargo fmt`, `clippy` sin warnings y `cargo test` en verde.
2. Tests nuevos para la lógica nueva:
   - Unitarios dentro del módulo (`#[cfg(test)]`).
   - De punta a punta en `tests/fake_server.rs`. El servidor falso exige cada comando **textual** en orden y responde con un guion fijo.
3. Revisión contra las secciones 2 y 3 (SOLID, DRY, KISS, YAGNI, capas, fail fast, nombres, sin mágicos).
4. `PLAN.md` actualizado: fases completadas con ✅ y estructura al día.

---

## 8. Convenciones

- **Idioma:**
  - Comentarios, docs y mensajes del log (el `Display` de los errores) en **español**. Los identificadores van en inglés.
  - Los textos de la interfaz, incluidos los errores que ve el usuario, salen del catálogo de `src/i18n/`, en inglés (por defecto) y español. Los tests de pantalla usan el español salvo que digan otro idioma.
- **Errores:**
  - `repository::Error` para acceso a datos.
  - `domain::InputError` para entradas inválidas.
  - `anyhow` solo en binarios y ejemplos.
- **Clippy:**
  - Variantes de enum grandes se encajonan (`Entry::File(Box<Song>)`, por `large_enum_variant`).
  - No se silencian lints con `allow` sin justificar.
- **Commits:**
  - Formato Conventional Commits en español: `<type>(<scope>): <descripción>`.
  - Body: listado por archivo de lo que se hizo.
  - Footer solo si hay `BREAKING CHANGE`; cerrar con `Co-Authored-By`.
  - El mensaje se **agrega al final de `.gitmessage`** (un hook lo abre en gedit).
  - Va delimitado por una línea vacía, después `# ---- inicio commit ----` y al final `# ---- fin commit ----`.
  - No se hace commit sin pedido explícito.

## 9. Lecciones aprendidas

- **El prompt debe ignorar combinaciones con Ctrl o Alt.** Si no, `Ctrl+u` agrega una "u" al texto. Lo cubre el test `control_combinations_are_not_text`.
- **Nunca registrar comandos completos en el log**, porque el comando `password` lleva la contraseña. `Password` implementa `Debug` enmascarado.
- **Las listas largas no se clonan al dibujar.** `SelectableList::parts_mut` devuelve elementos y `TableState` a la vez.
- **Con 1000+ temas en la cola hace falta un salto al tema actual** (`o`).
- **La ayuda debe entrar en 30 filas.** Se agrupa por acción y se muestra en dos columnas.
- **ratatui trae glifos Unicode de fábrica** (divisor de `Tabs`, símbolos de `Scrollbar`, `LineGauge`, bordes). Siempre se configuran desde `Glyphs`; el test `ascii_screens_have_no_unicode_glyphs` lo vigila.
- **Los anchos se miden en columnas, también en los tests:** `str::find` devuelve bytes y `á` ocupa dos. Para ubicar columnas usar `chars().position` o `Span::width()`.
- **Los diálogos y acciones que modifican el MPD no se prueban en el servidor real:** se dibujan con `TestBackend` (las superposiciones están en los tests de `views/mod.rs`).
- **tmux puede ignorar `-x`/`-y`** si hay otro cliente conectado: la ventana toma el tamaño de ese cliente. Usar `tmux resize-window -t <sesión> -x 100 -y 30` después de crearla.
- **En los tests de texto, buscar patrones específicos:** "reintento" contiene "intento" y el medidor de volumen ASCII contiene `##`. Buscar `(intento` o `## ##`.
- **El primer tick de `tokio::time::interval` es inmediato:** un test de una tarea periódica no puede suponer que el primer cuadro ya trae datos; hay que esperar el cuadro que cumple la condición.
- **Los textos de UI también pueden colar glifos** (p. ej. `→` en una descripción de la ayuda). La tilde y `·`/`—`/`«»` son texto y están permitidos; flechas, cajas y figuras no.

## 10. Forma de trabajo acordada

- Por defecto, avanzar **fase por fase** según `PLAN.md` y frenar al terminar cada una para que el usuario revise. Si el usuario pide completar todo, se sigue sin frenar hasta terminar.
- Resolver todo de forma lineal, **sin lanzar subagentes**.
- Cuando el usuario pide un markdown en pantalla, se entrega crudo dentro de un único bloque de código, listo para copiar.
- Un "workitem" es un markdown para GitLab centrado en requisitos, no en lo técnico.
