# DESIGN_EXTRA.md — mpctui: extras visuales

Cuatro agregados que `DESIGN.md` dejó fuera de alcance:

1. **Formato de audio y bitrate** en el panel "ahora suena".
2. **Indicador de actualización de la base de datos** (`updating_db`).
3. **Cuenta regresiva del próximo reintento** de conexión.
4. **Visualizador de espectro** leyendo la salida FIFO de MPD.

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
- En modo compacto queda solo el spinner junto a `mpctui` (ver §5).

### 2.2 Datos de MPD

- `status` incluye `updating_db: <id de trabajo>` solo mientras hay una actualización en curso. Se parsea como `Option<u32>` (estricto: un valor no numérico es `Error::Protocol`).
- `idle` informa `changed: update` cuando una actualización **empieza o termina**. Hoy ese subsistema se ignora: se mapea a un nuevo `Subsystem::Update`. Al recibirlo, se vuelve a pedir `status` para saber cuál de las dos cosas pasó.
- `changed: database` sigue significando "la base cambió" y se procesa como hasta ahora.

Estado en la TUI: `AppState` guarda `database_update: Option<Instant>` (cuándo se vio por primera vez). El cálculo de "empezó / terminó" a partir de dos `status` sucesivos es lógica de negocio: va en un servicio, no en la vista.

### 2.3 Fuera de alcance

Disparar una actualización desde mpctui (`update`) **no** forma parte de esto: sería un atajo nuevo y un comando que modifica el servidor. Se agrega solo si el usuario lo pide.

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

## 4. Visualizador de espectro

### 4.1 Qué se ve

Una vista nueva, **`6` Espectro**, que entra en el ciclo de `Tab` / `Shift+Tab`. El panel "ahora suena" sigue abajo, igual que en las demás vistas.

Unicode (bloques de octavos `▁▂▃▄▅▆▇█`, 8 niveles por celda):

```text
┌─ mpctui ────────────────────────────────────────────────────────────────────── ● localhost:6600 ─┐
│  1 Cola  2 Biblioteca  3 Directorios  4 Búsqueda  5 Playlists [6 Espectro]  1229 temas  65:04:13 │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Espectro                                                32 bandas  40 Hz - 16 kHz  /tmp/mpd.fifo │
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
|  1 Cola  2 Biblioteca  3 Directorios  4 Búsqueda  5 Playlists [6 Espectro]  1229 temas  65:04:13 |
+--------------------------------------------------------------------------------------------------+
| Espectro                                                32 bandas  40 Hz - 16 kHz  /tmp/mpd.fifo |
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

Elementos:

- **Fila de título:** `Espectro` y, a la derecha, la cantidad de bandas, el rango de frecuencias y la ruta del FIFO.
- **Barras:** una por banda, de `SPECTRUM_BAR_WIDTH` (2) columnas con 1 de separación, alineadas abajo. La cantidad de bandas sale del ancho: `bandas = (ancho + 1) / 3`, entre `SPECTRUM_MIN_BANDS` (8) y `SPECTRUM_MAX_BANDS` (64). El sobrante se reparte a los costados para centrar.
- **Resolución vertical:** la altura de cada barra se calcula en sub-celdas (`alto × 8` en Unicode, `alto × 3` en ASCII); la celda de arriba de la barra usa el glifo parcial que corresponda.
- **Picos:** un `▔`/`-` marca el valor máximo reciente de cada banda. Se queda `PEAK_HOLD` (0,5 s) y después cae a `PEAK_FALL_PER_SECOND`. Solo se dibuja si esa celda está vacía.
- **Eje de frecuencias:** etiquetas `63 125 250 500 1k 2k 4k 8k 16k` debajo de la banda más cercana a cada frecuencia. Si dos etiquetas se pisan, se omite la de la derecha.
- **Color** (16 ANSI, en `theme.rs`): por altura, el tercio de abajo verde, el del medio amarillo y el de arriba rojo. El color solo refuerza: la altura ya transmite el nivel (principio 2 de `DESIGN.md`).
- **Sin señal** (pausado, detenido o sin datos en `SILENCE_TIMEOUT` = 250 ms): las barras caen hasta cero con la misma velocidad de caída y, ya en cero, se muestra `sin señal` centrado.

Si el FIFO no existe o no se puede abrir, la vista lo explica en lugar de fallar:

```text
│ Espectro                                                                               sin señal │
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

Con terminal chica (modo compacto de `DESIGN.md` §7) el visualizador sigue disponible: usa barras de 1 columna sin separación y oculta el eje y la fila de título.

### 4.2 Configuración

```toml
# ~/.config/mpctui/config.toml
visualizer_fifo   = "/tmp/mpd.fifo"   # defecto
visualizer_format = "44100:16:2"      # defecto; debe coincidir con el format de la salida fifo de MPD
```

- Fail fast en el borde: `visualizer_format` se valida al cargar la config con un newtype en `src/domain/inputs.rs` (`PcmFormat`). Solo se aceptan **16 bits** y **1 o 2 canales**, con frecuencia entre 8000 y 192000. Cualquier otra cosa no arranca y explica el motivo.
- `visualizer_fifo` vacío es un error de configuración (no un valor que "desactiva" la vista: YAGNI).
- Un FIFO inexistente **no** es error de configuración: se resuelve en tiempo de ejecución con la pantalla de ayuda de arriba, porque MPD puede crearlo después.

### 4.3 Lectura del FIFO

- MPD escribe PCM crudo, entrelazado, con signo, en el **orden de bytes nativo** (`i16::from_ne_bytes`), en el formato de la salida fifo de `mpd.conf`.
- Se abre en modo no bloqueante con `tokio::net::unix::pipe::OpenOptions::new().open_receiver(ruta)`: abrir para lectura no bloquea aunque MPD todavía no esté escribiendo.
- Las lecturas pueden cortar un frame por la mitad (4 bytes en 16 bit estéreo): los bytes sobrantes se guardan para la lectura siguiente.
- Se guardan solo las últimas `FFT_SIZE` muestras en un búfer circular. Al abrir o al volver de una pausa, lo que haya acumulado en el pipe es viejo: se lee todo y se descarta todo menos lo último.
- **Fin de archivo (0 bytes):** MPD cerró su lado (se detuvo o se reinició). Se reabre usando el `Backoff` que ya existe en `src/service/` (DRY), sin mostrar error: la vista pasa a "sin señal".
- **Solo lectura:** leer el FIFO no cambia nada en MPD, así que **sí** se puede probar contra el servidor real (ver §6).

### 4.4 Análisis

Pasos por cuadro, todos en funciones puras:

1. **Mono:** promedio de los canales, normalizado a `f32` en [-1, 1].
2. **Ventana de Hann** sobre las últimas `FFT_SIZE` (2048) muestras: unos 46 ms a 44,1 kHz, con resolución de unos 21,5 Hz por bin.
3. **FFT real** con el crate `rustfft` (dependencia nueva, justificada: implementar una FFT a mano va contra KISS).
4. **Magnitud por bin:** `|X| × 2 / suma_de_la_ventana`.
5. **Bandas logarítmicas** entre `SPECTRUM_MIN_HZ` (40) y `SPECTRUM_MAX_HZ` (16000): `borde_i = min × (max/min)^(i/bandas)`. Cada banda toma el **máximo** de los bins que caen en su rango; si una banda grave no contiene ningún bin, toma el bin más cercano a su centro.
6. **Decibeles y normalización:** `dB = 20·log10(magnitud + ε)`, `nivel = clamp((dB − FLOOR_DB) / (CEIL_DB − FLOOR_DB), 0, 1)` con `FLOOR_DB = -60` y `CEIL_DB = 0`. Son constantes para ajustar con música real usando el ejemplo de §6.
7. **Suavizado:** si el nivel nuevo es mayor, se toma enseguida; si es menor, baja como mucho `FALL_PER_SECOND` (1,5 alturas completas por segundo) × tiempo transcurrido. Los picos, igual, con su propia caída.

### 4.5 Arquitectura

Respeta las capas de `AGENTS.md`: el FIFO es una **fuente de datos**, así que va en `repository`. El análisis es lógica, así que va en `service`. La vista solo dibuja.

| Capa | Archivo | Contenido |
|---|---|---|
| domain | `src/domain/inputs.rs` | `PcmFormat` validado (frecuencia, 16 bit, canales) |
| domain | `src/domain/spectrum.rs` | `SpectrumFrame { levels: Vec<f32>, peaks: Vec<f32> }` (valores 0..1) |
| repository | `src/repository/traits.rs` | Trait angosto `PcmSource { fn read(&mut self, destination: &mut [u8]) -> impl Future<Output = Result<usize>> + Send; }` |
| repository | `src/repository/fifo.rs` | `FifoPcmSource`, abre y lee el pipe. **No** va en `mpd/`: no es el protocolo de MPD |
| service | `src/service/spectrum.rs` | Funciones puras: decodificar PCM, ventana, bandas, dB, suavizado |
| service | `src/service/visualizer.rs` | `Visualizer<S: PcmSource>`: tarea que lee, analiza a `VISUALIZER_FPS` (20) y publica el último `SpectrumFrame` |
| tui | `src/tui/views/spectrum.rs` | Dibuja las barras, picos y eje desde `AppState` |
| tui | `src/tui/glyphs.rs` | Campos nuevos (§5) |
| factory | `src/factory.rs` | Arma `FifoPcmSource` + `Visualizer` con la config y lo inyecta |

Decisiones de concurrencia:

- El `Visualizer` publica en un **`tokio::sync::watch`**: solo importa el último cuadro, no hay cola que se acumule si la TUI va más lenta.
- La tarea **solo corre mientras la vista 6 está activa.** Al entrar se lanza; al salir se cancela (`JoinHandle::abort` o un `CancellationToken`). Fuera de esa vista no se lee el FIFO ni se gasta CPU.
- El loop `select!` de `App` agrega una rama para el `watch` y redibuja al recibir un cuadro. Las demás vistas no cambian su frecuencia de dibujado.
- `6` y la vista nueva se agregan **solo en la tabla `BINDINGS`**: la ayuda y la línea inferior se actualizan solas.

---

## 5. Glifos nuevos

Se agregan a `Glyphs` (`DESIGN.md` §2.1). Mismas reglas: una columna en Unicode, sin emoji, nada literal fuera de `glyphs.rs`.

| Rol (campo) | Unicode | ASCII | Uso |
|---|---|---|---|
| `spectrum_levels` | `" ▁▂▃▄▅▆▇█"` (9: vacío + 8 octavos) | `" .:#"` (4: vacío + 3) | Sub-celdas de las barras del espectro. La cantidad de niveles sale de la longitud del arreglo, no de una constante aparte |
| `spectrum_peak` | `▔` | `-` | Marca de pico |
| `updating` | reutiliza `spinner` | reutiliza `spinner` | Indicador de actualización |

Los indicadores en modo compacto (72 columnas):

```text
┌─ mpctui ──────────────────────────────────────── ✖ reintento en 5 s ─┐
┌─ mpctui ◐ ──────────────────────────────────────── ● localhost:6600 ─┐
+- mpctui -------------------------------------- (x) reintento en 5 s -+
+- mpctui / -------------------------------------- (*) localhost:6600 -+
```

Advertencia que se suma a `DESIGN.md` §2.3: los bloques `▁…█` y `▔` también tienen ancho ambiguo. En terminales con "ancho ambiguo = doble" el espectro se rompe: es otro caso para `glyphs = "ascii"`.

---

## 6. Tests

1. **Formato de audio:** parseo de `44100:16:2`, `96000:24:2`, `48000:f:2`, `dsd64:2`, `dsd128:2`; valores inválidos (`44100:16`, `abc:16:2`) → `Error::Protocol`. Formato de presentación de cada uno (tabla de §1.1). Recorte por ancho.
2. **`updating_db`:** `Subsystem::Update` mapeado desde `changed: update`; en `tests/fake_server.rs`, el guion `idle` → `changed: update` → `status` con `updating_db: 3` → `idle` → `changed: update` → `status` sin la clave, y se verifica empieza/termina.
3. **Cuenta regresiva:** con `tokio::time::pause()`, `ChangeWatcher` emite `Disconnected` con `retry_at` correcto en cada intento; redondeo hacia arriba (0,3 s → `1 s`); el medidor a 0 %, 50 % y 100 %; recorte por ancho en el orden de §3.1.
4. **Espectro (puro):**
   - Decodificar PCM con frames partidos entre dos lecturas.
   - Seno de 1 kHz generado en el test → la banda que contiene 1 kHz es la más alta.
   - Silencio → todas las bandas en 0.
   - Bordes de banda logarítmicos y bandas graves sin bin.
   - Suavizado y picos con tiempos simulados.
5. **`FifoPcmSource`:** test de integración con un FIFO real creado en un directorio temporal (`mkfifo`), escribiendo un seno desde el test. El servicio se testea con un `PcmSource` en memoria.
6. **Render:** la vista 6 con `TestBackend` en ambos juegos y en modo compacto. En ASCII, el mismo chequeo de rangos prohibidos de `DESIGN.md` §9.3.5 (U+2581–U+2594 caen dentro).
7. **Contra el MPD real (solo lectura):** un ejemplo nuevo, `cargo run --example spectrum_smoke`, lee `/tmp/mpd.fifo` durante 3 s e imprime los niveles por banda en texto. Sirve para ajustar `FLOOR_DB` y `CEIL_DB` mientras el usuario escucha música. No envía ningún comando a MPD.
8. **tmux:** `6` está en la lista de teclas de lectura permitidas contra el MPD real (no modifica nada).

---

## 7. Fases

Mismas reglas que `DESIGN.md` §10: fmt, clippy sin warnings, tests en verde, revisión contra `AGENTS.md`, `PLAN.md` y `README.md` actualizados, y frenar al terminar cada fase.

| Fase | Contenido | Listo cuando |
|---|---|---|
| **E1 · Formato de audio** | `AudioFormat`, `bitrate`, códec por extensión, título del panel, consulta de `status` cada 2 s mientras suena | El borde del panel muestra `FLAC  44.1 kHz  16 bit  estéreo  1411 kbps` y el bitrate se mueve con un MP3 VBR |
| **E2 · Actualización de la base** | `Subsystem::Update`, `updating_db`, indicador con spinner y tiempo, mensaje al terminar | Con `mpc update` (lo corre el usuario, no el agente) aparece y desaparece el indicador |
| **E3 · Cuenta regresiva** | `ServerEvent` ampliado, estado de conexión como enum, texto + medidor + intento | Con MPD detenido (lo hace el usuario) la cuenta baja de a un segundo y se recupera sola |
| **E4 · Espectro** | `PcmFormat` + config, `PcmSource` + `FifoPcmSource`, análisis, `Visualizer`, vista 6, ejemplo `spectrum_smoke` | Las barras siguen la música, caen a cero en pausa y la vista explica qué hacer si falta el FIFO |

Orden sugerido: E1 → E2 → E3 → E4, de menor a mayor riesgo. E4 es la única que agrega una dependencia (`rustfft`).

---

## 8. Sigue fuera de alcance

- Disparar `update` desde mpctui (§2.3).
- Tecla para "reintentar ahora" (§3.3).
- Espectro estéreo (dos mitades espejadas), forma de onda u osciloscopio.
- Paletas de color configurables para el espectro.
- Detectar la salida fifo automáticamente con el comando `outputs`.
