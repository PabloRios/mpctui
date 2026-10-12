# DECISIONES.md — mpctui

Registro de ideas pendientes y de decisiones tomadas durante la implementación que no
están en `DESIGN.md` ni en `DESIGN_EXTRA.md`.

---

## Ideas pendientes (sin implementar)

### Volumen en la línea inferior

- **Pedido:** mostrar el volumen en la línea inferior, después de `? ayuda`, alineado a la
  derecha, con un medidor de bloques crecientes como `▁▂▃▄▅▆▇`.
- **Estado:** anotado, sin implementar. Se retoma cuando el usuario lo pida.
- **A definir al retomarlo:**
  - Si reemplaza al medidor `vol ■■■■■■■□□□  70%` del panel o lo duplica.
  - El equivalente ASCII. Los bloques `▁…█` tienen ancho ambiguo, igual que el espectro.
  - Qué pasa cuando un aviso ocupa la línea inferior.

### Otras fuentes de letras sincronizadas

- **Pedido:** sumar otra fuente de letras sincronizadas además de LRCLIB.
- **Estado:** evaluado el 2026-10-12; se decidió seguir solo con LRCLIB (abierta, sin
  clave y legal). Se retoma cuando el usuario lo pida.
- **Candidatas y su problema:**
  - **NetEase Cloud Music:** API no oficial, sin clave, muchas letras en LRC (fuerte en
    música asiática). Es la mejor segunda opción si alguna vez hace falta.
  - **Musixmatch:** el catálogo más grande, pero solo con la API interna de su app de
    escritorio (token anónimo), contra sus términos. La API oficial gratuita no da
    letras sincronizadas.
  - **Megalobiz:** LRC de la comunidad, leyendo el HTML del sitio; frágil y de
    catálogo chico.
  - **QQ Music y Kugou:** APIs no oficiales, catálogo sobre todo chino; Kugou usa su
    formato KRC.
  - **Apple Music, Spotify y Deezer:** requieren cuenta o suscripción.
  - **Genius:** solo letras sin sincronizar.
- **A definir al retomarlo:**
  - Cómo se agregaría: un valor en `LyricsSource`, una implementación de
    `LyricsProvider`, su nombre en `LYRICS_SOURCE_NAMES` (`config.rs`) y en
    `config.toml.dist`. No entraría en las fuentes por defecto.
  - Cómo elegir el resultado correcto cuando el servicio devuelve nombres en otro idioma.

---

## Decisiones tomadas

### E1 · Formato de audio

- **Relectura de `status` cada 2 s:** se cuenta en ticks (`STATUS_POLL_TICKS`), no como
  tiempo desde la última lectura. La demora de cada tick corría la relectura a 3 s.
- **En pausa se mantiene el último formato sin lógica extra:** MPD sigue informando
  `audio` y `bitrate` en pausa (verificado contra el servidor real).
- **Códec en streams:** una ruta con `://` no muestra códec, aunque termine en `.mp3`.

### E3 · Cuenta regresiva del reintento

- **`ping` en `ChangesRepository`:** `idle` no sirve para confirmar que se recuperó la
  conexión, porque bloquea hasta el próximo cambio. El vigilante reintenta con `ping`
  y recién después vuelve a `idle`.
- **El vigilante es la única fuente del estado de conexión.** La app ya no reintenta en
  cada tick por su cuenta: resincroniza al recibir `Connected`. Así la cuenta regresiva
  coincide con el reintento real.
- **`Disconnected` reemplaza a `Unavailable`:** se extendió la variante existente en lugar
  de duplicarla. Conserva el motivo para el aviso de error.
- **El error de desconexión se avisa una sola vez:** solo en el primer intento. Los
  reintentos fallidos van al log, y el encabezado ya muestra la cuenta regresiva.
- **El spinner de "conectando" no se recorta:** el orden de recorte del diseño (intento,
  medidor, `sin conexión, `) solo nombra el medidor de la espera.
- **El recorte por ancho no alcanza en modo compacto:** a 72 columnas el texto completo
  entra. El compacto usa directamente la forma mínima, como en el mockup.

### E2 · Actualización de la base

- **`changed: update` relee solo `status`:** si en el mismo evento vino un subsistema del
  reproductor, ya se relee el estado completo y no se repite.
- **La transición se evalúa en cada estado leído** (eventos, relectura cada 2 s,
  reconexión), comparando con el anterior. Si al reconectar la actualización ya terminó,
  igual se avisa.

### E4 · Visualizador de espectro

- **La vista va en `src/tui/views/pages/spectrum.rs`,** no en `views/spectrum.rs` como
  dice la tabla del diseño. Las vistas viven en `pages/` desde la fase D5.
- **`SpectrumFrame` suma `signal: bool`.** La vista muestra `sin señal` cuando no hay
  datos y las barras ya cayeron a cero. Una canción en silencio con datos no lo muestra.
- **Cantidad de bandas:** la decide la vista según el ancho. La app la calcula después de
  cada dibujado (`views::spectrum_bands`) y se la pasa al visualizador por un `watch`.
  Un cuadro de otro ancho, como pasa durante un cambio de tamaño, se dibuja recortado
  hasta que llegue el siguiente.
- **`VisualizerLauncher` guarda un `Box<dyn Fn() -> S>`** que abre una fuente nueva en
  cada lanzamiento. Al cancelar la tarea se suelta el FIFO. Los traits de repositorio
  siguen consumiéndose con genéricos.
- **Backoff propio para el FIFO:** de 50 ms a 1 s, en lugar de 1 s a 30 s. Al reanudar
  la música, las barras tienen que volver enseguida.
- **`PEAK_FALL_PER_SECOND = 0.5`:** el diseño no fijaba el valor. Igual que `FLOOR_DB` y
  `CEIL_DB`, se ajusta con `cargo run --example spectrum_smoke`.
- **Mientras el FIFO no se puede leer, no se publican cuadros:** la vista queda en la
  ayuda hasta que llega el primer dato.
- **Línea inferior de la vista 6:** `p pausa   ? ayuda`. Las sugerencias salen de
  `BINDINGS`, con una tecla por acción, así que no aparece el `1..6 vistas` del mockup.
- **Resumen de la cola en las pestañas:** con seis pestañas a 100 columnas ya no entra y
  se oculta, como ya preveía la regla de D5. El mockup lo mostraba sin el `cola:`.

### E5 · Ciclo y formas del visualizador

- **La pestaña se llama `Visual`** (igual en modo compacto), como en el mockup.
- **La ayuda dice `Visualizador / otra forma`:** `Visualizador / siguiente forma` no entra en
  la columna de la ayuda a 100 columnas y se cortaba. La línea inferior de la vista 6
  sugiere `6 siguiente forma`, y el aviso al cambiar es el del diseño.
- **`view_action(vista)`:** la pestaña 6 busca su tecla en `ShowVisualizer`, no en
  `ShowView`. Las demás vistas no cambian.
- **Dónde va cada cosa:** la vista en `tui/views/pages/visual/` (las vistas viven en `pages/`)
  y el análisis en `service/visual/`. Barras y Espejo no tienen módulo propio en el servicio:
  son el analizador de bandas más `Smoother`, que ya está en `bands.rs` (KISS).
- **`VisualRequest` lo calcula la vista al dibujar,** no una función aparte con el tamaño de
  la terminal: cada forma conoce su área exacta (márgenes, ejes, puntos por celda). La vista
  lo deja en `state.visual.request` y la app se lo pasa al visualizador. En lugar de
  `width`/`height`, lleva `columns` (bandas o columnas de datos) y `rows` (bandas de la
  Cascada), que es lo que necesita el servicio.
- **`VisualFrame` = `VisualContent` + `signal`,** en lugar de una variante `Silent`. La vista
  muestra `sin señal` cuando no hay señal y la forma ya no tiene nada que dibujar, igual que
  hacían las barras.
- **Sin señal se vacía la ventana de muestras:** todas las formas ven silencio con la misma
  regla. Las barras caen con su velocidad de caída y la Cascada sigue avanzando con
  columnas vacías.
- **Un cuadro de otra forma no se dibuja:** durante el cambio de forma (un cuadro, 50 ms) la
  vista muestra solo el título.
- **Glifos:** `scope_guides` y `scope_density` del diseño quedaron dentro de
  `Plot::Cells` (`CellPlot`), porque solo existen en ASCII. En braille, las guías son puntos
  calculados. `BRAILLE_BASE` vive en `glyphs.rs`.
- **`HeldPeak` es compartido:** lo usan las barras (0,5 s y 0,5 alturas/s) y los Medidores.
- **Valores que el diseño no fijaba:** el pico de los Medidores cae a 20 dB/s;
  `VECTORSCOPE_GAIN_DECAY` es 0,5 por segundo; por debajo de 1e-4 de amplitud el
  Vectorscopio no dibuja puntos; la línea de correlación mide 41 celdas y se acorta si no
  entra. Se ajustan con música real.
- **Medidores en el piso:** el número se muestra como `—` en lugar de `-60.0`.
- **Onda continua:** cada columna incluye la última muestra de la anterior, así la línea no
  se corta cuando la señal salta entre columnas.
- **Cascada:** si cambia la cantidad de bandas (otra altura), el historial se descarta. Las
  etiquetas de tiempo terminan en la columna de ese instante; por eso `-1 s` queda a 10
  columnas de `ahora`.
- **Espejo en modo compacto:** sin letras `L`/`R` ni etiquetas; queda la línea del eje.
- **`spectrum_smoke`:** lee 3 s con Barras y otros 3 s con Medidores, porque el servicio
  solo calcula la forma activa.

### L1 · Letras locales

- **Letra embebida solo en FLAC/Ogg:** contra el MPD real (0.23), `readcomments` de un MP3
  devuelve solo frames `TXXX`; la letra `USLT` no aparece. Los comentarios Vorbis sí, con
  la clave tal como está en el archivo (`LYRICS`, `Lyrics`…).
- **Valores multilínea en el protocolo:** MPD imprime el valor de `readcomments` sin
  escapar. Una línea que no es un par con clave válida para MPD (letras, `_` y `-`, como
  su `IsValidName`) se toma como continuación del valor anterior; sin par previo sigue
  siendo `Error::Protocol`. Límite conocido: una línea de la letra con forma
  `Palabra: texto` se lee como otro comentario y se pierde de la letra.
- **Búsqueda sin cancelar:** una búsqueda empezada no se interrumpe al cambiar de tema
  (cortar un comando de MPD a la mitad desfasaría la conexión). El buscador atiende solo
  el último pedido y la app descarta resultados de un tema que ya no suena.
- **La caché usa el tag `Title`,** no el nombre del archivo: un tema sin artista o título
  no se busca en la caché (la vista lo explica), pero sí junto al archivo y en los tags.
- **Sin directorio del usuario:** si no se puede saber y no se configuró
  `lyrics.directory`, mpctui no arranca (`UnknownHomeDirectory`), igual que con cualquier
  otro valor de configuración inválido.
- **`[offset:…]` positivo adelanta la letra,** como dice el formato LRC. Un metadato
  desconocido entre corchetes con `:` se descarta; un `[Estribillo]` sin `:` queda como
  texto.
- **La línea actual usa `CURRENT_LYRIC` (= `ACCENT`)** y el cursor `▸`; las cantadas van en
  `MUTED`. Antes de la primera marca, la primera línea espera en el medio sin resaltar.
- **`7` con las letras desactivadas** avisa `Letras desactivadas (lyrics.enabled = false)`
  en lugar de no hacer nada.

### L2 · LRCLIB

- **`reqwest` 0.13 con `rustls`** (sin OpenSSL) y las features `json` y `query`. En 0.13 la
  feature se llama `rustls`, no `rustls-tls`; usa `aws-lc-rs` como proveedor de
  criptografía y compiló sin dependencias del sistema.
- **`/api/get` exige álbum y duración:** si al tema le falta alguno, se va directo a
  `/api/search` por artista y título.
- **Elección en la búsqueda:** entre los resultados con duración dentro de ±2 s (o
  cualquiera si no se sabe la duración), primero uno sincronizado y si no, cualquiera
  con letra o instrumental.
- **Memoria de la sesión:** se recuerda toda respuesta de LRCLIB (letra, "no hay" e
  instrumental), no solo los "no encontrado". Así, sin `cache` en las fuentes, volver a
  un tema no repite el pedido. Los errores de red no se recuerdan: se reintenta al
  volver al tema.
- **Los instrumentales no se guardan en disco:** el formato de la caché (`.lrc`/`.txt`)
  no tiene cómo decirlo. Se recuerdan solo durante la sesión.
- **Sin cancelar la descarga al cambiar de tema** (como en L1): una búsqueda empezada
  termina (como mucho 10 s por pedido) y su resultado se descarta si el tema ya cambió.
- **`Error::Http`** nuevo en el repositorio, con un texto corto para la vista
  (`LRCLIB: no se pudo conectar`, `no respondió a tiempo`, `respondió 500`).
- **`lyrics_smoke` no guarda nada:** usa el proveedor directo, sin el servicio ni la caché.
- **Primer uso real:** al probar la vista se creó `~/.lyrics/` con la letra del tema que
  sonaba (`Soulsavers - You Will Miss me when I Burn.lrc`), que es el comportamiento
  esperado de la caché.

### L3 · Letras fuera de la vista

- **Dos opciones de `[lyrics]`, `false` por defecto:** `fetch_in_background` busca la letra
  de cada tema aunque la vista 7 no esté activa; `now_playing_line` muestra la línea actual
  en el panel "ahora suena". La segunda implica buscar siempre (si no, la línea solo
  aparecería con la vista 7 abierta). Con `enabled = false` se ignoran, igual que `sources`.
- **Dónde viajan:** `Appearance.lyrics_line` (cómo se ve el panel; la factory ya lo cruza
  con `enabled`) y `Services.lyrics_in_background` (cuándo se usa el servicio). La
  decisión vive en dos funciones puras de `tui/app/lyrics.rs` (`needs_lyrics`,
  `shows_current_line`).
- **Ubicación en el panel:** después del título, `♪ Time · Ticking away…`. Al recortar se
  pierde primero el álbum, después la letra, el artista y el título. En compacto, la
  línea reemplaza al artista. Solo letras sincronizadas: sin sincronizar, antes de la
  primera línea, en una línea vacía o detenido, la fila queda como siempre.
- **`sync_lyrics` antes de dibujar:** antes iba después, y un tema nuevo podía mostrar
  hasta el próximo evento (≤ 1 s) la letra del anterior. Con la línea siempre a la vista
  en el panel, eso se notaba.
- **Privacidad:** con cualquiera de las dos y `lrclib` en las fuentes, se consulta cada
  tema que suena (si no está en las fuentes anteriores); la memoria de la sesión evita
  repetir pedidos.
- **Reintento con `Ctrl+r` (`Action::RetryLyrics`):** la `r` sola es "repetir" y casi todas
  las letras libres ya tienen uso; `Ctrl+r` se asocia con "recargar". Solo actúa en la
  vista Letra y solo si la búsqueda terminó en `NotFound` con un error
  (`LyricsStatus::can_retry`): un "no hay letra" de LRCLIB queda en la memoria de la
  sesión, así que reintentarlo no cambiaría nada. Vuelve a pedir la misma consulta a
  `LyricsLookup` (el `watch` marca el cambio aunque el valor sea igual). La tecla se
  sugiere debajo del error, no en la línea inferior: solo tiene sentido en ese estado.


### I1 · Interfaz en inglés y español

- **Catálogo tipado en el código, sin crates** (ni Fluent ni gettext): `Phrase` para los
  textos fijos, `Message` para los que llevan datos y un `match` exhaustivo por idioma.
  Con dos idiomas y unos 190 textos alcanza, y una traducción que falta no compila. Si
  algún día hacen falta muchos idiomas o traductores externos, se migra a Fluent sin
  cambiar `Catalog`.
- **Inglés por defecto** (`language = "en"`); `es` lo cambia. Sin `auto` ni variable de
  entorno: el pedido fue solo la opción del archivo.
- **`src/i18n/` es un módulo de primer nivel** y no parte de `tui/`: también lo usa `main`
  para explicar los errores de arranque (configuración y conexión).
- **Los errores viajan tipados hasta la TUI.** `repository::Error` pasó a ser
  `Clone + Eq` (`IoFailure` guarda el tipo y el texto del error de E/S) y `Http` lleva
  `HttpFailure` en vez de un texto. `ServerEvent::Disconnected`, `LyricsOutcome::NotFound`
  y `VisualState::Unavailable` llevan el `Error`; `Prompt::error` es un `InputError`. Así
  el visualizador ya no arma "no existe" en el servicio.
- **El `Display` de los errores queda en español para el log**, aunque repite lo que dice
  el catálogo en español: el repositorio no puede depender de `i18n`.
- **Lo que no se traduce:** el texto que viene del sistema operativo, de MPD (`ACK`), de
  `reqwest` y el detalle de un `Error::Protocol` se muestran tal cual detrás del prefijo
  traducido. Tampoco las unidades, LRCLIB ni `Enter`/`Tab`/`Ctrl+`.
- **Un error de configuración se explica en el idioma del archivo** si se pudo leer
  `language`; si el TOML no se puede parsear (o `language` es inválido), en inglés. El
  error de un valor inválido de `glyphs` dentro del TOML viene de serde y queda en
  español.
- **La tecla para confirmar** que se muestra cambia con el idioma (`[y] yes` / `[s] sí`),
  pero se aceptan las dos en cualquier idioma.
- **Las flechas del juego ASCII** se nombran con palabras del idioma (`Down`/`Abajo`):
  `Glyphs::arrow_keys` es `None` en ASCII.
- **Pestañas cortas en inglés** (`Lib.`, `Dir.`, `Srch.`, `Playl.`) no más largas que las
  españolas, para que el modo compacto entre igual.
- **Los tests de pantalla siguen en español** (fijan `Spanish`) y hay tests propios del
  inglés; el test de glifos ASCII recorre las pantallas en los dos idiomas.
