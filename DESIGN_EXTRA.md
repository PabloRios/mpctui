# DESIGN_EXTRA.md — mpctui: extras visuales

Cuatro agregados que `DESIGN.md` dejó fuera de alcance:

1. **Formato de audio y bitrate** en el panel "ahora suena".
2. **Indicador de actualización de la base de datos** (`updating_db`).
3. **Cuenta regresiva del próximo reintento** de conexión.
4. **Visualizador** leyendo la salida FIFO de MPD, con **seis formas** que se recorren con `6`.

> **Requisitos previos:** leer `AGENTS.md` y `DESIGN.md` completos. Este documento usa todo lo que define `DESIGN.md`: los dos juegos de glifos (Unicode por defecto, ASCII como fallback), `Glyphs`, `theme.rs`, `BarCell`/`MeterCell`, el panel "ahora suena" y la línea inferior. **Las fases de `DESIGN.md` (D1–D6) tienen que estar terminadas antes de empezar.** Si algo choca con `AGENTS.md`, gana `AGENTS.md` y se avisa al usuario.

Los mockups están a **100 columnas**. Están en Unicode y, donde la diferencia importa, también en ASCII.

---

## 1. Formato de audio y bitrate

### 1.1 Qué se ve

El formato va a la derecha del título del panel "ahora suena", en la misma línea del borde. Así no ocupa filas nuevas:

```text
├─ ▶ reproduciendo ─────────────────────────────────── FLAC  44.1 kHz  16 bit  estéreo  1411 kbps ─┤
├─ ▶ reproduciendo ───────────────────────────────────────────── MP3  44.1 kHz  estéreo  245 kbps ─┤
├─ ▶ reproduciendo ───────────────────────────────────── FLAC  96 kHz  24 bit  estéreo  2741 kbps ─┤
├─ ▶ reproduciendo ────────────────────────────────────────────────────────── DSF  DSD64  estéreo ─┤
├─ ▶ reproduciendo ──────────────────────────────────── OPUS  48 kHz  flotante  estéreo  128 kbps ─┤
├─ ▶ reproduciendo ────────────────────────────────────────────────────── 48 kHz  16 bit  estéreo ─┤
+- |> reproduciendo ---------------------------------- FLAC  44.1 kHz  16 bit  estéreo  1411 kbps -+
```

Orden fijo, con dos espacios entre partes. Una parte que no se conoce **se omite** (no se muestra `?` ni `n/d`):

| Parte | Origen | Regla de formato |
|---|---|---|
| Códec | extensión del `file` del tema actual | En mayúsculas: `FLAC`, `MP3`, `OGG`, `OPUS`, `M4A`, `DSF`. Sin extensión (streams `http://...`) → se omite |
| Frecuencia | `status` → `audio` | Hz a kHz sin ceros de más: `44100` → `44.1 kHz`, `48000` → `48 kHz`, `96000` → `96 kHz`. DSD: `DSD64`, `DSD128`… |
| Profundidad | `status` → `audio` | `16 bit`, `24 bit`, `32 bit`; `f` → `flotante`; en DSD se omite |
| Canales | `status` → `audio` | `1` → `mono`, `2` → `estéreo`, `n` → `n canales` |
| Bitrate | `status` → `bitrate` | `1411 kbps`. `0` o ausente → se omite |

- Detenido: no se muestra nada (MPD no informa `audio` ni `bitrate`).
- En pausa: se mantiene el último valor.
- Si no entra en el borde, se quitan partes **desde la izquierda** (el códec primero, el bitrate último). Nunca se trunca una parte por la mitad.
- En modo compacto (`DESIGN.md` §7) no se muestra.

### 1.2 Datos de MPD

`status` devuelve, mientras hay reproducción:

```text
audio: 44100:16:2        frecuencia:bits:canales
audio: 96000:f:2         f = punto flotante
audio: dsd64:2           DSD: tasa:canales
bitrate: 1411            kbps, instantáneo (varía en VBR)
```

Parseo estricto, como pide `AGENTS.md` §3: un `audio` que no tenga ninguna de esas formas es `Error::Protocol`.

```rust
// src/domain/
pub enum SampleFormat { Integer { bits: u8 }, Float, Dsd { rate: u16 } }  // rate: 64, 128, ...
pub struct AudioFormat { pub sample_rate: Option<u32>, pub format: SampleFormat, pub channels: u8 }
// Status gana: audio: Option<AudioFormat>, bitrate: Option<u32>  (0 → None)
```

Claves nuevas en `vocabulary.rs` (`keys::AUDIO`, `keys::BITRATE`). El códec **no** se parsea del servidor: es una función de presentación en `format.rs` sobre la ruta del tema.

### 1.3 Actualización del bitrate

`bitrate` cambia cada segundo en archivos VBR, pero MPD no emite ningún evento `idle` por eso. Regla:

- Mientras el estado es *reproduciendo*, el `PlaybackService` vuelve a pedir `status` cada `STATUS_POLL_INTERVAL` (constante, 2 s), usando el tick que ya existe en el loop.
- Pausado o detenido: no se consulta nada.
- Es un comando de solo lectura, va por la conexión del propio servicio (no por la de `idle`) y de paso corrige la deriva de `PlaybackClock`.

---

## 2. Indicador de actualización de la base de datos

### 2.1 Qué se ve

Mientras MPD actualiza la base, el título del bloque del encabezado lo indica con el spinner y el tiempo transcurrido:

```text
┌─ mpctui ─ ◐ actualizando biblioteca 0:42 ───────────────────────────────────── ● localhost:6600 ─┐
+- mpctui - / actualizando biblioteca 0:42 ----------------------------------- (*) localhost:6600 -+
```

- El spinner (`glyphs.spinner`) avanza un cuadro por tick. El tiempo se cuenta desde que mpctui **vio** la actualización (MPD no informa cuándo empezó ni cuánto falta, así que no hay porcentaje ni barra: sería inventado).
- Al terminar: desaparece el indicador y la línea inferior muestra `· Biblioteca actualizada (1:05)` durante `MESSAGE_DURATION`. Las vistas Biblioteca y Directorios se refrescan con `SelectionPolicy::Keep` (eso ya ocurre con el evento `database`).
- En modo compacto queda solo el spinner junto a `mpctui` (ver §6).

### 2.2 Datos de MPD

- `status` incluye `updating_db: <id de trabajo>` solo mientras hay una actualización en curso. Se parsea como `Option<u32>` (estricto: un valor no numérico es `Error::Protocol`).
- `idle` informa `changed: update` cuando una actualización **empieza o termina**. Hoy ese subsistema se ignora: se mapea a un nuevo `Subsystem::Update`. Al recibirlo, se vuelve a pedir `status` para saber cuál de las dos cosas pasó.
- `changed: database` sigue significando "la base cambió" y se procesa como hasta ahora.

Estado en la TUI: `AppState` guarda `database_update: Option<Instant>` (cuándo se vio por primera vez). El cálculo de "empezó / terminó" a partir de dos `status` sucesivos es lógica de negocio: va en un servicio, no en la vista.

### 2.3 Fuera de alcance

Disparar una actualización desde mpctui (`update`) no formaba parte de esto. El usuario lo pidió después: la tecla `u` envía `update` y el indicador sigue llegando por `idle`.

---

## 3. Cuenta regresiva del próximo reintento

### 3.1 Qué se ve

Reemplaza al `sin conexión, reintentando ◐` de `DESIGN.md` §3.3. Dos estados, a la derecha del título del encabezado:

```text
┌─ mpctui ───────────────────────────────── ✖ sin conexión, reintento en 5 s ■■■□□□□□ (intento 4) ─┐
┌─ mpctui ────────────────────────────────────────────── ✖ sin conexión, conectando ◐ (intento 5) ─┐
+- mpctui ----------------------------- (x) sin conexión, reintento en 5 s [###-----] (intento 4) -+
+- mpctui -------------------------------------------- (x) sin conexión, conectando / (intento 5) -+
```

| Estado | Texto | Gráfico |
|---|---|---|
| Esperando | `sin conexión, reintento en N s` | Medidor de 8 celdas (`DESIGN.md` §4.3) con la parte **ya esperada** de la espera actual: se llena hasta que se reintenta |
| Conectando | `sin conexión, conectando` | Spinner |
| Ambos | `(intento N)` al final, desde el segundo intento | — |

- `N` se redondea **hacia arriba**: con 0,3 s restantes se ve `1 s`, nunca `0 s`.
- Cuando la espera supera 59 s se muestra `m:ss` (con el backoff actual de 1 s → 30 s no pasa, pero la función de formato no debe asumirlo).
- Al reconectar: el encabezado vuelve a `● host` y la línea inferior muestra `· Conexión recuperada` durante `MESSAGE_DURATION`.
- Si no entra todo, se quita primero `(intento N)`, después el medidor, después `sin conexión, `. Lo último que queda es `✖ reintento en 5 s`.

### 3.2 Cambios en el servicio

Hoy `ChangeWatcher` reintenta con `Backoff` exponencial pero la TUI no sabe cuándo será el próximo intento. Se amplía `ServerEvent` (si ya hay una variante de desconexión, se extiende en lugar de duplicarla):

```rust
pub enum ServerEvent {
    // ... variantes existentes ...
    Disconnected { attempt: u32, retry_at: Instant, delay: Duration },
    Connecting { attempt: u32 },
    Connected,
}
```

- `Instant` es `tokio::time::Instant`, para poder testear con `tokio::time::pause()`.
- La TUI calcula lo que falta en cada tick (`retry_at - now`) y el medidor como `1 - restante / delay`. No hace falta ningún tick extra: con el de 1 s alcanza para una cuenta en segundos.
- El estado de conexión en `AppState` pasa a ser un enum (`Connected`, `Waiting { attempt, retry_at, delay }`, `Connecting { attempt }`) en lugar de un booleano.

### 3.3 Fuera de alcance

"Reintentar ahora" con una tecla sería útil con esperas de 30 s, pero es un atajo nuevo: se agrega solo si el usuario lo pide.

---

## 4. Visualizador

### 4.1 Qué se ve

Una vista nueva, **`6` Visual**, que entra en el ciclo de `Tab` / `Shift+Tab`. El panel "ahora suena" sigue abajo, igual que en las demás vistas. La vista tiene **seis formas** que se recorren en ciclo apretando `6` otra vez (§5). Esta sección describe la primera, **Barras**, y todo lo que comparten las seis: configuración, lectura del FIFO y arquitectura.

Unicode (bloques de octavos `▁▂▃▄▅▆▇█`, 8 niveles por celda):

```text
┌─ mpctui ────────────────────────────────────────────────────────────────────── ● localhost:6600 ─┐
│  1 Cola  2 Biblioteca  3 Directorios  4 Búsqueda  5 Playlists [6 Visual]    1229 temas  65:04:13 │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Visualizador: Barras  ● ○ ○ ○ ○ ○  1/6                                 32 bandas  40 Hz - 16 kHz │
│ ▔▔ ▁▁ ▔▔    ▔▔ ▇▇ ▔▔ ▔▔ ▔▔                                                                       │
│ ▄▄ ██       ▇▇ ██ ▅▅                                                                             │
│ ██ ██ ▄▄ ▔▔ ██ ██ ██ ██ ██                                                                       │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ▂▂    ▔▔ ▔▔                      ▔▔                                   │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ▔▔    ▅▅ ▔▔ ▔▔ ▔▔ ▔▔       ▔▔       ▔▔ ▔▔                          │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██    ▄▄ ██ ▇▇    ▆▆ ▃▃ ▔▔    ▁▁ ▇▇ ▂▂ ▃▃ ▁▁                          │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██       ██ ██ ██ ██ ██                          │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ▄▄ ▂▂ ██ ██ ██ ██ ██ ▔▔    ▔▔ ▂▂              │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ▆▆ ▔▔ ▃▃ ██ ▔▔ ▔▔ ▔▔ ▔▔  │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ▂▂ ██ ██ ▇▇    ▅▅     │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██  │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██  │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██  │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██  │
│ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██  │
│       63          125      250         500         1k       2k          4k          8k       16k │
├─ ▶ reproduciendo ─────────────────────────────────── FLAC  44.1 kHz  16 bit  estéreo  1411 kbps ─┤
│ ♪ Time                                             Pink Floyd - The Dark Side of the Moon (1973) │
│ tema   ━━━━━━━━━━━━━━━━━━●─────────────────────────────────────                      2:13 / 6:53 │
│ álbum  ━┃━━━┃━━━━┃━━●─────┃─────┃───────┃─────────┃────┃────┃──         tema 4/10   9:52 / 42:51 │
│ cola   …┃━━━━━┃━━━━━━━┃━━━━━━━┃━●────┃────────┃──────┃───────┃…  álbum 12/87  8:11:05 / 65:04:13 │
│ vol ■■■■■■■□□□  70%     z ● aleatorio   r ○ repetir   y ◐ single   R ○ consumir     pos 304/1229 │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
 1..6 vistas   p pausa   ? ayuda
```

ASCII (3 niveles por celda: `.`, `:`, `#`):

```text
+- mpctui -------------------------------------------------------------------- (*) localhost:6600 -+
|  1 Cola  2 Biblioteca  3 Directorios  4 Búsqueda  5 Playlists [6 Visual]    1229 temas  65:04:13 |
+--------------------------------------------------------------------------------------------------+
| Visualizador: Barras  * . . . . .  1/6                                 32 bandas  40 Hz - 16 kHz |
| -- -- --    -- ## -- -- --                                                                       |
| :: ##       ## ## ::                                                                             |
| ## ## :: -- ## ## ## ## ##                                                                       |
| ## ## ## ## ## ## ## ## ## ..    -- --                      --                                   |
| ## ## ## ## ## ## ## ## ## ## --    :: -- -- -- --       --       -- --                          |
| ## ## ## ## ## ## ## ## ## ##    .. ## ##    :: .. --       ## .. ..                             |
| ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ##       ## ## ## ## ##                          |
| ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## .. .. ## ## ## ## ## --    -- ..              |
| ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## :: -- .. ## -- -- -- --  |
| ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## .. ## ## ##    ::     |
| ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ##  |
| ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ##  |
| ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ##  |
| ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ##  |
| ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ##  |
|       63          125      250         500         1k       2k          4k          8k       16k |
+- |> reproduciendo ---------------------------------- FLAC  44.1 kHz  16 bit  estéreo  1411 kbps -+
| ~ Time                                             Pink Floyd - The Dark Side of the Moon (1973) |
| tema   [=================>------------------------------------]                      2:13 / 6:53 |
| álbum  [=|===|====|==>----|-----|-------|--------|----|----|--]         tema 4/10   9:52 / 42:51 |
| cola   [..|=====|======|======|=>----|-------|------|------|..]  álbum 12/87  8:11:05 / 65:04:13 |
| vol [#######---]  70%     z[x] aleatorio  r[ ] repetir  y[1] single  R[ ] consumir  pos 304/1229 |
+--------------------------------------------------------------------------------------------------+
 1..6 vistas   p pausa   ? ayuda
```

Elementos de la forma Barras:

- **Fila de título** (común a las seis formas): `Visualizador: <forma>`, el indicador de páginas `● ○ ○ ○ ○ ○` / `* . . . . .` con `1/6` y, a la derecha, los datos propios de la forma (acá: cantidad de bandas y rango de frecuencias).
- **Barras:** una por banda, de `SPECTRUM_BAR_WIDTH` (2) columnas con 1 de separación, alineadas abajo. La cantidad de bandas sale del ancho: `bandas = (ancho + 1) / 3`, entre `SPECTRUM_MIN_BANDS` (8) y `SPECTRUM_MAX_BANDS` (64). El sobrante se reparte a los costados para centrar.
- **Resolución vertical:** la altura de cada barra se calcula en sub-celdas (`alto × 8` en Unicode, `alto × 3` en ASCII); la celda de arriba de la barra usa el glifo parcial que corresponda.
- **Picos:** un `▔`/`-` marca el valor máximo reciente de cada banda. Se queda `PEAK_HOLD` (0,5 s) y después cae a `PEAK_FALL_PER_SECOND`. Solo se dibuja si esa celda está vacía.
- **Eje de frecuencias:** etiquetas `63 125 250 500 1k 2k 4k 8k 16k` debajo de la banda más cercana a cada frecuencia. Si dos etiquetas se pisan, se omite la de la derecha.
- **Color** (16 ANSI, en `theme.rs`): por altura, el tercio de abajo verde, el del medio amarillo y el de arriba rojo. El color solo refuerza: la altura ya transmite el nivel (principio 2 de `DESIGN.md`).
- **Sin señal** (pausado, detenido o sin datos en `SILENCE_TIMEOUT` = 250 ms): las barras caen hasta cero con la misma velocidad de caída y, ya en cero, se muestra `sin señal` centrado.

Si el FIFO no existe o no se puede abrir, la vista lo explica en lugar de fallar (en cualquiera de las formas):

```text
│ Visualizador                                                                           sin señal │
│                                                                                                  │
│   No se puede leer /tmp/mpd.fifo (no existe).                                                    │
│                                                                                                  │
│   Agregá una salida fifo en mpd.conf y reiniciá MPD:                                             │
│                                                                                                  │
│       audio_output {                                                                             │
│           type   "fifo"                                                                          │
│           name   "visualizador"                                                                  │
│           path   "/tmp/mpd.fifo"                                                                 │
│           format "44100:16:2"                                                                    │
│       }                                                                                          │
│                                                                                                  │
│   Si usás otra ruta o formato, configurá visualizer_fifo y visualizer_format.                    │
│                                                                                                  │
```

Con terminal chica (modo compacto de `DESIGN.md` §7) el visualizador sigue disponible: se oculta la fila de título y los ejes, y cada forma se adapta según §5.10.

### 4.2 Configuración

```toml
# ~/.config/mpctui/config.toml
visualizer_fifo   = "/tmp/mpd.fifo"   # defecto
visualizer_format = "44100:16:2"      # defecto; debe coincidir con el format de la salida fifo de MPD
visualizer_style  = "bars"            # defecto; forma con la que arranca la vista (§5.1)
```

- Fail fast en el borde: `visualizer_format` se valida al cargar la config con un newtype en `src/domain/inputs.rs` (`PcmFormat`). Solo se aceptan **16 bits** y **1 o 2 canales**, con frecuencia entre 8000 y 192000. Cualquier otra cosa no arranca y explica el motivo.
- `visualizer_style` acepta `bars`, `mirror`, `wave`, `waterfall`, `vectorscope` o `meters` (en inglés, como `glyphs`). Otro valor no arranca. Con un FIFO de 1 canal, elegir `mirror` o `vectorscope` también es error de configuración: esas formas necesitan dos canales.
- `visualizer_fifo` vacío es un error de configuración (no un valor que "desactiva" la vista: YAGNI).
- Un FIFO inexistente **no** es error de configuración: se resuelve en tiempo de ejecución con la pantalla de ayuda de arriba, porque MPD puede crearlo después.

### 4.3 Lectura del FIFO

- MPD escribe PCM crudo, entrelazado, con signo, en el **orden de bytes nativo** (`i16::from_ne_bytes`), en el formato de la salida fifo de `mpd.conf`.
- Se abre en modo no bloqueante con `tokio::net::unix::pipe::OpenOptions::new().open_receiver(ruta)`: abrir para lectura no bloquea aunque MPD todavía no esté escribiendo.
- Las lecturas pueden cortar un frame por la mitad (4 bytes en 16 bit estéreo): los bytes sobrantes se guardan para la lectura siguiente.
- Se guardan los últimos `ANALYSIS_FRAMES` (2048) frames **estéreo** en un búfer circular (`StereoWindow`). Las formas que trabajan en mono lo derivan de ahí; las que necesitan los dos canales (Espejo, Vectorscopio, Medidores) los usan por separado.
- Al abrir o al volver de una pausa, lo que haya acumulado en el pipe es viejo: se lee todo y se descarta todo menos lo último.
- **Fin de archivo (0 bytes):** MPD cerró su lado (se detuvo o se reinició). Se reabre usando el `Backoff` que ya existe en `src/service/` (DRY), sin mostrar error: la vista pasa a "sin señal".
- **Solo lectura:** leer el FIFO no cambia nada en MPD, así que **sí** se puede probar contra el servidor real (ver §7).

### 4.4 Análisis de la forma Barras

Pasos por cuadro, todos en funciones puras. Los pasos 1 a 6 forman el **analizador de bandas** que también usan Espejo (una vez por canal) y Cascada (DRY):

1. **Mono:** promedio de los canales, normalizado a `f32` en [-1, 1].
2. **Ventana de Hann** sobre los últimos `ANALYSIS_FRAMES` (2048) frames: unos 46 ms a 44,1 kHz, con resolución de unos 21,5 Hz por bin.
3. **FFT real** con el crate `rustfft` (dependencia nueva, justificada: implementar una FFT a mano va contra KISS).
4. **Magnitud por bin:** `|X| × 2 / suma_de_la_ventana`.
5. **Bandas logarítmicas** entre `SPECTRUM_MIN_HZ` (40) y `SPECTRUM_MAX_HZ` (16000): `borde_i = min × (max/min)^(i/bandas)`. Cada banda toma el **máximo** de los bins que caen en su rango; si una banda grave no contiene ningún bin, toma el bin más cercano a su centro.
6. **Decibeles y normalización:** `dB = 20·log10(magnitud + ε)`, `nivel = clamp((dB − FLOOR_DB) / (CEIL_DB − FLOOR_DB), 0, 1)` con `FLOOR_DB = -60` y `CEIL_DB = 0`. Son constantes para ajustar con música real usando el ejemplo de §7.
7. **Suavizado:** si el nivel nuevo es mayor, se toma enseguida; si es menor, baja como mucho `FALL_PER_SECOND` (1,5 alturas completas por segundo) × tiempo transcurrido. Los picos, igual, con su propia caída.

### 4.5 Arquitectura

Respeta las capas de `AGENTS.md`: el FIFO es una **fuente de datos**, así que va en `repository`. El análisis es lógica, así que va en `service`. La vista solo dibuja.

| Capa | Archivo | Contenido |
|---|---|---|
| domain | `src/domain/inputs.rs` | `PcmFormat` validado (frecuencia, 16 bit, canales) |
| domain | `src/domain/visual.rs` | `VisualizerStyle` (§5.9) y `VisualFrame`, un cuadro listo para dibujar |
| repository | `src/repository/traits.rs` | Trait angosto `PcmSource { fn read(&mut self, destination: &mut [u8]) -> impl Future<Output = Result<usize>> + Send; }` |
| repository | `src/repository/fifo.rs` | `FifoPcmSource`, abre y lee el pipe. **No** va en `mpd/`: no es el protocolo de MPD |
| service | `src/service/visual/` | Funciones puras: decodificar PCM, `StereoWindow` y un módulo por forma (§5.9) |
| service | `src/service/visualizer.rs` | `Visualizer<S: PcmSource>`: tarea que lee, analiza a `VISUALIZER_FPS` (20) y publica el último `VisualFrame` |
| tui | `src/tui/views/visual/` | Un archivo por forma, más los dibujos compartidos (§5.9) |
| tui | `src/tui/glyphs.rs` | Campos nuevos (§6) |
| factory | `src/factory.rs` | Arma `FifoPcmSource` + `Visualizer` con la config y lo inyecta |

Decisiones de concurrencia:

- El `Visualizer` publica en un **`tokio::sync::watch`**: solo importa el último cuadro, no hay cola que se acumule si la TUI va más lenta.
- La TUI le avisa al `Visualizer` qué forma está activa y de qué tamaño es el área por **otro `watch`** (`VisualRequest { style, width, height }`), porque la cantidad de bandas, de columnas y de puntos depende de eso. El servicio calcula **solo** lo que necesita la forma activa.
- La tarea **solo corre mientras la vista 6 está activa.** Al entrar se lanza; al salir se cancela (`JoinHandle::abort` o un `CancellationToken`). Fuera de esa vista no se lee el FIFO ni se gasta CPU.
- El loop `select!` de `App` agrega una rama para el `watch` y redibuja al recibir un cuadro. Las demás vistas no cambian su frecuencia de dibujado.
- `6` y la vista nueva se agregan **solo en la tabla `BINDINGS`**: la ayuda y la línea inferior se actualizan solas.

---

## 5. Formas del visualizador (cambian con `6`)

### 5.1 Cómo se cambia de forma

- **`6` desde otra vista:** entra al visualizador con la **última forma usada** en esta sesión. Al arrancar mpctui es la de `visualizer_style` (defecto `bars`).
- **`6` estando en el visualizador:** pasa a la siguiente forma, en ciclo:

  **Barras → Espejo estéreo → Onda → Cascada → Vectorscopio → Medidores → Barras…**

- **Indicador:** la fila de título muestra el nombre, `● ○ ○ ○ ○ ○` (`* . . . . .` en ASCII) con la forma activa marcada, y `n/6`.
- **Aviso:** al cambiar, la línea inferior muestra `· Visualizador: Cascada (6: siguiente forma)` durante `MESSAGE_DURATION`.
- **FIFO de 1 canal:** Espejo estéreo y Vectorscopio necesitan dos canales, así que **salen del ciclo** (quedan 4 formas y el indicador muestra `n/4`). Medidores muestra una sola barra `M`.
- **Historial:** al cambiar de forma se reinicia todo lo acumulado (picos, historial de la Cascada, escala automática). No se calcula nada para las formas que no se ven.
- **Ayuda:** `6  visualizador / siguiente forma`.
- La forma elegida **no** se guarda al salir de mpctui (YAGNI); para cambiar la de arranque está `visualizer_style`.

Implementación del atajo: `BINDINGS` asocia `6` a una acción `ShowVisualizer`. `App` la interpreta: si la vista activa no es el visualizador, cambia de vista; si ya lo es, `style = style.next(channels)`. Solo el visualizador se comporta así; las demás teclas de vista no cambian.

### 5.2 Resumen de las formas

| # | Forma | Qué muestra | Datos que usa | Costo | Sirve para |
|---|---|---|---|---|---|
| 1 | **Barras** | Espectro por bandas | FFT mono | 1 FFT por cuadro | Vista general |
| 2 | **Espejo estéreo** | Espectro del canal izquierdo hacia arriba y del derecho hacia abajo | FFT por canal | 2 FFT | Comparar los dos canales |
| 3 | **Onda** | Forma de onda (osciloscopio) | Muestras crudas, mono | Sin FFT | Ver ataques, timbre y recortes |
| 4 | **Cascada** | Espectrograma: el tiempo corre hacia la izquierda y la intensidad es sombreado | FFT mono + historial | 1 FFT | Ver ritmo y estructura de la canción |
| 5 | **Vectorscopio** | Imagen estéreo (goniómetro M/S) y correlación | Pares L/R | Sin FFT | Ancho estéreo, mono, problemas de fase |
| 6 | **Medidores** | Nivel RMS y pico por canal en dBFS, y correlación | Muestras por canal | Sin FFT | Niveles reales |

### 5.3 Barras

Descrita en §4.1 y §4.4.

### 5.4 Espejo estéreo

```text
│ Visualizador: Espejo estéreo  ○ ● ○ ○ ○ ○  2/6            31 bandas por canal  L arriba, R abajo │
│    ▄▄ ▄▄       ▄▄ ██ ▄▄                                                                          │
│    ██ ██ ██ ▄▄ ██ ██ ██ ██ ██                                                                    │
│    ██ ██ ██ ██ ██ ██ ██ ██ ██ ██    ▄▄ ██ ▄▄    ▄▄ ▄▄       ▄▄ ██ ▄▄                             │
│ L  ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ▄▄ ▄▄ ██ ██ ██ ██ ██          ▄▄           │
│    ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ▄▄ ██ ██ ▄▄    ▄▄  │
│    ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██  │
│    ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██  │
│   ────── 63 ───────────────── 250 ──────────────── 1k ───────────────── 4k ──────────────── 16k  │
│    ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██  │
│    ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ▀▀ ▀▀ ██  │
│    ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ▀▀ ██ ██ ██ ▀▀ ▀▀ ▀▀           │
│ R  ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ▀▀          ██ ██ ▀▀                ██                       │
│    ██ ██ ██ ▀▀       ▀▀ ██ ██ ██                ▀▀ ▀▀                                            │
│    ██ ██ ▀▀             ▀▀ ██ ▀▀                                                                 │
│    ▀▀ ██                   ▀▀                                                                    │
```

```text
| Visualizador: Espejo estéreo  . * . . . .  2/6            31 bandas por canal  L arriba, R abajo |
|    .. ..       .. ## ..                                                                          |
|    ## ## ## .. ## ## ## ## ##                                                                    |
|    ## ## ## ## ## ## ## ## ## ##    .. ## ..    .. ..       .. ## ..                             |
| L  ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## .. .. ## ## ## ## ##          ..           |
|    ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## .. ## ## ..    ..  |
|    ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ##  |
|    ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ##  |
|   ------ 63 ----------------- 250 ---------------- 1k ----------------- 4k ---------------- 16k  |
|    ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ##  |
|    ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## '' '' ##  |
|    ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## ## '' ## ## ## '' '' ''           |
| R  ## ## ## ## ## ## ## ## ## ## ## ''          ## ## ''                ##                       |
|    ## ## ## ''       '' ## ## ##                '' ''                                            |
|    ## ## ''             '' ## ''                                                                 |
|    '' ##                   ''                                                                    |
```

- Mitad de arriba: canal **L**, las barras crecen hacia arriba desde el eje. Mitad de abajo: canal **R**, crecen hacia abajo. Cada mitad tiene `(alto − 1) / 2` filas.
- **Eje central:** la fila del medio es el eje de frecuencias, con las etiquetas dentro de la línea (`── 63 ── 250 ── 1k ──`).
- **Resolución:** media celda. Unicode no tiene bloques "superiores" de octavos, así que las dos mitades usan medias celdas para verse simétricas: arriba `▄ █`, abajo `▀ █`. En ASCII, arriba `. #`, abajo `' #`.
- Misma distribución de bandas que Barras (misma función, DRY), dos análisis de bandas (uno por canal), sin marcas de pico (KISS).
- Las letras `L` y `R` van en el margen izquierdo, a mitad de su zona.

### 5.5 Onda (osciloscopio)

```text
│ Visualizador: Onda  ○ ○ ● ○ ○ ○  3/6                                         23 ms  sincronizado │
│ +1                                                                                               │
│                                                                        ⢰⠛⣆                       │
│                                           ⢠⠏⠳⡄                        ⢀⡏ ⠸⡄                      │
│        ⢀⡞⠙⣆                               ⡞  ⠙⠒⢦⡀                     ⡼   ⢳⡀                     │
│      ⣀⣠⠞  ⠸⡄ ⣠⠶⣄ ⢀⡤⣄                     ⢸⠁     ⢧                    ⣰⠃    ⠓⢦⡀                   │
│     ⣸⠁     ⠳⠴⠃ ⠘⠦⠞ ⠸⡄                 ⡤⢤⣠⠏      ⠸⡄                 ⢀⡞⠁       ⢳                   │
│                     ⢧            ⡴⢦⡀ ⡼⠁          ⣇                 ⡞         ⠈⣇                ⢀ │
│  0                  ⠸⡄          ⢸⠁ ⠙⠚⠁           ⢸                ⢰⠃          ⢸⡀ ⣠⢤⡀           ⢸ │
│                      ⢧         ⢀⡏                ⠈⡇               ⡞            ⠳⠞⠁ ⢳⡀          ⡏ │
│                      ⠈⢧⡀       ⡼                  ⢹⡀     ⡴⠲⡄ ⢠⠖⢦ ⢰⠃                 ⠓⠚⠙⣆      ⢰⠃ │
│                        ⠹⡄    ⡤⠞⠁                   ⠉⠙⢦  ⢰⠃ ⠙⠶⠋ ⠈⠓⠋                     ⢸⡀     ⡞  │
│                         ⢳   ⡼⠁                       ⠈⢧⣠⠏                               ⢧  ⣠⠤⠞⠁  │
│                         ⠈⣇ ⢰⠃                                                           ⠘⣆⡴⠃     │
│                          ⠸⣤⠏                                                                     │
│ -1                                                                                               │
```

```text
| Visualizador: Onda  . . * . . .  3/6                                         23 ms  sincronizado |
| +1                                                                                               |
|                                                                         '.                       |
|                                            ''                          '  |                      |
|         -'.                               -  '--                      .   -                      |
|      ..-   | .-.  -.                      |     -                    .     '-                    |
|     .      '-   -'  |                 ...'       |                  -        -                   |
|                     -            --  -           .                 -          .                  |
|  0                   |           | '-             |                |           | .-              |
|                      -          '                 |               -            --  -           ' |
|                       -        -                  '      .-   -.  |                 '''.       | |
|                        '     .-                    ''-   | '-'  -'                      |     -  |
|                         '   -                         -.'                               -  .--   |
|                          .  |                                                            ..      |
|                           .'                                                                     |
| -1                                                                                               |
```

- **Ventana de tiempo:** `WAVE_FRAMES` (1024) frames, unos 23 ms a 44,1 kHz. Se muestra en el título.
- **Sincronización:** la ventana empieza en el primer cruce por cero ascendente (`muestra[i−1] < 0 ≤ muestra[i]`) dentro de la primera mitad del búfer. Así un sonido periódico se ve quieto en lugar de correr. Si no hay cruce, empieza en 0 y el título dice `libre` en lugar de `sincronizado`.
- **Muestreo por columna:** cada columna de puntos cubre un tramo de muestras y dibuja el segmento vertical entre su **mínimo y su máximo**. Así la línea queda continua y no se pierden picos (no se toma una muestra suelta por columna).
- **Unicode — braille:** cada celda es una matriz de 2 × 4 puntos (`U+2800`–`U+28FF`), o sea el doble de resolución horizontal y cuatro veces la vertical. El braille tiene ancho "neutral" en Unicode (no ambiguo), así que no sufre el problema de §6, pero hay fuentes que no lo traen: otro caso para ASCII.
- **ASCII:** 3 sub-niveles por celda (`'` arriba, `-` medio, `.` abajo) y `|` para unir saltos verticales de más de una celda.
- **Escala fija** (`WAVE_GAIN` = 1,0): un tema bajo se ve chico a propósito. Etiquetas `+1`, `0`, `-1` a la izquierda.

### 5.6 Cascada (espectrograma)

```text
│ Visualizador: Cascada  ○ ○ ○ ● ○ ○  4/6                                   14 bandas  últimos 9 s │
│ 16k           ░   ░░  ░ ░░ ░ ░  ░ ░         ░  ░ ░  ░ ░░  ░░   ░░░ ░ ░ ░░ ░   ░░    ░   ░░  ░ ░  │
│            ░░ ░  ░░░   ░░░░░░░░░░░░  ░ ░    ░    ░░   ░ ░░░░    ░  ░░░░ ░░░░   ░░░  ░░░░░░ ░  ░  │
│            ░░░░░░░░░░░░ ░░░░░░░░░░░░░░ ░ ░░ ░░░░░░ ░░░░░░░░░░░░░░ ░░░░░░░░░░ ░░░░ ░░░░░░░░░░░░░░ │
│  4k        ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│            ░░░░░░░░░░░░░░░░░▒▒▒▒▓▒▓▓▓▓▓▒▒▒▓▒▒▒▒▒▒░░░░░░░░░░░░░░░░░░░░░░░░░░░░▒▒▒▒▓▓▓▓▓▓▓▓▒▓▓▒▒▒░ │
│            ░░░░░░░░▒▒▒▒▒▒▒▓▓▓▓▓▓▓▓▒▓▓▒▒▒▒▒░░░░░░░░░░░░░░░░░░░░░░░░▒▒░▒▒▒▒▒▒▓▓▓▓▓▓▓▓▓▓▓▒▒▒░░░░░░░ │
│  1k        ▒▒▒▒▒▒▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▓▒▒▒▒▒░▒░▒░▒▒░░▒░░░░▒▒░▒░░░░░░▒▒▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒░░▒░░░▒░░░░░▒ │
│            ▓▓▓▓▓▓▓█▓▓▓▓▓▓▓▒▒▒▒░▒░▒▒░░░▒▒░░▒▒▒░▒░░▒▒▒▒▒▒▒▒▓▒▒▓▓▓▓▓█▓▓▓▓▓▓▓▓▒▒▒▒▒░░▒░▒▒░░▒▒▒░▒░▒░░ │
│            █▓▓█▓▓▓▓▒▒▒░▒▒▒▒▒▒▒▒▒▒░▒▒▒▒▒▒░▒▒▒░░░▒▒▓▓▓▓▓▓▓█▓▓▓██▓▓▓▓▓▓▓▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░▒▒▒░░▒▒▒▒▒▒ │
│ 250        ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ │
│            ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ │
│            ▓▒▓▓█▓▓▒▒█▒▓▒▒█▓▓▒▒█▒▒▒▒█▓▓▒▒█▓▒▒▓█▓▓▒▓█▓▓▒▓█▓▓▒▒█▒▓▒▒█▓▒▒▒█▒▒▒▒█▒▒▒▒█▓▒▒▒█▓▒▒▒█▓▒▒▒█ │
│  63        ▓▒▓▒█▓▒▓▒█▓▒▒▓█▓▓▒▒█▓▓▓▒█▓▒▓▓█▒▓▓▒█▓▒▒▒█▓▓▓▓█▓▓▒▓█▓▓▓▒█▒▒▒▓█▓▒▓▒█▓▓▓▓█▓▓▓▓█▓▓▒▒█▓▓▒▒█ │
│            ▓▓▓▒█▓▓▓▓█▓▓▓▒█▓▒▓▓█▓▓▓▒█▓▓▓▓█▓▓▓▓█▓▓▓▓█▓▓▓▓█▓▓▓▒█▓▒▒▓█▓▓▓▓█▓▓▓▒█▓▓▓▓█▓▒▓▓█▓▓▓▓█▓▓▒▒█ │
│       -9 s                -7 s                -5 s                -3 s                -1 s ahora │
```

```text
| Visualizador: Cascada  . . . * . .  4/6                                   14 bandas  últimos 9 s |
| 16k         . :  ... .:.:. . . .. :... . ...:. ......... .... ..:....:..:.: ...: .. :   .. .. :. |
|            ::.:....: ...-.....:::::....:.........-....:.::.:....-. :.:...:-....-.:..-:::::...... |
|            :..:..::-:::.-::::::.:::.::.:.::.-::.::.:::-.::.:..:.:.:.:-:.::-:.:.::.::-.:::::..:-: |
|  4k        ::::-:--::-:..:.:.::-.::.--:-::-:.::::::.::.:::.-:-.:.::.:-::.-::..::.-::.:.-:::-:.:: |
|            -:::::::-:::-:-:-===+*+*****+++*+==+==:::::::--::-::::::::::::::--==++****#***+**+++- |
|            ---:::-:--=++++***##*#****+++==-----:::-:-:::---:---:-:---=+++++#*#*##*#***++=----::- |
|  1k        ====++*******###***++*++===---=-=---=----==--------+++*#****#*#####*+++=--=---=-----= |
|            *#*#%%%%#**###*++==-=-==---==--==--=--=======***+*###%%#####***+=+-=--=-==---==---=-- |
|            %##%#*#*++=-==+-+=====-==+==+-===---+=****%##%###%%%%*****+===+==========-===--==+++= |
| 250        =+++++=+===+====+===+==+++==+++====+=+=+==++=++=====++===+===+=====+==+===+====++==+= |
|            +==+++=+++==++=+++==+++=+++++=+=+++==+++==++=++=+=+++=++++++=++=+++=+==+=++=+=+++=+++ |
|            #+**@**++@+*+*@**++@*+++@**+*@*++*@**+*@**+*@**++@+*++@**+=@++++@+++=@*+++@*+++@*+++@ |
|  63        *+*+@#+*+@*++*@**++@***+@*+**@****@*+++@****@**+*@***+@*++*@#+*+@#***@****@#*++@**++@ |
|            #**+@*#*#@###+@*+**@#**+@#***@****@#***@#***@#*#+@*+**@****@#***@****@#+**@#***@*#+*@ |
|       -9 s                -7 s                -5 s                -3 s                -1 s ahora |
```

- **Filas = bandas** (una por fila, logarítmicas, agudos arriba) y **columnas = tiempo**, la más nueva a la derecha. Avanza `WATERFALL_COLUMNS_PER_SECOND` (10): con 90 columnas se ven unos 9 s.
- **Intensidad por sombreado:** Unicode `░ ▒ ▓ █` (5 niveles contando el vacío), ASCII ` . : - = + * # % @` (10 niveles). La cantidad de niveles sale de la longitud del arreglo.
- **Ejes:** frecuencias a la izquierda (`16k 4k 1k 250 63`), segundos abajo (`-9 s … ahora`).
- **Historial:** un `VecDeque` de columnas en el servicio, con tantas columnas como el ancho. Si la terminal se achica, se conservan las más nuevas.
- **Pausa:** el tiempo sigue corriendo y las columnas nuevas salen vacías. El hueco muestra cuándo se pausó.
- **Color:** de gris oscuro (bajo) a cian, amarillo y rojo (alto). El sombreado ya transmite la intensidad.

### 5.7 Vectorscopio

```text
│ Visualizador: Vectorscopio  ○ ○ ○ ○ ● ○  5/6                                  M/S  1800 muestras │
│                                                M                                                 │
│                                 L ⠁⢀                        ⡀⠈ R                                 │
│                                      ⠄   ⣦⠚⢑⣿⣻⡹⣺⠿⣛⢛⣿⠤     ⠠                                      │
│                                       ⠐ ⠈⢪⣾⣿⣿⡹⡓⡟⡿⣺⣯⣯⡟⣳⡶⡒ ⠂                                       │
│                                         ⠁⢰⣿⢳⡺⠍⢧⣵⢾⢭⣿⣿⢎⡹⡾⡉                                         │
│                                          ⢮⠘⡭⣽⣮⠌⣆⡻⡫⣾⡏⡷⣷⠱                                          │
│                                         ⡵⠁ ⠌⡸⣝⢿⣴⣄⠟⡀⢻⡿⢵⢀                                          │
│                                          ⠙⣄⢾⣾⣧⣗⣨⣾⡿⣧⣸⡺⠱⢇                                          │
│                                         ⢠⣬⠷⠯⣟⢿⣹⣱⢲⢿⣻⣹⣣⣼⠂⡃                                         │
│                                         ⢸⣿⡧⣦⣟⣳⣿⣯⢾⠶⡞⣾⣿⣷⡦⠌                                         │
│                                          ⡾⣿⣿⣽⣷⣾⣲⣽⡽⣿⣞⡧⢵⠟⠁                                         │
│                                         ⠠⢀⠘⠟⣟⠟⠛⠛⠙⢍⡫⢙⢼⣇⠈⠄                                         │
│                                        ⠂ ⡂⡜ ⢫⠰⡠⠠⡎⠘⡡⣰⡗⡉⢄ ⠐                                        │
│                                     ⡀⠈  ⠐⢰⡅   ⡿⣚⢠⣤⠖⠟⡇⢅⠼   ⠁⢀                                     │
│                                   ⠠      ⠈⠛ ⠲⠗⠖⠚⠋⠚⠺⠦⠾⠟⠂      ⠄                                   │
│               -1 ────────────────────┼───────────●──────── +1   correlación +0.62                │
```

```text
| Visualizador: Vectorscopio  . . . . * .  5/6                                  M/S  1800 muestras |
|                                                M                                                 |
|                                 L \                          / R                                 |
|                                     \    +.++++++++#       /                                     |
|                                       \ .++###+####+++#. /                                       |
|                                         \++##+++++##+++.                                         |
|                                          +++#+++++#++++.                                         |
|                                         +. .+##+++++#+.                                          |
|                                          .++#++++++++++                                          |
|                                         .++++#+++##+++..                                         |
|                                         .###+##+#++###+.                                         |
|                                          +##########+++.                                         |
|                                          ..+#++++++.++\                                          |
|                                        / ...+.+....++.. \                                        |
|                                      /   .+ ..++++++.++   \                                      |
|                                    /      . ++.. ..+++      \                                    |
|               -1 --------------------+-----------o-------- +1   correlación +0.62                |
```

- **Ejes M/S:** vertical `M = (L + R) / 2`, horizontal `S = (R − L) / 2`. Un tema mono es una línea vertical; uno muy abierto, una nube redonda; uno con un canal invertido, una línea horizontal.
- **Guías:** diagonales punteadas hacia `L` (arriba a la izquierda) y `R` (arriba a la derecha), y `M` arriba. Los puntos de la señal tapan las guías.
- **Área cuadrada:** en braille cada punto es casi cuadrado (una celda mide 1 × 2 y tiene 2 × 4 puntos), así que el lado es `alto × 4` puntos = `alto × 2` celdas. En ASCII se usan `alto × 2` columnas por la misma razón, y cada celda muestra la **densidad** de puntos: `.` pocos, `+` medianos, `#` muchos (umbrales con nombre).
- **Puntos por cuadro:** los últimos `VECTORSCOPE_FRAMES` (2048) pares L/R.
- **Escala automática:** a diferencia de la Onda, acá importa la forma, no el volumen. La escala se ajusta al máximo reciente y vuelve despacio (`VECTORSCOPE_GAIN_DECAY`), para que un tema bajo no quede como un punto.
- **Correlación** debajo: `Σ L·R / √(Σ L² · Σ R²)`, de −1 a +1, con el marcador `●`/`o` sobre una línea con el centro `┼`/`+`. Con silencio vale 0. +1 es mono; cerca de 0, muy abierto; negativo, problemas de fase.

### 5.8 Medidores

```text
│ Visualizador: Medidores  ○ ○ ○ ○ ○ ●  6/6                                   RMS y pico por canal │
│                                                                                                  │
│  L  ██████████████████████████████████████████████████████      │       -9.4 dB  pico  -3.1      │
│     ██████████████████████████████████████████████████████      │                                │
│                                                                                                  │
│  R  ████████████████████████████████████████████████████      │        -11.2 dB  pico  -4.6      │
│     ████████████████████████████████████████████████████      │                                  │
│                                                                                                  │
│     ┴────────────────────┴──────────┴─────────┴───────┴──────┴──┴──┴  dBFS                       │
│     -60                 -40        -30       -20     -12    -6 -3  0                             │
│                                                                                                  │
│                                                                                                  │
│     -1 ────────────────────┼───────────●──────── +1   correlación +0.62                          │
│                                                                                                  │
│     barra = RMS (300 ms)   │ = pico retenido 1,5 s                                               │
```

```text
| Visualizador: Medidores  . . . . . *  6/6                                   RMS y pico por canal |
|                                                                                                  |
|  L  ######################################################      |       -9.4 dB  pico  -3.1      |
|     ######################################################      |                                |
|                                                                                                  |
|  R  ####################################################      |        -11.2 dB  pico  -4.6      |
|     ####################################################      |                                  |
|                                                                                                  |
|     +--------------------+----------+---------+-------+------+--+--+  dBFS                       |
|     -60                 -40        -30       -20     -12    -6 -3  0                             |
|                                                                                                  |
|                                                                                                  |
|     -1 --------------------+-----------o-------- +1   correlación +0.62                          |
|                                                                                                  |
|     barra = RMS (300 ms)   | = pico retenido 1,5 s                                               |
```

- **Una barra por canal** (`L` y `R`; `M` con FIFO mono), de 2 filas de alto. La barra es el **RMS** de los últimos `RMS_WINDOW` (300 ms). Una línea vertical `│`/`|` marca el **pico retenido** durante `METER_PEAK_HOLD` (1,5 s); después cae.
- **Escala en dBFS**, lineal en dB de −60 a 0, con marcas en −60, −40, −30, −20, −12, −6, −3 y 0.
- **Cálculo:** `rms_dB = 20·log10(rms)`, `pico_dB = 20·log10(máx |x|)`. Un seno a escala completa da pico 0 dBFS y RMS −3 dBFS (sin la corrección AES de +3 dB: KISS).
- **Resolución horizontal:** Unicode usa los octavos horizontales `▏▎▍▌▋▊▉█` para que la barra se mueva suave; ASCII usa celdas enteras `#`.
- **Color por zona:** verde hasta −12, amarillo hasta −3 y rojo arriba de −3. Lo que transmite el nivel es la longitud de la barra y el número.
- **A la derecha:** RMS y pico en números (`-9.4 dB  pico -3.1`).
- **Correlación** debajo, con el mismo widget que el Vectorscopio (DRY). Con FIFO mono no se muestra.

### 5.9 Arquitectura de las formas

```rust
// src/domain/visual.rs
pub enum VisualizerStyle { Bars, Mirror, Wave, Waterfall, Vectorscope, Meters }

impl VisualizerStyle {
    pub const ALL: [VisualizerStyle; 6] = [/* en el orden del ciclo */];
    pub fn needs_stereo(self) -> bool;               // Mirror y Vectorscope
    pub fn next(self, channels: u8) -> Self;         // salta las estéreo si channels == 1
}

pub enum VisualFrame {
    Bars(BandFrame),                                 // niveles + picos 0..1
    Mirror { left: BandFrame, right: BandFrame },
    Wave { columns: Vec<(f32, f32)>, synced: bool }, // (mínimo, máximo) por columna de puntos
    Waterfall { columns: Vec<Vec<f32>> },            // de la más vieja a la más nueva
    Vectorscope { points: Vec<(f32, f32)>, correlation: f32 }, // (S, M) ya escalados
    Meters { channels: Vec<ChannelLevel>, correlation: Option<f32> },
    Silent,
}
```

- **Servicio** (`src/service/visual/`): un módulo por forma (`bars.rs`, `mirror.rs`, `wave.rs`, `waterfall.rs`, `vectorscope.rs`, `meters.rs`) y dos compartidos: `bands.rs` (FFT + bandas: Barras, Espejo y Cascada) y `levels.rs` (RMS, pico y correlación: Medidores y Vectorscopio). El `Visualizer` elige con un `match` sobre `VisualizerStyle` (sin `dyn`, como el resto del proyecto).
- **TUI** (`src/tui/views/visual/`): un archivo por forma y tres dibujos compartidos: `braille.rs` (lienzo de puntos 2 × 4, usado por Onda y Vectorscopio), `correlation.rs` (Vectorscopio y Medidores) y el título con el indicador de páginas.
- **Agregar una forma** es agregar una variante, su análisis y su dibujo. No se toca nada más (abierto/cerrado).

### 5.10 Modo compacto

| Forma | Cambios en terminal chica |
|---|---|
| Barras | Barras de 1 columna, sin separación ni eje |
| Espejo estéreo | Barras de 1 columna; el eje central queda como línea sin etiquetas |
| Onda | Sin etiquetas de escala |
| Cascada | Sin ejes |
| Vectorscopio | Sin correlación ni letras |
| Medidores | Barras de 1 fila; sin escala ni correlación |

---

## 6. Glifos nuevos

Se agregan a `Glyphs` (`DESIGN.md` §2.1). Mismas reglas: una columna en Unicode, sin emoji, nada literal fuera de `glyphs.rs`.

| Rol (campo) | Unicode | ASCII | Uso |
|---|---|---|---|
| `spectrum_levels` | `" ▁▂▃▄▅▆▇█"` (9: vacío + 8 octavos) | `" .:#"` (4: vacío + 3) | Barras. La cantidad de niveles sale de la longitud del arreglo |
| `spectrum_peak` | `▔` | `-` | Marca de pico en Barras |
| `mirror_up` / `mirror_down` | `" ▄█"` / `" ▀█"` | `" .#"` / `" '#"` | Espejo estéreo |
| `plot` | `Plot::Braille` | `Plot::Cells { levels: "'-.", join: "\|" }` | Onda y Vectorscopio. El braille se genera por código (`U+2800 + bits`) dentro de `braille.rs`; es el único caso de glifos calculados |
| `scope_guides` | braille punteado | `\` y `/` | Guías L y R del Vectorscopio |
| `scope_density` | — | `" .+#"` | Densidad de puntos del Vectorscopio en ASCII |
| `shades` | `" ░▒▓█"` | `" .:-=+*#%@"` | Cascada |
| `meter_eighths` | `"▏▎▍▌▋▊▉█"` | `"#"` (sin parciales) | Barras de los Medidores |
| `meter_peak` | `│` | `\|` | Pico retenido de los Medidores |
| `scale_tick` | `┴` | `+` | Marcas de la escala en dBFS |
| `correlation_center` / `correlation_marker` | `┼` / `●` | `+` / `o` | Correlación |
| `page_active` / `page_inactive` | `●` / `○` | `*` / `.` | Indicador de forma en la fila de título |
| `updating` | reutiliza `spinner` | reutiliza `spinner` | Indicador de actualización |

Los indicadores en modo compacto (72 columnas):

```text
┌─ mpctui ──────────────────────────────────────── ✖ reintento en 5 s ─┐
┌─ mpctui ◐ ──────────────────────────────────────── ● localhost:6600 ─┐
+- mpctui -------------------------------------- (x) reintento en 5 s -+
+- mpctui / -------------------------------------- (*) localhost:6600 -+
```

Advertencia que se suma a `DESIGN.md` §2.3: los bloques `▁…█`, `▔`, `▄`, `▀`, `░▒▓` y `▏…▉` también tienen ancho ambiguo. En terminales con "ancho ambiguo = doble" el visualizador se rompe: es otro caso para `glyphs = "ascii"`. El braille no tiene ese problema, pero sí puede faltar en la fuente.

---

## 7. Tests

1. **Formato de audio:** parseo de `44100:16:2`, `96000:24:2`, `48000:f:2`, `dsd64:2`, `dsd128:2`; valores inválidos (`44100:16`, `abc:16:2`) → `Error::Protocol`. Formato de presentación de cada uno (tabla de §1.1). Recorte por ancho.
2. **`updating_db`:** `Subsystem::Update` mapeado desde `changed: update`; en `tests/fake_server.rs`, el guion `idle` → `changed: update` → `status` con `updating_db: 3` → `idle` → `changed: update` → `status` sin la clave, y se verifica empieza/termina.
3. **Cuenta regresiva:** con `tokio::time::pause()`, `ChangeWatcher` emite `Disconnected` con `retry_at` correcto en cada intento; redondeo hacia arriba (0,3 s → `1 s`); el medidor a 0 %, 50 % y 100 %; recorte por ancho en el orden de §3.1.
4. **Análisis de bandas (puro):**
   - Decodificar PCM con frames partidos entre dos lecturas.
   - Seno de 1 kHz generado en el test → la banda que contiene 1 kHz es la más alta.
   - Silencio → todas las bandas en 0.
   - Bordes de banda logarítmicos y bandas graves sin bin.
   - Suavizado y picos con tiempos simulados.
5. **Ciclo de formas:** `next()` vuelve a `Bars` después de `Meters`; con `channels == 1` salta `Mirror` y `Vectorscope`; `6` desde otra vista entra con la última forma; `6` en el visualizador avanza; `visualizer_style` inválido o estéreo con FIFO mono → error de configuración.
6. **Formas (puro):**
   - **Espejo:** canal L con seno y R en silencio → solo la mitad de arriba tiene barras.
   - **Onda:** mínimo/máximo por columna; el cruce ascendente se encuentra; un seno da la misma imagen en cuadros sucesivos (`synced`); ruido sin cruces → `libre`.
   - **Cascada:** avanza una columna cada 100 ms simulados; al achicar el ancho conserva las más nuevas.
   - **Vectorscopio:** canales iguales → todos los puntos con S = 0 y correlación +1; un canal invertido → M = 0 y −1; silencio → sin puntos y correlación 0.
   - **Medidores:** seno a escala completa → pico 0 dBFS y RMS −3,01; a la mitad → −6,02 y −9,03; pico retenido 1,5 s y luego cae.
7. **`FifoPcmSource`:** test de integración con un FIFO real creado en un directorio temporal (`mkfifo`), escribiendo un seno desde el test. El servicio se testea con un `PcmSource` en memoria.
8. **Render:** las seis formas con `TestBackend`, en ambos juegos y en modo compacto. En ASCII, el mismo chequeo de rangos prohibidos de `DESIGN.md` §9.3.5 (los bloques U+2580–U+259F y el braille U+2800–U+28FF caen dentro).
9. **Contra el MPD real (solo lectura):** un ejemplo nuevo, `cargo run --example spectrum_smoke`, lee `/tmp/mpd.fifo` durante 3 s e imprime en texto los niveles por banda, el RMS y pico por canal y la correlación. Sirve para ajustar `FLOOR_DB` y `CEIL_DB` mientras el usuario escucha música. No envía ningún comando a MPD.
10. **tmux:** `6` (también apretado varias veces seguidas) está en la lista de teclas de lectura permitidas contra el MPD real: no modifica nada.

---

## 8. Fases

Mismas reglas que `DESIGN.md` §10: fmt, clippy sin warnings, tests en verde, revisión contra `AGENTS.md`, `PLAN.md` y `README.md` actualizados, y frenar al terminar cada fase.

| Fase | Contenido | Listo cuando |
|---|---|---|
| **E1 · Formato de audio** | `AudioFormat`, `bitrate`, códec por extensión, título del panel, consulta de `status` cada 2 s mientras suena | El borde del panel muestra `FLAC  44.1 kHz  16 bit  estéreo  1411 kbps` y el bitrate se mueve con un MP3 VBR |
| **E2 · Actualización de la base** | `Subsystem::Update`, `updating_db`, indicador con spinner y tiempo, mensaje al terminar | Con `mpc update` (lo corre el usuario, no el agente) aparece y desaparece el indicador |
| **E3 · Cuenta regresiva** | `ServerEvent` ampliado, estado de conexión como enum, texto + medidor + intento | Con MPD detenido (lo hace el usuario) la cuenta baja de a un segundo y se recupera sola |
| **E4 · Visualizador: Barras** | `PcmFormat` + config, `PcmSource` + `FifoPcmSource`, `StereoWindow`, análisis de bandas, `Visualizer`, vista 6 con la forma Barras, ejemplo `spectrum_smoke` | Las barras siguen la música, caen a cero en pausa y la vista explica qué hacer si falta el FIFO |
| **E5 · Ciclo y formas** | `VisualizerStyle`, `VisualRequest`, ciclo con `6`, indicador de páginas, `visualizer_style`. Después, una forma por paso y en este orden (de la más simple a la más compleja): Medidores, Onda, Vectorscopio, Espejo, Cascada | Cada `6` pasa a la forma siguiente y vuelve a Barras; cada forma coincide con su mockup en ambos juegos |

Orden sugerido: E1 → E2 → E3 → E4 → E5, de menor a mayor riesgo. E4 es la única que agrega una dependencia (`rustfft`). En E5 conviene frenar después de cada forma para que el usuario la vea con su música.

---

## 9. Sigue fuera de alcance

- Tecla para "reintentar ahora" (§3.3).
- Rotación automática de formas cada N segundos (modo demo).
- Guardar la última forma al salir de mpctui.
- Persistencia tipo fósforo en Onda y Vectorscopio (rastro que se desvanece).
- Formas circulares o radiales: con celdas de 1 × 2 se ven deformes y no aportan sobre las seis.
- Paletas de color configurables para el visualizador.
- Detectar la salida fifo automáticamente con el comando `outputs`.
