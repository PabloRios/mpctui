# mpctui

Cliente de terminal para [MPD](https://www.musicpd.org/) (Music Player Daemon), escrito en Rust. Al estilo de ncmpcpp: cola, biblioteca, navegador de directorios, búsqueda, playlists guardadas y un visualizador con seis formas, con actualización en vivo cuando algo cambia en el servidor.

## Requisitos

- Rust estable reciente (probado con 1.99) y `cargo`.
- Un servidor MPD accesible (por defecto `localhost:6600`), versión 0.21 o superior.

## Instalación

```bash
cargo install --path .     # instala el binario mpctui en ~/.cargo/bin
# o, sin instalar:
cargo run --release
```

## Configuración

Todo es opcional. El orden de precedencia es: valores por defecto < archivo < variables de entorno.

**Archivo** `~/.config/mpctui/config.toml`. En la raíz del repositorio está `config.toml.dist`, una plantilla comentada con todas las opciones y sus valores por defecto: `cp config.toml.dist ~/.config/mpctui/config.toml`. Resumen:

```toml
host = "localhost"        # o la ruta a un socket unix, p. ej. "/run/mpd/socket"
port = 6600
password = "secreto"
glyphs = "auto"           # "auto" (defecto) | "unicode" | "ascii"
language = "en"           # idioma de la interfaz: "en" (defecto, inglés) | "es" (español)
album_progress = false    # barra de progreso del álbum en el panel (oculta por defecto)
queue_progress = false    # barra de progreso de la cola en el panel (oculta por defecto)
visualizer_fifo = "/tmp/mpd.fifo"   # salida fifo de MPD que lee el visualizador
visualizer_format = "44100:16:2"    # debe coincidir con el format de esa salida
visualizer_style = "bars"           # forma con la que arranca: bars, mirror, wave, waterfall, vectorscope o meters

[lyrics]
enabled = true                                         # false: sin vista 7 ni búsquedas
sources = ["cache", "song_folder", "tags", "lrclib"]   # orden de búsqueda
directory = "~/.lyrics"                                # caché (la misma de ncmpcpp y ncmpc)
music_directory = "~/Music"                            # raíz de la biblioteca; la necesita song_folder
```

**Variables de entorno** (las mismas que usa `mpc`):

| Variable | Ejemplo | Efecto |
|---|---|---|
| `MPD_HOST` | `servidor`, `clave@servidor`, `/run/mpd/socket` | Host o socket; admite `contraseña@host` |
| `MPD_PORT` | `6600` | Puerto TCP |
| `MPCTUI_GLYPHS` | `ascii` | Fuerza el juego de glifos (`auto`, `unicode` o `ascii`) |

Si la configuración es inválida (puerto no numérico, host vacío, campo desconocido en el TOML, contraseña con saltos de línea, `glyphs` distinto de `auto`/`unicode`/`ascii`, `language` distinto de `en`/`es`, `album_progress` o `queue_progress` que no sean `true`/`false`, `visualizer_fifo` vacío, `visualizer_format` que no sea de 16 bits, 1 o 2 canales y entre 8000 y 192000 Hz, `visualizer_style` desconocido, `mirror`/`vectorscope` con un FIFO de un canal, una fuente de letras desconocida o repetida, `lyrics.sources` vacía con las letras activadas, o `song_folder` sin `music_directory`), mpctui no arranca y explica el motivo en el idioma configurado (si el TOML no se puede leer, en inglés). Que el FIFO no exista no es un error de configuración: la vista Visual explica cómo crearlo.

### Glifos: Unicode o ASCII

mpctui dibuja con símbolos Unicode (`▶ ⏸ ▸ ♪ ≡`, bordes `┌─┐`) y tiene un juego ASCII equivalente (`|> || > ~ =`, bordes `+-|`) con el mismo layout. Con `glyphs = "auto"` usa ASCII si el locale (`LC_ALL`, `LC_CTYPE` o `LANG`, el primero no vacío) no es UTF-8, o si `TERM` es `linux` (consola virtual) o `dumb`. En cualquier otro caso usa Unicode.

> **Ancho ambiguo:** varios símbolos (`▶ ■ ● ○ ◐ ▸ ♪ ≡ □`, y los bloques `▁…█`, `▔`, `▄`, `▀`, `░▒▓` y `▏…▉` del visualizador) tienen ancho "ambiguo" en Unicode. Si la terminal está configurada para dibujarlos en dos columnas (común con locales CJK), las columnas se desalinean. Lo mismo pasa si la fuente no tiene alguno de los símbolos, o si la terminal muestra `⏸` como emoji. El braille de la Onda y el Vectorscopio no tiene ancho ambiguo, pero puede faltar en la fuente. En esos casos, usar `glyphs = "ascii"` o `MPCTUI_GLYPHS=ascii`.

## Uso

```bash
mpctui
```

### Vistas

| Tecla | Vista | Qué muestra |
|---|---|---|
| `1` | Cola | La cola agrupada por álbum; el tema que suena lleva `▶`/`⏸` |
| `2` | Biblioteca | Migas de pan y tres columnas: Artista → Álbum → Tema, con la duración relativa de cada tema |
| `3` | Directorios | La biblioteca por carpetas, con migas de pan desde la raíz |
| `4` | Búsqueda | Resultados agrupados por álbum, con las coincidencias en negrita y subrayadas |
| `5` | Playlists | Playlists guardadas y el contenido de la seleccionada |
| `6` | Visual | Visualizador en vivo, leyendo la salida fifo de MPD. Apretando `6` otra vez se pasa a la forma siguiente |
| `7` | Letra | La letra del tema que suena; si viene sincronizada (LRC), sigue la canción |

`Tab` / `Shift+Tab` recorren las vistas.

### Pantalla

- **Encabezado:** el servidor y el estado de la conexión, las pestañas (la activa entre corchetes) y, si entra, el resumen de la cola (`cola: 1229 temas  65:04:13`). Mientras MPD actualiza la base de datos, junto al nombre aparece `◐ actualizando biblioteca 0:42` (el tiempo cuenta desde que mpctui la vio; en modo compacto, solo el indicador). Al terminar, la línea inferior muestra `Biblioteca actualizada (1:05)`.
- **Listas de temas** (cola, búsqueda, playlists): agrupadas por álbum con una fila separadora (`── Artista - Álbum (año) ──── 10 temas  42:51 ──`) y columnas `pos` (solo en la cola), `#` (pista), Título, Álbum y duración. Si la lista empieza en medio de un álbum, arriba se ve su separadora. La selección se marca con `▸` y video inverso. A la derecha aparece una barra de desplazamiento si la lista no entra.
- **Panel "ahora suena":** en su borde superior, el estado (`▶ reproduciendo`) y, a la derecha, el formato de audio (`FLAC  44.1 kHz  16 bit  estéreo  1411 kbps`). Las partes que no se conocen se omiten y, si no entran, se quitan desde la izquierda; detenido o en modo compacto no se muestra. Mientras suena, el bitrate se actualiza cada 2 segundos. Adentro, título y álbum del tema, y barras alineadas:
  - **tema:** el progreso del tema. Siempre se muestra.
  - **álbum:** un segmento por tema del álbum en curso. Oculta por defecto; se activa con `album_progress = true`.
  - **cola:** un segmento por álbum, mostrando los álbumes cercanos al actual. Oculta por defecto; se activa con `queue_progress = true`.

  Cada barra activada suma una fila al panel. Si falta la duración de algún tema, las barras del álbum y de la cola cuentan temas en lugar de tiempo. Debajo van el volumen, los modos con su tecla (`z ● aleatorio`) y la posición en la cola.
- **Visual:** la fila de título muestra la forma activa, el indicador `● ○ ○ ○ ○ ○  1/6` y, a la derecha, los datos de la forma. `6` desde otra vista entra con la última forma usada; estando en el visualizador pasa a la siguiente, en ciclo:
  - **Barras:** una barra por banda (entre 8 y 64, según el ancho) en escala logarítmica de 40 Hz a 16 kHz, con picos que se sostienen medio segundo y caen. El color va por altura: verde abajo, amarillo al medio y rojo arriba.
  - **Espejo estéreo:** el canal L crece hacia arriba y el R hacia abajo desde un eje central con las frecuencias.
  - **Onda:** osciloscopio de unos 23 ms que arranca en un cruce por cero ascendente, así un sonido periódico se ve quieto (`sincronizado`; si no hay cruce, `libre`). Escala fija de -1 a +1.
  - **Cascada:** espectrograma con una banda por fila (agudos arriba) y el tiempo hacia la izquierda, 10 columnas por segundo. La intensidad va en sombreado.
  - **Vectorscopio:** cada par L/R como punto en ejes M/S (un tema mono es una línea vertical), con escala automática y la correlación debajo.
  - **Medidores:** RMS (300 ms) y pico retenido (1,5 s) por canal en dBFS, con escala de -60 a 0 y la correlación.

  Con un FIFO de un canal, Espejo y Vectorscopio salen del ciclo y los Medidores muestran una sola barra. En pausa o detenido las formas caen a cero y se lee `sin señal`. En modo compacto se ocultan la fila de título, los ejes y las escalas. El FIFO solo se lee mientras la vista está activa, y solo se calcula la forma que se ve.
- **Letra:** la del tema que suena. Una letra sincronizada deja la línea actual en el medio, marcada con `▸`, y las ya cantadas apagadas; cambia de línea justo a tiempo. Una sin sincronizar se recorre con `j`/`k`, las páginas y `g`/`G`. El título dice si es `sincronizada` o `texto` y de dónde salió. Mientras busca se lee `buscando letra ◐`; si no la encuentra, lo dice (y por qué, si se sabe).
- **Línea inferior:** los atajos más útiles de la vista activa. Los avisos y errores la reemplazan durante 5 segundos.
- **Tamaño:** desde 100 × 28 se ve todo. Desde 60 × 16 hay un modo compacto: panel de 3 filas, pestañas abreviadas y, por debajo de 80 columnas, sin columna Álbum. Más chica, mpctui muestra el mínimo necesario y el tamaño actual.

### Atajos principales

| Teclas | Acción |
|---|---|
| `j` `k` / `↓` `↑` | Moverse; `g`/`G` principio/final; `AvPág`/`RePág` o `Ctrl+d`/`Ctrl+u` por páginas |
| `Enter` | Cola: reproducir · Biblioteca/Directorios: entrar o agregar · Búsqueda: agregar · Playlists: cargar |
| `l` / `h` | Columna siguiente/anterior · entrar/subir de directorio |
| `a` / `A` | Agregar la selección / agregar todos (resultados, temas del álbum) |
| `d` | Borrar de la cola · borrar playlist (pide confirmación) |
| `p` o `Espacio` | Pausa / reanudar |
| `s` · `>` · `<` | Detener · siguiente · anterior |
| `+` `-` · `f` `b` | Volumen ±5 · adelantar/retroceder 5 s |
| `z` `r` `y` `R` | Aleatorio · repetir · single (no, sí, una vez) · consumir |
| `/` | Buscar (`Tab` cambia el campo: todo, artista, álbum, título) |
| `S` | Guardar la cola como playlist (si el nombre no es válido, el diálogo lo dice y sigue abierto) |
| `c` | Vaciar la cola (pide confirmación) |
| `o` | Ir al tema que suena |
| `?` | Ayuda completa |
| `q` / `Ctrl+c` | Salir |

### Conexión

- La pantalla se actualiza sola cuando MPD informa un cambio, aunque venga de otro cliente.
- El encabezado muestra **● localhost:6600** (`(*)` en ASCII). Si se corta la conexión, mpctui reintenta solo con esperas crecientes (de 1 s a 30 s) y el encabezado muestra la cuenta regresiva: **✖ sin conexión, reintento en 5 s ■■■□□□□□ (intento 4)**. El medidor se llena a medida que pasa la espera, y el número de intento aparece desde el segundo. Mientras reintenta, se ve **✖ sin conexión, conectando ◐**. Si no entra todo, se quitan el intento, el medidor y `sin conexión, `, en ese orden. Al recuperarla, la línea inferior muestra `Conexión recuperada`.

### Visualizador

La vista Visual lee el PCM que MPD escribe en una salida fifo. Si no existe, la vista muestra qué agregar en `mpd.conf`:

```
audio_output {
    type   "fifo"
    name   "visualizador"
    path   "/tmp/mpd.fifo"
    format "44100:16:2"
}
```

Con otra ruta o formato, configurar `visualizer_fifo` y `visualizer_format`; la forma con la que arranca se elige con `visualizer_style`. Para ajustar la sensibilidad con música real: `cargo run --example spectrum_smoke` lee el FIFO 3 segundos con Barras y otros 3 con Medidores, e imprime el nivel máximo de cada banda, el RMS y el pico por canal y la correlación (no envía comandos a MPD).

### Letras

La vista Letra prueba las fuentes de `lyrics.sources` en orden y muestra la primera letra que encuentra. Al cambiar el tema, busca la del nuevo; volver a la vista con el mismo tema no busca de nuevo.

| Fuente | Dónde busca |
|---|---|
| `cache` | `~/.lyrics/Artista - Título.lrc` y, si no está, `.txt` (necesita los tags de artista y título) |
| `song_folder` | Junto al archivo del tema, con su mismo nombre: `Album/01 Tema.lrc` al lado de `Album/01 Tema.flac`. Necesita `music_directory`, porque MPD no informa dónde está la música si se conecta por TCP |
| `tags` | La letra embebida en el archivo (`LYRICS` o `UNSYNCEDLYRICS`, en FLAC y Ogg). En MP3, MPD no expone la letra (`USLT`) |
| `lrclib` | [lrclib.net](https://lrclib.net): gratis y sin registro, muchas letras sincronizadas. Pide la coincidencia exacta (artista, título, álbum y duración) y, si no hay, busca por artista y título y elige un resultado con una duración parecida (±2 s), prefiriendo los sincronizados. Lo descargado se guarda en la caché si `cache` está en `sources`; lo que no encontró se recuerda hasta cerrar mpctui, para no volver a pedirlo |

Si LRCLIB dice que el tema es instrumental, la vista muestra `Instrumental`.

> **Privacidad:** con `lrclib` en `sources`, mpctui envía a lrclib.net el artista, el título, el álbum y la duración de cada tema cuya letra no encuentra en las fuentes anteriores. Para no usar la red, sacar `lrclib` de la lista.

Los archivos `.lrc` pueden tener varias marcas por línea, metadatos (`[ar:…]`) y `[offset:…]`; las marcas por palabra del formato extendido se descartan. Un archivo sin marcas de tiempo se muestra como texto.

## Log

Los errores se registran en `~/.local/state/mpctui/mpctui.log`.

## Desarrollo

La arquitectura, los principios de diseño y las convenciones están en [`AGENTS.md`](AGENTS.md). El plan y su avance, en [`PLAN.md`](PLAN.md).

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --example smoke            # prueba de solo lectura contra un MPD real
cargo run --example spectrum_smoke   # bandas, RMS, pico y correlación leyendo el FIFO (solo lectura)
cargo run --example lyrics_smoke     # letra del tema actual en LRCLIB (no guarda nada ni modifica MPD)
```
