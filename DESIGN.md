# DESIGN.md — mpctui: diseño visual de la TUI

Especificación visual de mpctui: cómo se ve cada vista, qué ayudas gráficas muestra y cómo se dibujan. Está escrita para que un agente la implemente fase por fase.

> **Antes de empezar:** leer `AGENTS.md` completo. Este documento **no cambia** la arquitectura, los atajos ni el comportamiento: solo agrega presentación. Todo lo que diga acá se implementa respetando SOLID, DRY, KISS, YAGNI, la separación `domain ← repository ← service ← tui`, fail fast y "sin números ni strings mágicos". Si algo de este diseño choca con `AGENTS.md`, gana `AGENTS.md` y se avisa al usuario.

Los mockups están dibujados a **100 columnas**. Los anchos son ilustrativos: lo que manda son las reglas de cada sección.

---

## 1. Principios visuales

1. **Unicode por defecto, ASCII como fallback.** Hay dos juegos de glifos con exactamente el mismo layout. Unicode (`▶ ⏸ ■ ♪ ▸ ≡`, bordes `┌─┐`) se ve mejor; ASCII (`|> || [] ~ > =`, bordes `+-|`) funciona en cualquier terminal, consola de Linux (`TERM=linux`), sesiones serie, `screen` viejo o fuentes sin esos símbolos. El usuario puede forzar cualquiera de los dos.
2. **La información nunca depende solo del color.** Todo estado se ve con un glifo o un modificador (negrita, video inverso) además del color. Con un terminal monocromo la interfaz tiene que seguir siendo usable.
3. **Todo glifo ocupa exactamente una columna** (o un ancho fijo y conocido en ASCII, como `|>`). Los anchos se miden con `unicode-width` (`Line::width()` / `Span::width()` de ratatui o `UnicodeWidthStr::width`), **nunca con `.len()`**: `━` ocupa 3 bytes y 1 columna.
4. **Sin emoji.** Nada con presentación de emoji (color, doble ancho). Ver advertencias en la sección 2.3.
5. **Ningún glifo literal fuera de `src/tui/glyphs.rs`.** Las vistas y widgets piden el glifo por su rol (`glyphs.play`, `glyphs.bar_filled`), nunca escriben `"▶"` ni `"|>"`.
6. **El texto de UI sigue en español con tildes.** Las letras acentuadas son texto, no gráficos: no se reemplazan en modo ASCII.
7. **Solo los 16 colores ANSI básicos** (más los modificadores). Nada de RGB ni de paletas de 256 colores: máxima compatibilidad.
8. **Lo que se ve en pantalla se puede leer como texto.** La selección se marca con video inverso **y** con el glifo de cursor, para que `tmux capture-pane -p` (que no captura atributos) permita verificar la UI desde los tests manuales de `AGENTS.md`.

---

## 2. Juegos de glifos

### 2.1 Tabla de roles

Cada fila es un campo de la struct `Glyphs` (sección 9.1). La columna "Unicode" es el juego principal; "ASCII" es el fallback.

| Rol (campo) | Unicode | ASCII | Dónde se usa |
|---|---|---|---|
| `play` | `▶` | `\|>` | Estado reproduciendo: título del panel, fila del tema en curso |
| `pause` | `⏸` | `\|\|` | Estado en pausa |
| `stop` | `■` | `[]` | Estado detenido |
| `now_playing` | `♪` | `~` | Delante del título que suena; insignia de tema en Directorios |
| `cursor` | `▸` | `>` | Fila seleccionada (junto con video inverso) |
| `directory` | `▪` | `+` | Insignia de directorio |
| `playlist` | `≡` | `=` | Insignia de playlist (Directorios y vista Playlists) |
| `bar_filled` | `━` | `=` | Parte ya escuchada de una barra |
| `bar_head` | `●` | `>` | Posición actual en una barra |
| `bar_empty` | `─` | `-` | Parte pendiente de una barra |
| `bar_separator` | `┃` | `\|` | Límite entre segmentos (temas o álbumes) |
| `bar_open` / `bar_close` | *(vacío)* | `[` / `]` | Extremos de una barra |
| `bar_unknown` | `╌` | `~` | Barra sin duración conocida (streams) |
| `bar_more` | `…` | `..` | Hay más segmentos fuera de la ventana visible |
| `meter_filled` / `meter_empty` | `■` / `□` | `#` / `-` | Medidores (volumen, duración relativa) |
| `flag_on` / `flag_off` / `flag_once` | `●` / `○` / `◐` | `x` / ` ` / `1` | Modos aleatorio, repetir, single, consumir |
| `group_rule` | `─` | `=` | Relleno de las filas separadoras de álbum |
| `breadcrumb` | ` › ` | ` > ` | Migas de pan |
| `ellipsis` | `…` | `...` | Texto truncado |
| `scroll_up` / `scroll_down` | `▲` / `▼` | `^` / `v` | Extremos de la barra de desplazamiento |
| `scroll_track` / `scroll_thumb` | `│` / `█` | `:` / `#` | Riel y cursor de la barra de desplazamiento |
| `connected` / `disconnected` | `●` / `✖` | `(*)` / `(x)` | Estado de la conexión en el encabezado |
| `spinner` | `◐ ◓ ◑ ◒` | `\| / - \` | Reintentando la conexión (avanza un cuadro por tick) |
| `message_info` / `message_error` | `·` / `✖` | `(i)` / `(x)` | Prefijo de mensajes en la línea inferior |
| `border` | `border::PLAIN` (`┌─┐│└┘`) | `+ - \|` | Bordes de bloques (`ratatui::symbols::border::Set`) |
| `junction` | `├ ┤ ┬ ┴ ┼` | `+` | Uniones entre paneles |

Reglas de la tabla:

- Los campos que en ASCII ocupan más de una columna (`play`, `pause`, `stop`, `bar_more`, `connected`, etc.) definen el ancho de su columna en el layout. Los anchos de columna se calculan a partir del glifo, no se fijan a mano: así ambos juegos se alinean solos.
- En Unicode, `play`/`pause`/`stop` ocupan 1 columna y se rellenan con un espacio cuando están en una columna de 2 (fila de la cola).

### 2.2 Cómo se elige el juego

Nueva opción de configuración, con la misma precedencia que el resto (defecto < archivo < entorno):

```toml
# ~/.config/mpctui/config.toml
glyphs = "auto"      # "auto" (defecto) | "unicode" | "ascii"
```

| Variable | Ejemplo | Efecto |
|---|---|---|
| `MPCTUI_GLYPHS` | `ascii` | Fuerza el juego de glifos. Útil para probar con tmux sin tocar el archivo |

Resolución de `auto` (función pura, testeable, en `src/config.rs`):

1. Tomar el locale efectivo: el primer valor **no vacío** de `LC_ALL`, `LC_CTYPE`, `LANG`.
2. Si no hay locale, o no contiene `UTF-8` / `utf8` (sin distinguir mayúsculas) → **ASCII**.
3. Si `TERM` es `linux` (consola virtual: su fuente no tiene la mayoría de estos símbolos) o `dumb` → **ASCII**.
4. En cualquier otro caso → **Unicode**.

Fail fast: un valor distinto de `auto`, `unicode` o `ascii` (en el TOML o en `MPCTUI_GLYPHS`) es un error de configuración y mpctui no arranca, igual que hoy con un puerto inválido.

### 2.3 Advertencias sobre Unicode (por qué existe el fallback)

- **Ancho ambiguo.** `▶ ■ ● ○ ◐ ▸ ♪ ≡ □` tienen ancho "ambiguo" en Unicode (East Asian Width = A). Las terminales configuradas con "ancho ambiguo = doble" (común con locales CJK) los dibujan en 2 columnas y rompen la alineación. No se puede detectar de forma confiable: para eso está `glyphs = "ascii"`. Documentarlo en el README.
- **`⏸` (U+23F8) puede verse como emoji** en algunas terminales y fuentes (a color y en 2 columnas). Se usa igual porque el diseño lo pide, pero queda aislado en `glyphs.pause`: si da problemas, se cambia en un solo lugar por `‖` (U+2016). No agregar el selector de variación U+FE0E: no todas las terminales lo respetan y complica el cálculo de anchos.
- **Fuentes sin el glifo.** Si la fuente no tiene un símbolo, la terminal muestra un cuadrado o `?`. De nuevo: fallback ASCII.

---

## 3. Layout general

La pantalla se divide en cuatro zonas, siempre en este orden vertical:

| Zona | Alto | Contenido |
|---|---|---|
| Encabezado | 3 filas | Nombre, estado de la conexión, pestañas de vistas y resumen de la cola |
| Vista | el resto | La vista activa (Cola, Biblioteca, Directorios, Búsqueda, Playlists) |
| Panel "ahora suena" | 7 filas (completo) / 4 (compacto) | Título, barras de progreso, volumen, modos |
| Línea inferior | 1 fila | Atajos de la vista activa o el último mensaje |

### 3.1 Pantalla completa — Unicode (vista Cola, 100 × 30)

```text
┌─ mpctui ────────────────────────────────────────────────────────────────────── ● localhost:6600 ─┐
│ [1 Cola]  2 Biblioteca   3 Directorios   4 Búsqueda   5 Playlists     cola: 1229 temas  65:04:13 │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│      pos  Título                                       Artista                            dur    │
│── Pink Floyd - Meddle (1971) ───────────────────────────────────────────────── 6 temas  46:46 ──▲│
│      297  Fearless                                     Pink Floyd                        6:08   ││
│      298  San Tropez                                   Pink Floyd                        3:44   ││
│      299  Seamus                                       Pink Floyd                        2:16   ││
│      300  Echoes                                       Pink Floyd                       23:31   ││
│── Pink Floyd - The Dark Side of the Moon (1973) ───────────────────────────── 10 temas  42:51 ──█│
│      301  Speak to Me                                  Pink Floyd                        1:05   █│
│      302  Breathe (In the Air)                         Pink Floyd                        2:49   ││
│      303  On the Run                                   Pink Floyd                        3:45   ││
│  ▶   304  Time                                         Pink Floyd                        6:53   ││
│      305  The Great Gig in the Sky                     Pink Floyd                        4:44   ││
│▸     306  Money                                        Pink Floyd                        6:22   ││
│      307  Us and Them                                  Pink Floyd                        7:49   ││
│      308  Any Colour You Like                          Pink Floyd                        3:26   ││
│      309  Brain Damage                                 Pink Floyd                        3:46   ││
│      310  Eclipse                                      Pink Floyd                        2:12   ││
│── Radiohead - OK Computer (1997) ──────────────────────────────────────────── 12 temas  53:21 ──││
│      311  Airbag                                       Radiohead                         4:44   ▼│
├─ ▶ reproduciendo ────────────────────────────────────────────────────────────────────────────────┤
│ ♪ Time                                             Pink Floyd - The Dark Side of the Moon (1973) │
│ tema   ━━━━━━━━━━━━━━━━━━●─────────────────────────────────────                      2:13 / 6:53 │
│ álbum  ━┃━━━┃━━━━┃━━●─────┃─────┃───────┃─────────┃────┃────┃──         tema 4/10   9:52 / 42:51 │
│ cola   …┃━━━━━┃━━━━━━━┃━━━━━━━┃━●────┃────────┃──────┃───────┃…  álbum 12/87  8:11:05 / 65:04:13 │
│ vol ■■■■■■■□□□  70%     z ● aleatorio   r ○ repetir   y ◐ single   R ○ consumir     pos 304/1229 │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
 Enter reproducir   d borrar   o ir al actual   / buscar   ? ayuda
```

### 3.2 La misma pantalla — ASCII

```text
+- mpctui -------------------------------------------------------------------- (*) localhost:6600 -+
| [1 Cola]  2 Biblioteca   3 Directorios   4 Búsqueda   5 Playlists     cola: 1229 temas  65:04:13 |
+--------------------------------------------------------------------------------------------------+
|      pos  Título                                       Artista                            dur    |
|== Pink Floyd - Meddle (1971) ================================================= 6 temas  46:46 ==^|
|      297  Fearless                                     Pink Floyd                        6:08   :|
|      298  San Tropez                                   Pink Floyd                        3:44   :|
|      299  Seamus                                       Pink Floyd                        2:16   :|
|      300  Echoes                                       Pink Floyd                       23:31   :|
|== Pink Floyd - The Dark Side of the Moon (1973) ============================= 10 temas  42:51 ==#|
|      301  Speak to Me                                  Pink Floyd                        1:05   #|
|      302  Breathe (In the Air)                         Pink Floyd                        2:49   :|
|      303  On the Run                                   Pink Floyd                        3:45   :|
|  |>  304  Time                                         Pink Floyd                        6:53   :|
|      305  The Great Gig in the Sky                     Pink Floyd                        4:44   :|
|>     306  Money                                        Pink Floyd                        6:22   :|
|      307  Us and Them                                  Pink Floyd                        7:49   :|
|      308  Any Colour You Like                          Pink Floyd                        3:26   :|
|      309  Brain Damage                                 Pink Floyd                        3:46   :|
|      310  Eclipse                                      Pink Floyd                        2:12   :|
|== Radiohead - OK Computer (1997) ============================================ 12 temas  53:21 ==:|
|      311  Airbag                                       Radiohead                         4:44   v|
+- |> reproduciendo -------------------------------------------------------------------------------+
| ~ Time                                             Pink Floyd - The Dark Side of the Moon (1973) |
| tema   [=================>------------------------------------]                      2:13 / 6:53 |
| álbum  [=|===|====|==>----|-----|-------|--------|----|----|--]         tema 4/10   9:52 / 42:51 |
| cola   [..|=====|======|======|=>----|-------|------|------|..]  álbum 12/87  8:11:05 / 65:04:13 |
| vol [#######---]  70%     z[x] aleatorio  r[ ] repetir  y[1] single  R[ ] consumir  pos 304/1229 |
+--------------------------------------------------------------------------------------------------+
 Enter reproducir   d borrar   o ir al actual   / buscar   ? ayuda
```

### 3.3 Encabezado

- Título del bloque: `mpctui`. A la derecha, el estado de la conexión: `● localhost:6600` (o el socket unix) cuando hay conexión.
- Sin conexión reemplaza a la **✖ sin conexión** actual del README:

```text
┌─ mpctui ──────────────────────────────────────────────────────── ✖ sin conexión, reintentando ◐ ─┐
+- mpctui ------------------------------------------------------ (x) sin conexión, reintentando / -+
```

  El spinner avanza un cuadro por cada tick de 1 s que ya existe en el loop. No se muestra cuenta regresiva del backoff (YAGNI: requeriría exponer el tiempo del próximo reintento desde `ChangeWatcher`).
- Pestañas: la activa entre corchetes `[1 Cola]`, en negrita y video inverso; las demás con un espacio a cada lado para que nada se mueva al cambiar de vista. Siempre se muestra el número, que es el atajo.
- A la derecha de las pestañas: `cola: <cantidad> temas  <duración total>`. Si alguna duración es desconocida, `cola: 1229 temas` sin duración.

### 3.4 Panel "ahora suena"

Fila por fila:

1. **Título del bloque:** glifo de estado + palabra: `▶ reproduciendo`, `⏸ en pausa`, `■ detenido`.
2. **Tema:** `♪ <título>` a la izquierda; `<artista> - <álbum> (<año>)` alineado a la derecha. Si no entra, se trunca primero el álbum, después el artista y por último el título, siempre con `ellipsis`. Si falta el año, sin paréntesis.
3. **Barra del tema** (`tema`): progreso del tema actual. Derecha: `2:13 / 6:53`.
4. **Barra del álbum** (`álbum`): el álbum en curso, **un segmento por tema**, separados por `┃`/`|`. Derecha: `tema 4/10   9:52 / 42:51`.
5. **Barra de la cola** (`cola`): la cola completa, **un segmento por álbum**, separados por `┃`/`|`, con ventana alrededor del álbum actual. Derecha: `álbum 12/87  8:11:05 / 65:04:13`.
6. **Volumen y modos:** medidor de volumen + porcentaje; los cuatro modos con su tecla delante (`z ● aleatorio`, en ASCII `z[x] aleatorio`); a la derecha `pos 304/1229`.

Las tres barras tienen **el mismo ancho y empiezan en la misma columna**, así se leen como una sola columna de progreso a distintas escalas. Las etiquetas (`tema`, `álbum`, `cola`) ocupan un ancho fijo y el texto de la derecha se alinea a la derecha.

En pausa solo cambia el título del bloque y la barra deja de avanzar:

```text
├─ ⏸ en pausa ─────────────────────────────────────────────────────────────────────────────────────┤
│ ♪ Time                                             Pink Floyd - The Dark Side of the Moon (1973) │
```

Detenido: las tres barras vacías, tiempos `--:--`, y la fila 2 muestra el tema que sonaría al dar play (si MPD informa uno) o `sin reproducción`.

### 3.5 Línea inferior

- Por defecto: los atajos más útiles de la vista activa (`Enter reproducir   d borrar   o ir al actual   / buscar   ? ayuda`).
- **DRY:** las teclas salen de la tabla `BINDINGS`. Cada vista declara solo la lista de `Action` que quiere sugerir; el texto de la tecla se busca en `BINDINGS`, nunca se escribe a mano.
- Un mensaje (error o confirmación de una acción) reemplaza a los atajos durante `MESSAGE_DURATION` (constante, 5 s) con su prefijo: `✖ No se pudo conectar: ...` / `(x) No se pudo conectar: ...`. Si ya existe una línea de mensajes, se reutiliza.

---

## 4. Ayudas gráficas

### 4.1 Barra segmentada (tema, álbum y cola)

Las tres barras del panel son **la misma función** con distintos segmentos (DRY):

| Barra | Segmentos | Peso de cada segmento | Ventana |
|---|---|---|---|
| Tema | 1 (el tema) | duración del tema | no |
| Álbum | uno por tema del álbum en curso | duración de cada tema | sí, si no entran |
| Cola | uno por álbum de la cola | duración de cada álbum | sí, radio 3 |

**Anatomía** (ASCII para que se vean bien los límites):

```text
[==|==|===|=>---|---|----|------|--|---|-]
 ^  ^      ^^
 |  |      |+-- vacío: lo que falta del segmento actual y los segmentos siguientes
 |  |      +--- cabeza: posición actual dentro del segmento actual
 |  +---------- separador: límite entre dos segmentos (temas o álbumes)
 +------------- lleno: lo ya escuchado
```

**Estados y ejemplos — Unicode:**

```text
reproduciendo   ━━━━━━━━━━━━●───────────────────────────  2:13 / 6:53
recién empieza  ●───────────────────────────────────────  0:00 / 6:53
detenido        ────────────────────────────────────────  --:-- / 6:53
sin duración    ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌ en vivo ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌  2:13 / --:--
álbum (temas)   ━━┃━━┃━━━┃━●───┃───┃────┃──────┃──┃───┃─  tema 4/10
cola (álbumes)  …┃━━━━┃━━━━┃━━━━┃●───┃─────┃────┃─────┃…  álbum 12/87
volumen         ■■■■■■■□□□  70%
duración rel.   ■■■■■□  6:53 (el más largo: 7:49)
```

**Los mismos — ASCII:**

```text
reproduciendo   [============>---------------------------]  2:13 / 6:53
recién empieza  [>---------------------------------------]  0:00 / 6:53
detenido        [----------------------------------------]  --:-- / 6:53
sin duración    [~~~~~~~~~~~~~~~ en vivo ~~~~~~~~~~~~~~~~]  2:13 / --:--
álbum (temas)   [==|==|===|=>---|---|----|------|--|---|-]  tema 4/10
cola (álbumes)  [..|===|====|====|>---|-----|----|----|..]  álbum 12/87
volumen         [#######---]  70%
duración rel.   [#####-]  6:53 (el más largo: 7:49)
```

#### Algoritmo

Entradas: `weights` (peso de cada segmento, en milisegundos), `position` (milisegundos desde el inicio del primer segmento), `width` (columnas disponibles **entre** `bar_open` y `bar_close`) y una política (`min_cells` por segmento y ventana opcional).

1. **Ventana** (solo si la política la tiene): partir de `current ± radius` recortado a los límites. Calcular las columnas fijas: un separador entre cada par de segmentos, y `bar_more` + un separador por cada extremo recortado. Si `segmentos × min_cells + fijas > width`, achicar el radio en 1 y repetir hasta radio 0. La barra pasa a representar solo los segmentos de la ventana: `position` se corrige restando el peso de los segmentos que quedaron afuera a la izquierda.
2. **Celdas disponibles** = `width − separadores − marcadores de ventana`.
3. **Reparto proporcional por mayor resto:** `ideal_i = peso_i × celdas / suma_pesos`; asignar `max(min_cells, floor(ideal_i))`; repartir las celdas que sobran de a una a los segmentos con mayor parte fraccionaria; si sobran celdas asignadas (por el mínimo), quitarlas de a una al segmento con mayor exceso sobre su ideal que todavía esté por encima del mínimo.
4. **Pintado de cada segmento:** si ya terminó → todo `bar_filled`; si no empezó → todo `bar_empty`; si es el actual → `cabeza = min(floor(fracción × celdas), celdas − 1)`, `bar_filled` antes de la cabeza, `bar_head` en la cabeza y `bar_empty` después.
5. **Separadores:** `bar_separator` entre segmentos, siempre, sin importar si están llenos o vacíos. Es la "línea" que marca el cambio de álbum (o de tema en la barra del álbum).

Casos especiales:

| Caso | Comportamiento |
|---|---|
| Detenido | Sin cabeza: todo `bar_empty`. Tiempos `--:--` |
| Duración desconocida del tema (stream) | Barra del tema llena de `bar_unknown` con ` en vivo ` centrado. Tiempo `2:13 / --:--` |
| Algún tema del álbum sin duración | La barra del álbum pasa a **modo por cantidad**: todos los pesos valen 1 y `position = índice del tema + fracción` (0 si no se conoce). La derecha muestra `tema 4/10` sin tiempos |
| Algún tema de la cola sin duración | Igual que arriba, para la barra de la cola: peso de cada álbum = cantidad de temas |
| `width` menor que `BAR_MIN_WIDTH` (constante, 10) | No se dibuja la barra (ver sección 7) |

Parámetros (constantes con nombre, no números sueltos): `ALBUM_BAR_MIN_CELLS = 1`, `QUEUE_BAR_MIN_CELLS = 3`, `QUEUE_BAR_WINDOW_RADIUS = 3`, `BAR_MIN_WIDTH = 10`.

**Diseño interno:** la función de reparto devuelve celdas abstractas, no texto:

```rust
pub enum BarCell { Filled, Head, Empty, Separator, More, Unknown }
```

Un segundo paso convierte cada `BarCell` en un `Span` con el glifo de `Glyphs` y el estilo de `theme.rs`. Así el algoritmo se testea sin ratatui ni glifos, y Unicode/ASCII y los colores son solo una tabla de traducción.

### 4.2 Separadores de álbum en las listas

En **Cola**, **Búsqueda** y el contenido de **Playlists**, las canciones se agrupan en *tramos*: secuencias **consecutivas** de temas del mismo álbum. Antes de cada tramo va una fila separadora a todo el ancho:

```text
── Pink Floyd - The Dark Side of the Moon (1973) ─────────────── 10 temas  42:51 ──
== Pink Floyd - The Dark Side of the Moon (1973) =============== 10 temas  42:51 ==
```

- **Clave del tramo:** `(Album, AlbumArtist)`; si falta `AlbumArtist`, `(Album, Artist)`. Usar `AlbumArtist` evita que un compilado se parta en un tramo por tema.
- **Temas sin álbum:** forman su propio tramo con el título `[sin álbum]`.
- **Derecha:** cantidad de temas del tramo y su duración (sin duración si falta alguna). Singular/plural: `1 tema`, `10 temas`.
- **Truncado:** si no entra, se trunca el título (artista - álbum) con `ellipsis`; la cantidad y la duración nunca se truncan.
- **No son seleccionables.** `j`/`k`, `g`/`G` y las páginas saltan las filas separadoras: la selección y los índices siguen siendo de canciones, igual que hoy.
- **Cabecera fija:** si la primera fila visible está en medio de un tramo, en su lugar se muestra la separadora de ese tramo. Con 1000+ temas siempre se sabe qué álbum se está mirando.
- **Marcadores de fila** (columnas fijas a la izquierda): `cursor` en la fila seleccionada, glifo de estado (`play`/`pause`) en el tema que suena. En ASCII: `>` y `|>`.
- Como el álbum ya está en la separadora, la columna "Álbum" **desaparece** de estas tablas y su espacio pasa a Título y Artista.

### 4.3 Medidores

Un medidor muestra un valor sobre un máximo en `n` celdas: `celdas_llenas = round(valor × n / máximo)`. Misma idea que la barra: devuelve `MeterCell::{Filled, Empty}` y se traduce a glifos.

| Uso | Celdas | Ejemplo Unicode | Ejemplo ASCII |
|---|---|---|---|
| Volumen | 10 | `■■■■■■■□□□  70%` | `[#######---]  70%` |
| Volumen sin mixer (`volume: -1`) | 10 | `□□□□□□□□□□  n/d` | `[----------]  n/d` |
| Duración relativa (columna Tema de Biblioteca) | 6 | `■■■■■□` | `[#####-]` |

La duración relativa compara cada tema con el **más largo del álbum mostrado**; con duración desconocida, la celda queda vacía (espacios).

### 4.4 Indicadores de estado

| Indicador | Unicode | ASCII | Regla |
|---|---|---|---|
| Reproducción | `▶` / `⏸` / `■` | `\|>` / `\|\|` / `[]` | Título del panel y fila del tema actual |
| Modos | `z ● aleatorio   r ○ repetir   y ◐ single   R ○ consumir` | `z[x] aleatorio  r[ ] repetir  y[1] single  R[ ] consumir` | La tecla va delante: enseña el atajo. `◐`/`1` = single "una vez". `consume oneshot` cuenta como activo |
| Conexión | `● host` / `✖ sin conexión, reintentando ◐` | `(*) host` / `(x) sin conexión, reintentando /` | Encabezado, a la derecha |

### 4.5 Navegación

- **Barra de desplazamiento** en el borde derecho de toda lista que no entre en pantalla: `▲ │ █ ▼` / `^ : # v`. Si la lista entra completa, no se dibuja.
- **Contador de posición** en el panel (`pos 304/1229`) y en el título de columnas o paneles (`Artista (223)`, `Playlists (3)`).
- **Migas de pan** en Biblioteca y Directorios: `Biblioteca › Pink Floyd › The Dark Side of the Moon` / `/ › Rock › Pink Floyd › 1973 - ...`. Si no entran, se recortan **por la izquierda** con `ellipsis` (lo importante es dónde estoy, no de dónde vine).
- **Columna o panel con foco:** su título va entre corchetes (`[Álbum (15)]`) y su selección en video inverso; las demás columnas muestran solo el `cursor`, sin video inverso.

---

## 5. Vistas

Los mockups de esta sección muestran solo la zona de la vista (encabezado y panel son los de la sección 3). Están en Unicode; en ASCII cambian solo los glifos según la tabla 2.1.

### 5.1 Cola (`1`)

Ver sección 3.1. Columnas: marcadores · `pos` (posición en la cola, 1-based) · Título · Artista · `dur`. Filas separadoras de álbum según 4.2. La tecla `o` lleva al tema actual y lo deja centrado verticalmente si se puede.

### 5.2 Biblioteca (`2`)

```text
│ Biblioteca › Pink Floyd › The Dark Side of the Moon                                              │
│ Artista (223)            │ [Álbum (15)]                  │ Tema (10)                             │
├──────────────────────────┼───────────────────────────────┼───────────────────────────────────────┤
│   Patti Smith            │   A Saucerful of Secrets      │▸  1 Speak to Me        1:05 ■□□□□□    │
│   Pearl Jam              │   Animals                     │   2 Breathe (In the …  2:49 ■■□□□□    │
│ ▸ Pink Floyd             │   Atom Heart Mother           │   3 On the Run         3:45 ■■■□□□    │
│   Pixies                 │   Meddle                      │   4 Time               6:53 ■■■■■□    │
│   Portishead             │   Obscured by Clouds          │   5 The Great Gig in…  4:44 ■■■■□□    │
│   PJ Harvey              │ ▸ The Dark Side of the Moon   │   6 Money              6:22 ■■■■■□    │
│   Queen                  │   The Division Bell           │   7 Us and Them        7:49 ■■■■■■    │
│   Radiohead              │   The Piper at the Gates of D…│   8 Any Colour You L…  3:26 ■■■□□□    │
│   Ramones                │   The Wall                    │   9 Brain Damage       3:46 ■■■□□□    │
│   R.E.M.                 │   Wish You Were Here          │  10 Eclipse            2:12 ■■□□□□    │
```

En ASCII:

```text
| Biblioteca > Pink Floyd > The Dark Side of the Moon                                              |
| Artista (223)            | [Álbum (15)]                  | Tema (10)                             |
+--------------------------+-------------------------------+---------------------------------------+
|   Patti Smith            |   A Saucerful of Secrets      |>  1 Speak to Me        1:05 [#-----]  |
|   Pearl Jam              |   Animals                     |   2 Breathe (In th...  2:49 [##----]  |
| > Pink Floyd             |   Atom Heart Mother           |   3 On the Run         3:45 [###---]  |
|   Pixies                 |   Meddle                      |   4 Time               6:53 [#####-]  |
|   Portishead             |   Obscured by Clouds          |   5 The Great Gig ...  4:44 [####--]  |
|   PJ Harvey              | > The Dark Side of the Moon   |   6 Money              6:22 [#####-]  |
|   Queen                  |   The Division Bell           |   7 Us and Them        7:49 [######]  |
|   Radiohead              |   The Piper at the Gates of...|   8 Any Colour You...  3:26 [###---]  |
|   Ramones                |   The Wall                    |   9 Brain Damage       3:46 [###---]  |
|   R.E.M.                 |   Wish You Were Here          |  10 Eclipse            2:12 [##----]  |
```

- Fila 1: migas de pan del recorrido actual.
- Proporción de columnas: 26 % / 32 % / 42 %. La columna Tema muestra número de pista, título, duración y la duración relativa (4.3).
- Contadores en los títulos de columna.

### 5.3 Directorios (`3`)

```text
│ / › Rock › Pink Floyd › 1973 - The Dark Side of the Moon                             12 entradas │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│ ▸ ▪ Scans/                                                                                       │
│   ♪ 01 - Speak to Me.flac                                                                   1:05 │
│   ♪ 02 - Breathe (In the Air).flac                                                          2:49 │
│   ♪ 03 - On the Run.flac                                                                    3:45 │
│   ♪ 04 - Time.flac                                                                          6:53 │
│   ♪ 05 - The Great Gig in the Sky.flac                                                      4:44 │
│   ♪ 06 - Money.flac                                                                         6:22 │
│   ♪ 07 - Us and Them.flac                                                                   7:49 │
│   ♪ 08 - Any Colour You Like.flac                                                           3:26 │
│   ♪ 09 - Brain Damage.flac                                                                  3:46 │
│   ♪ 10 - Eclipse.flac                                                                       2:12 │
│   ≡ dsotm.m3u                                                                                    │
```

- Fila 1: migas de pan desde la raíz (`/`) y, a la derecha, la cantidad de entradas.
- Insignias: `directory` (`▪`/`+`) con `/` al final del nombre; `now_playing` (`♪`/`~`) para temas, con su duración a la derecha; `playlist` (`≡`/`=`) para `.m3u`.
- Orden: el que devuelve MPD (no se reordena).

### 5.4 Búsqueda (`4`)

```text
│ Buscar: time_                                campo: (todo)  artista   álbum   título  Tab cambia │
│ 4 resultados en 4 álbumes                                                                        │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│── Cyndi Lauper - She's So Unusual (1983) ──────────────────────────────────────── 1 tema  4:01 ──│
│▸       Time After Time                              Cyndi Lauper                      4:01       │
│── Foo Fighters - One by One (2002) ────────────────────────────────────────────── 1 tema  4:25 ──│
│        Times Like These                             Foo Fighters                      4:25       │
│── Muse - Absolution (2003) ────────────────────────────────────────────────────── 1 tema  3:57 ──│
│        Time Is Running Out                          Muse                              3:57       │
│── Pink Floyd - The Dark Side of the Moon (1973) ───────────────────────────────── 1 tema  6:53 ──│
│        Time                                         Pink Floyd                        6:53       │
```

- Fila 1: el texto buscado (con el cursor `_` mientras se escribe) y, a la derecha, los campos posibles con el activo entre paréntesis.
- Fila 2: `N resultados en M álbumes`. Sin resultados: `Sin resultados para "time"`.
- Resultados agrupados por tramos de álbum (4.2).
- El texto que coincide con la búsqueda va **subrayado y en negrita** (no solo color) dentro de Título, Artista y la separadora.

### 5.5 Playlists (`5`)

```text
│ [Playlists (3)]            │ domingo: 5 temas  25:12                                             │
├────────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ ▸ ≡ domingo                │ ── Pink Floyd - Wish You Were Here (1975) ──────── 2 temas  18:39 ──│
│   ≡ para correr            │    Wish You Were Here                 Pink Floyd             5:34   │
│   ≡ rock nacional          │    Shine On You Crazy Diamond         Pink Floyd            13:05   │
│                            │ ── Radiohead - In Rainbows (2007) ───────────────── 3 temas  6:33 ──│
│                            │    Nude                               Radiohead              4:15   │
│                            │    Reckoner                           Radiohead              4:50   │
│                            │    Videotape                          Radiohead              4:39   │
```

- Panel izquierdo: nombres con la insignia `playlist`. Panel derecho: contenido de la seleccionada, agrupado por tramos.
- Título del panel derecho: `<nombre>: <cantidad> temas  <duración>`. No se piden los contenidos de todas las playlists para mostrar cantidades en el panel izquierdo (sería un comando por playlist).
- Sin playlists: `No hay playlists guardadas. S guarda la cola como playlist.` centrado.

### 5.6 Ayuda (`?`)

Superpuesta y centrada (limpiar el fondo con `Clear`). Dos columnas agrupadas por acción. **Debe entrar en 30 filas** (lección aprendida de `AGENTS.md`) y sale de `help_entries()` / `BINDINGS`: el mockup es ilustrativo.

```text
┌─ Ayuda ──────────────────────────────────────────────────────────────────────────────────┐
│                                                                                          │
│ Navegación                                    Reproducción                               │
│   j k ↓ ↑        moverse                        p Espacio      pausa / reanudar          │
│   g G            principio / final              s              detener                   │
│   Ctrl+d Ctrl+u  media página                   > <            siguiente / anterior      │
│   h l            columna / directorio           f b            adelantar / atrasar 5 s   │
│   Tab Shift+Tab  vista sig. / anterior          + -            volumen +5 / -5           │
│   1 .. 5         ir a la vista                                                           │
│                                               Modos                                      │
│ Cola y listas                                   z              aleatorio                 │
│   Enter          reproducir / entrar / cargar   r              repetir                   │
│   a A            agregar / agregar todos        y              single: no, sí, una vez   │
│   d              borrar (confirma)              R              consumir                  │
│   c              vaciar la cola (confirma)                                               │
│   S              guardar cola como playlist   Otros                                      │
│   o              ir al tema que suena           /  ?  q        buscar, ayuda, salir      │
│                                                                                          │
│                                                                           Esc o ? cierra │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

### 5.7 Diálogos

Centrados, superpuestos, con `Clear` debajo. Confirmación (las acciones destructivas siempre confirman):

```text
┌─ Confirmar ────────────────────────────────┐
│                                            │
│  ¿Vaciar la cola? Se quitan 1229 temas.    │
│                                            │
│            [s] sí        [n] no            │
│                                            │
└────────────────────────────────────────────┘
```

En ASCII:

```text
+- Confirmar --------------------------------+
|                                            |
|  ¿Vaciar la cola? Se quitan 1229 temas.    |
|                                            |
|            [s] sí        [n] no            |
|                                            |
+--------------------------------------------+
```

Texto libre (guardar playlist):

```text
┌─ Guardar cola como playlist ─────────────────────────┐
│                                                      │
│  Nombre: [rock nacional_                          ]  │
│                                                      │
│  Enter guarda   Esc cancela                          │
│                                                      │
└──────────────────────────────────────────────────────┘
```

- El campo muestra el cursor `_` al final del texto y se desplaza horizontalmente si el texto es más largo que el campo.
- Si la validación falla (`InputError` de `PlaylistName`), el mensaje aparece en la última fila del diálogo con `message_error`, y el diálogo sigue abierto.

---

## 6. Colores y estilos

Todos los estilos viven en `theme.rs`. Si falta un rol, se agrega ahí. Solo colores ANSI básicos.

| Rol | Estilo sugerido | Sin color se distingue por |
|---|---|---|
| Barra: lleno y cabeza | acento (cian) ; cabeza en negrita | glifo distinto (`━●` / `=>`) |
| Barra: vacío | gris oscuro | glifo `─` / `-` |
| Barra: separador | color normal, negrita | glifo `┃` / `\|` |
| Fila separadora de álbum | título en negrita; relleno en gris oscuro | el relleno `──` / `==` |
| Tema que suena | acento + negrita | glifo de estado |
| Selección | video inverso | glifo `cursor` |
| Pestaña / columna con foco | negrita + video inverso | corchetes |
| Modo activo / inactivo | acento / atenuado (`DIM`) | `●`/`○`/`◐` o `[x]`/`[ ]`/`[1]` |
| Sin conexión, errores | rojo + negrita | `✖` / `(x)` |
| Coincidencias de búsqueda | negrita + subrayado | modificadores |

---

## 7. Tamaño de terminal (responsivo)

| Tamaño | Modo | Cambios |
|---|---|---|
| ≥ 100 × 28 | Completo | Todo lo descrito |
| ≥ 60 × 16 | Compacto | Panel de 4 filas: solo barra del tema; modos con letras (`z● r○ y◐ R○`); pestañas abreviadas; tablas sin la columna Artista por debajo de 80 columnas |
| Menor | Aviso | Solo un mensaje con el mínimo y el tamaño actual |

Los umbrales son constantes con nombre (`FULL_MIN_WIDTH`, `FULL_MIN_HEIGHT`, `COMPACT_MIN_WIDTH`, `COMPACT_MIN_HEIGHT`). El modo se decide en cada dibujado a partir de `frame.area()`: es una función pura `layout_mode(width, height) -> LayoutMode`.

Compacto — Unicode (72 × 16):

```text
┌─ mpctui ────────────────────────────────────────── ● localhost:6600 ─┐
│ [1 Cola]  2 Bibl.  3 Dir.  4 Búsq.  5 Playl.                         │
├──────────────────────────────────────────────────────────────────────┤
│      303  On the Run                         Pink Floyd       3:45   │
│  ▶   304  Time                               Pink Floyd       6:53   │
│      305  The Great Gig in the Sky           Pink Floyd       4:44   │
│▸     306  Money                              Pink Floyd       6:22   │
│      307  Us and Them                        Pink Floyd       7:49   │
│      308  Any Colour You Like                Pink Floyd       3:26   │
├──────────────────────────────────────────────────────────────────────┤
│ ▶ ━━━━━━━━━━━━━━━━━●──────────────────────────────────── 2:13/6:53   │
│ ♪ Time - Pink Floyd                     vol 70%  tema 4/10  304/1229 │
│ z● r○ y◐ R○                                                          │
└──────────────────────────────────────────────────────────────────────┘
 ? ayuda
```

Compacto — ASCII:

```text
+- mpctui ---------------------------------------- (*) localhost:6600 -+
| [1 Cola]  2 Bibl.  3 Dir.  4 Búsq.  5 Playl.                         |
+----------------------------------------------------------------------+
|      303  On the Run                         Pink Floyd       3:45   |
|  |>  304  Time                               Pink Floyd       6:53   |
|      305  The Great Gig in the Sky           Pink Floyd       4:44   |
|>     306  Money                              Pink Floyd       6:22   |
|      307  Us and Them                        Pink Floyd       7:49   |
|      308  Any Colour You Like                Pink Floyd       3:26   |
+----------------------------------------------------------------------+
| |>[================>-----------------------------------] 2:13/6:53   |
| ~ Time - Pink Floyd                     vol 70%  tema 4/10  304/1229 |
| z[x] r[ ] y[1] R[ ]                                                  |
+----------------------------------------------------------------------+
 ? ayuda
```

Demasiado chica:

```text
+- mpctui -----------------------------+
|                                      |
|  La terminal es muy chica.           |
|  Mínimo: 60 x 16. Actual: 52 x 12.   |
|                                      |
+--------------------------------------+
```

---

## 8. Datos que necesita el diseño

YAGNI: se agrega solo lo que se muestra.

| Dato | Origen | Para qué |
|---|---|---|
| `AlbumArtist` (opcional) | tag de `playlistinfo`, `find`, `search`, `listplaylistinfo` | Clave de los tramos (compilados) |
| `Date` (opcional, texto) | mismo origen | Año en separadoras y panel. Se muestra solo si empieza con 4 dígitos; si no, se omite. Es un tag libre del usuario: no es un error de protocolo |
| Duración de cada tema | ya existe (`duration` / `Time`) | Barras, totales |
| Posición del tema actual (`song`) y `elapsed` | ya existe en `Status` / `PlaybackClock` | Cabezas de las barras |

**Línea de tiempo de la cola** (función pura en `src/domain/`, no conoce MPD ni la terminal):

```rust
pub struct AlbumRun {
    pub first_index: usize,          // índice del primer tema del tramo en la lista
    pub song_count: usize,
    pub duration: Option<Duration>,  // None si algún tema no tiene duración
}

pub struct Timeline {
    pub runs: Vec<AlbumRun>,
    pub song_offsets: Vec<Option<Duration>>, // inicio de cada tema desde el principio
    pub total: Option<Duration>,
}

pub fn timeline(songs: &[Song]) -> Timeline;
```

- Se calcula **una vez por cambio de la cola** (evento `Subsystem::Queue`) y se guarda en `AppState`, no en cada dibujado. Con 1229 temas es O(n) y trivial.
- El tramo de un índice se busca con `partition_point` (O(log n)).
- La misma función agrupa los resultados de búsqueda y el contenido de una playlist (DRY).
- Posición en la cola = `song_offsets[actual] + elapsed`. Posición en el álbum = esa misma menos `song_offsets[run.first_index]`.

---

## 9. Implementación

### 9.1 Módulos nuevos o modificados

| Archivo | Responsabilidad |
|---|---|
| `src/tui/glyphs.rs` | `pub struct Glyphs { pub play: &'static str, ... , pub border: border::Set }` y dos constantes: `pub const UNICODE: Glyphs` y `pub const ASCII: Glyphs`. Único lugar con glifos literales |
| `src/tui/gauges.rs` | Funciones puras: `segmented_bar(...) -> Vec<BarCell>`, `allocate_cells(...)`, `meter(...) -> Vec<MeterCell>`, y su traducción a `Line` con `Glyphs` + `theme` |
| `src/tui/widgets.rs` | Nuevo widget `GroupedSongTable` (lista con separadoras, cabecera fija y barra de desplazamiento) usado por Cola, Búsqueda y Playlists |
| `src/tui/views/now_playing.rs` | Panel "ahora suena" (completo y compacto) |
| `src/tui/layout.rs` | `LayoutMode` y `layout_mode(width, height)` |
| `src/tui/format.rs` | Duraciones (`m:ss`, `h:mm:ss`), truncado con `glyphs.ellipsis` por ancho visible, año desde `Date` |
| `src/domain/timeline.rs` | `AlbumRun`, `Timeline`, `timeline()` |
| `src/domain/` + `parse.rs` | `Song` gana `album_artist: Option<String>` y `date: Option<String>` |
| `src/config.rs` | `GlyphMode::{Unicode, Ascii}`, campo `glyphs` en el TOML, `MPCTUI_GLYPHS`, `resolve_glyph_mode(setting, locale_env) -> GlyphMode` pura |
| `src/factory.rs` | Elige `&glyphs::UNICODE` o `&glyphs::ASCII` según `Settings` y lo **inyecta** en `App` |
| `README.md` | Opción `glyphs`, variable `MPCTUI_GLYPHS`, advertencia de ancho ambiguo, nuevo indicador de conexión |

Las vistas reciben `&Glyphs` desde `App` (inyección): ninguna vista elige el juego ni lee el entorno.

### 9.2 Detalles de ratatui que rompen el fallback si se olvidan

- **Bordes:** `Block::bordered().border_set(glyphs.border)`. Para ASCII, un `border::Set` propio con `+`, `-`, `|`. El de Unicode es `border::PLAIN`.
- **`Tabs`:** el divisor por defecto es `│`. Poner `.divider(" ")` o un glifo de `Glyphs`.
- **`Scrollbar`:** setear explícitamente `begin_symbol`, `end_symbol`, `track_symbol` y `thumb_symbol` desde `Glyphs` (los de fábrica son Unicode).
- **No usar `Gauge`, `LineGauge`, `Sparkline` ni `BarChart`:** dibujan con bloques y líneas Unicode. Las barras se arman con `Paragraph` + la `Line` de `gauges.rs`.
- **`highlight_symbol`** de `List`/`Table`: `glyphs.cursor` seguido de un espacio, con `HighlightSpacing::Always` para que las columnas no se corran al seleccionar.
- **Tablas con separadoras:** ratatui no tiene celdas que ocupen varias columnas. `GroupedSongTable` dibuja solo las filas visibles como `Line`s, con columnas calculadas con `Layout`. Cumple además la lección de no clonar listas largas al dibujar.
- **Truncado:** siempre por ancho visible con `glyphs.ellipsis`, nunca cortando bytes (panic con UTF-8) ni con `…` literal.

### 9.3 Tests

1. **Unitarios de `gauges.rs`** (sobre `BarCell`, sin glifos): un segmento al 0 %, 32 % y 100 %; reparto por mayor resto que suma exactamente `width`; mínimo por segmento; ventana que se achica; modo por cantidad; detenido; duración desconocida.
2. **Unitarios de `timeline()`**: tramos consecutivos, mismo álbum repetido más adelante (tramo nuevo), compilado con `AlbumArtist`, temas sin álbum, duración desconocida.
3. **Unitarios de `resolve_glyph_mode`**: `LC_ALL` vacío y `LANG=es_AR.UTF-8` → Unicode; `LANG=C` → ASCII; `TERM=linux` → ASCII; valor inválido → error.
4. **Glifos:** todo campo de `ASCII` es ASCII (`str::is_ascii`); todo campo de una celda de `UNICODE` mide 1 con `unicode-width`.
5. **Render con `TestBackend`** de cada vista, en los dos juegos, a 100 × 30, 72 × 16 y 40 × 10. En modo ASCII, el buffer no puede tener **ningún** carácter de los rangos U+2190–U+2BFF (flechas, símbolos técnicos, cajas, bloques, figuras geométricas, símbolos varios) ni U+2026 (`…`). Las letras con tilde están permitidas.
6. **Manual con tmux** (ver `AGENTS.md`), en ambos modos, **solo con teclas de lectura**:

```bash
tmux new-session -d -s prueba -x 100 -y 30 ./target/debug/mpctui
tmux new-session -d -s ascii -x 100 -y 30 "MPCTUI_GLYPHS=ascii ./target/debug/mpctui"
tmux capture-pane -t ascii -p | grep -nP '[\x{2190}-\x{2BFF}\x{2026}]' && echo "FALLA: glifo no ASCII"
```

---

## 10. Fases

Cada fase termina con `cargo fmt`, `clippy` sin warnings, `cargo test` en verde, revisión contra `AGENTS.md` y `PLAN.md` actualizado. Frenar al terminar cada fase para que el usuario revise.

| Fase | Contenido | Listo cuando |
|---|---|---|
| **D1 · Glifos** | `glyphs.rs` con ambos juegos; `glyphs` en config + `MPCTUI_GLYPHS` + detección automática; bordes, pestañas, scrollbar, cursor y elipsis desde `Glyphs`; reemplazar el `✖` fijo | Con `MPCTUI_GLYPHS=ascii` ninguna pantalla actual tiene glifos fuera de ASCII (test 9.3.5); con Unicode se ve igual que antes salvo los glifos nuevos |
| **D2 · Barra y panel** | `gauges.rs` (barra segmentada + medidor); panel "ahora suena" con título, barra del tema, volumen y modos | Barra del tema avanza con el tick; pausa, detenido y stream se ven como en 4.1 |
| **D3 · Línea de tiempo** | Parseo de `AlbumArtist` y `Date`; `timeline()`; barras del álbum y de la cola con separadores y ventana | Las tres barras alineadas; los separadores coinciden con los cambios de tema/álbum |
| **D4 · Listas agrupadas** | `GroupedSongTable` con separadoras, cabecera fija y scrollbar en Cola, Búsqueda y Playlists | La navegación salta separadoras; `o` sigue funcionando con 1229 temas; sin clonar la cola al dibujar |
| **D5 · Vistas** | Migas de pan, insignias, contadores, duración relativa, foco de columna, resaltado de búsqueda, línea inferior desde `BINDINGS`, diálogos y ayuda con el nuevo estilo | Cada vista coincide con su mockup en ambos juegos |
| **D6 · Responsivo** | `layout_mode`, modo compacto y aviso de terminal chica | Render correcto a 100 × 30, 72 × 16 y 40 × 10 |

---

## 11. Fuera de alcance (YAGNI)

Estos cuatro agregados están especificados en **`DESIGN_EXTRA.md`** y se implementan **después** de D1–D6, en sus propias fases (E1–E4):

- Visualizador de espectro con barras ASCII/Unicode leyendo `/tmp/mpd.fifo`.
- Cuenta regresiva del próximo reintento de conexión.
- Indicador de actualización de la base de datos (`updating_db`).
- Formato de audio y bitrate en el panel.

Ideas que encajan con el estilo pero **no** se implementan hasta que se pidan:

- Temas de color configurables.
- Glifos configurables uno por uno (solo se elige el juego completo).
