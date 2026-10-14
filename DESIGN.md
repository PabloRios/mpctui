# DESIGN.md — mpctui: diseño visual de la TUI

Especificación visual de mpctui: cómo se ve cada vista, qué ayudas gráficas muestra, cómo usa el color y cómo se dibuja. Reúne en un solo documento el diseño base, los extras visuales (formato de audio, actualización de la base, cuenta regresiva de reconexión y visualizador) y el color (roles, temas y profundidad), y describe lo que hoy está implementado.

> **Antes de empezar:** leer `AGENTS.md` completo. Todo lo de acá respeta SOLID, DRY, KISS, YAGNI, la separación `domain ← repository ← service ← tui`, fail fast y "sin números ni strings mágicos". Si algo de este diseño choca con `AGENTS.md`, gana `AGENTS.md` y se avisa al usuario.

Los mockups están dibujados a **100 columnas** y en español (la interfaz está en inglés por defecto; ver `language` en `config.toml.dist`). Los anchos y los datos son ilustrativos: lo que manda son las reglas de cada sección. Están en Unicode y, donde la diferencia importa, también en ASCII. La vista previa con los colores reales de los cuatro temas está en el lienzo de diseño "mpctui · temas de color".

---

## 1. Principios visuales

1. **Unicode por defecto, ASCII como fallback.** Hay dos juegos de glifos con exactamente el mismo layout. Unicode (`▶ ⏸ ■ ♪ ▸ ≡`, bordes `┌─┐`) se ve mejor; ASCII (`|> || [] ~ > =`, bordes `+-|`) funciona en cualquier terminal, consola de Linux (`TERM=linux`), sesiones serie, `screen` viejo o fuentes sin esos símbolos. El usuario puede forzar cualquiera de los dos.
2. **La información nunca depende solo del color.** Todo estado se ve con un glifo o un modificador (negrita, video inverso) además del color. Con un terminal monocromo la interfaz tiene que seguir siendo usable.
3. **Todo glifo ocupa exactamente una columna** (o un ancho fijo y conocido en ASCII, como `|>`). Los anchos se miden con `unicode-width` (`Line::width()` / `Span::width()` de ratatui o `UnicodeWidthStr::width`), **nunca con `.len()`**: `━` ocupa 3 bytes y 1 columna.
4. **Sin emoji.** Nada con presentación de emoji (color, doble ancho). Ver advertencias en la sección 2.3.
5. **Ningún glifo literal fuera de `src/tui/glyphs.rs`.** Las vistas y widgets piden el glifo por su rol (`glyphs.play`, `glyphs.bar_filled`), nunca escriben `"▶"` ni `"|>"`.
6. **El texto de UI sigue en español con tildes.** Las letras acentuadas son texto, no gráficos: no se reemplazan en modo ASCII.
7. **Color con significado y con fallback.** Cada color es un **rol** (`artist`, `playing`, `error`…): las vistas piden roles, nunca colores. El texto común queda con el color de la terminal y el fondo no se pinta. Hay cuatro temas y la cantidad de colores se adapta a la terminal (24 bits → 256 → 16 → sin color), igual que los glifos. Detalle en la sección 6.
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
| `partition_in_use` | `◆` | `*` | La partición que usa mpctui (vista Salidas) |
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

### 2.2 Glifos del visualizador

También son campos de `Glyphs`, con las mismas reglas: una columna en Unicode, sin emoji, nada literal fuera de `glyphs.rs`.

| Rol (campo) | Unicode | ASCII | Uso |
|---|---|---|---|
| `spectrum_levels` | `" ▁▂▃▄▅▆▇█"` (9: vacío + 8 octavos) | `" .:#"` (4: vacío + 3) | Barras. La cantidad de niveles sale de la longitud del arreglo |
| `spectrum_peak` | `▔` | `-` | Marca de pico en Barras |
| `mirror_up` / `mirror_down` | `" ▄█"` / `" ▀█"` | `" .#"` / `" '#"` | Espejo estéreo |
| `plot` | `Plot::Braille` | `Plot::Cells { levels: "'-.", join: "\|" }` | Onda y Vectorscopio. El braille se genera por código (`BRAILLE_BASE` + bits) dentro de `pages/visual/canvas.rs`; es el único caso de glifos calculados |
| `scope_guides` | braille punteado | `\` y `/` | Guías L y R del Vectorscopio |
| `scope_density` | — | `" .+#"` | Densidad de puntos del Vectorscopio en ASCII |
| `shades` | `" ░▒▓█"` | `" .:-=+*#%@"` | Cascada |
| `meter_eighths` | `"▏▎▍▌▋▊▉█"` | `"#"` (sin parciales) | Barras de los Medidores |
| `meter_peak` | `│` | `\|` | Pico retenido de los Medidores |
| `scale_tick` | `┴` | `+` | Marcas de la escala en dBFS |
| `correlation_center` / `correlation_marker` | `┼` / `●` | `+` / `o` | Correlación |
| `page_active` / `page_inactive` | `●` / `○` | `*` / `.` | Indicador de forma en la fila de título |
| `updating` | reutiliza `spinner` | reutiliza `spinner` | Indicador de actualización |

### 2.3 Cómo se elige el juego

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

### 2.4 Advertencias sobre Unicode (por qué existe el fallback)

- **Ancho ambiguo.** `▶ ■ ● ○ ◐ ▸ ♪ ≡ □` tienen ancho "ambiguo" en Unicode (East Asian Width = A). Las terminales configuradas con "ancho ambiguo = doble" (común con locales CJK) los dibujan en 2 columnas y rompen la alineación. No se puede detectar de forma confiable: para eso está `glyphs = "ascii"`. Documentarlo en el README.
- **`⏸` (U+23F8) puede verse como emoji** en algunas terminales y fuentes (a color y en 2 columnas). Se usa igual porque el diseño lo pide, pero queda aislado en `glyphs.pause`: si da problemas, se cambia en un solo lugar por `‖` (U+2016). No agregar el selector de variación U+FE0E: no todas las terminales lo respetan y complica el cálculo de anchos.
- **Fuentes sin el glifo.** Si la fuente no tiene un símbolo, la terminal muestra un cuadrado o `?`. De nuevo: fallback ASCII.
- **Visualizador.** Los bloques `▁…█`, `▔`, `▄`, `▀`, `░▒▓` y `▏…▉` también tienen ancho ambiguo. En terminales con "ancho ambiguo = doble" el visualizador se rompe: es otro caso para `glyphs = "ascii"`. El braille no tiene ese problema, pero sí puede faltar en la fuente.

---

## 3. Layout general

La pantalla se divide en cuatro zonas, siempre en este orden vertical:

| Zona | Alto | Contenido |
|---|---|---|
| Encabezado | 3 filas | Nombre, estado de la conexión, pestañas de vistas y resumen de la cola |
| Vista | el resto | La vista activa (sección 5) |
| Panel "ahora suena" | 3 filas más una por barra opcional (completo) / 3 (compacto), más su regla | Título, barras de progreso, volumen, modos |
| Línea inferior | 1 fila | Atajos de la vista activa o el último mensaje |

### 3.1 Pantalla completa — Unicode (vista Cola, 100 × 30)

```text
┌─ mpctui ────────────────────────────────────────────────────────────────────── ● localhost:6600 ─┐
│ [1 Cola]  2 Bibl.   3 Dir.   4 Búsq.   5 Playl.   6 Visual   7 Letra   8 Sal.                    │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│      pos   # Título                                    Álbum                              dur    │
│── Pink Floyd - Meddle (1971) ───────────────────────────────────────────────── 6 temas  46:46 ──▲│
│      297   3 Fearless                                  Meddle                            6:08   ││
│      298   4 San Tropez                                Meddle                            3:44   ││
│      299   5 Seamus                                    Meddle                            2:16   ││
│      300   6 Echoes                                    Meddle                           23:31   ││
│── Pink Floyd - The Dark Side of the Moon (1973) ───────────────────────────── 10 temas  42:51 ──█│
│      301   1 Speak to Me                               The Dark Side of the Moon         1:05   █│
│      302   2 Breathe (In the Air)                      The Dark Side of the Moon         2:49   ││
│      303   3 On the Run                                The Dark Side of the Moon         3:45   ││
│  ▶   304   4 Time                                      The Dark Side of the Moon         6:53   ││
│      305   5 The Great Gig in the Sky                  The Dark Side of the Moon         4:44   ││
│▸     306   6 Money                                     The Dark Side of the Moon         6:22   ││
│      307   7 Us and Them                               The Dark Side of the Moon         7:49   ││
│      308   8 Any Colour You Like                       The Dark Side of the Moon         3:26   ││
│      309   9 Brain Damage                              The Dark Side of the Moon         3:46   ││
│      310  10 Eclipse                                   The Dark Side of the Moon         2:12   ││
│── Radiohead - OK Computer (1997) ──────────────────────────────────────────── 12 temas  53:21 ──││
│      311   1 Airbag                                    OK Computer                       4:44   ▼│
├─ ▶ reproduciendo ─────────────────────────────────── FLAC  44.1 kHz  16 bit  estéreo  1411 kbps ─┤
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
| [1 Cola]  2 Bibl.   3 Dir.   4 Búsq.   5 Playl.   6 Visual   7 Letra   8 Sal.                    |
+--------------------------------------------------------------------------------------------------+
|      pos   # Título                                    Álbum                              dur    |
|== Pink Floyd - Meddle (1971) ================================================= 6 temas  46:46 ==^|
|      297   3 Fearless                                  Meddle                            6:08   :|
|      298   4 San Tropez                                Meddle                            3:44   :|
|      299   5 Seamus                                    Meddle                            2:16   :|
|      300   6 Echoes                                    Meddle                           23:31   :|
|== Pink Floyd - The Dark Side of the Moon (1973) ============================= 10 temas  42:51 ==#|
|      301   1 Speak to Me                               The Dark Side of the Moon         1:05   #|
|      302   2 Breathe (In the Air)                      The Dark Side of the Moon         2:49   :|
|      303   3 On the Run                                The Dark Side of the Moon         3:45   :|
|  |>  304   4 Time                                      The Dark Side of the Moon         6:53   :|
|      305   5 The Great Gig in the Sky                  The Dark Side of the Moon         4:44   :|
|>     306   6 Money                                     The Dark Side of the Moon         6:22   :|
|      307   7 Us and Them                               The Dark Side of the Moon         7:49   :|
|      308   8 Any Colour You Like                       The Dark Side of the Moon         3:26   :|
|      309   9 Brain Damage                              The Dark Side of the Moon         3:46   :|
|      310  10 Eclipse                                   The Dark Side of the Moon         2:12   :|
|== Radiohead - OK Computer (1997) ============================================ 12 temas  53:21 ==:|
|      311   1 Airbag                                    OK Computer                       4:44   v|
+- |> reproduciendo ---------------------------------- FLAC  44.1 kHz  16 bit  estéreo  1411 kbps -+
| ~ Time                                             Pink Floyd - The Dark Side of the Moon (1973) |
| tema   [=================>------------------------------------]                      2:13 / 6:53 |
| álbum  [=|===|====|==>----|-----|-------|--------|----|----|--]         tema 4/10   9:52 / 42:51 |
| cola   [..|=====|======|======|=>----|-------|------|------|..]  álbum 12/87  8:11:05 / 65:04:13 |
| vol [#######---]  70%     z[x] aleatorio  r[ ] repetir  y[1] single  R[ ] consumir  pos 304/1229 |
+--------------------------------------------------------------------------------------------------+
 Enter reproducir   d borrar   o ir al actual   / buscar   ? ayuda
```

### 3.3 Encabezado

- Título del bloque: `mpctui`. A la derecha, el estado de la conexión: `● localhost:6600` (o el socket unix) cuando hay conexión. Si mpctui usa una partición que no es `default`, se agrega su nombre: `● localhost:6600 · cocina` (sección 5.8).
- **Actualización de la base de datos.** Mientras MPD actualiza la base, el título lo indica con el spinner y el tiempo transcurrido:

```text
┌─ mpctui ─ ◐ actualizando biblioteca 0:42 ───────────────────────────────────── ● localhost:6600 ─┐
+- mpctui - / actualizando biblioteca 0:42 ----------------------------------- (*) localhost:6600 -+
```

- El spinner (`glyphs.spinner`) avanza un cuadro por tick. El tiempo se cuenta desde que mpctui **vio** la actualización (MPD no informa cuándo empezó ni cuánto falta, así que no hay porcentaje ni barra: sería inventado).
- Al terminar: desaparece el indicador y la línea inferior muestra `· Biblioteca actualizada (1:05)` durante `MESSAGE_DURATION`. Las vistas Biblioteca y Directorios se refrescan con `SelectionPolicy::Keep` (eso ya ocurre con el evento `database`).
- En modo compacto queda solo el spinner junto a `mpctui` (sección 7).

- **Sin conexión: cuenta regresiva del próximo reintento.** Dos estados, a la derecha del título:

```text
┌─ mpctui ───────────────────────────────── ✖ sin conexión, reintento en 5 s ■■■□□□□□ (intento 4) ─┐
┌─ mpctui ────────────────────────────────────────────── ✖ sin conexión, conectando ◐ (intento 5) ─┐
+- mpctui ----------------------------- (x) sin conexión, reintento en 5 s [###-----] (intento 4) -+
+- mpctui -------------------------------------------- (x) sin conexión, conectando / (intento 5) -+
```

| Estado | Texto | Gráfico |
|---|---|---|
| Esperando | `sin conexión, reintento en N s` | Medidor de 8 celdas (sección 4.3) con la parte **ya esperada** de la espera actual: se llena hasta que se reintenta |
| Conectando | `sin conexión, conectando` | Spinner |
| Ambos | `(intento N)` al final, desde el segundo intento | — |

- `N` se redondea **hacia arriba**: con 0,3 s restantes se ve `1 s`, nunca `0 s`.
- Cuando la espera supera 59 s se muestra `m:ss` (con el backoff actual de 1 s → 30 s no pasa, pero la función de formato no debe asumirlo).
- Al reconectar: el encabezado vuelve a `● host` y la línea inferior muestra `· Conexión recuperada` durante `MESSAGE_DURATION`.
- Si no entra todo, se quita primero `(intento N)`, después el medidor, después `sin conexión, `. Lo último que queda es `✖ reintento en 5 s`.

- **Pestañas:** una por vista (`1`–`8`): Cola, Biblioteca, Directorios, Búsqueda, Playlists, Visual, Letra y Salidas. La activa va entre corchetes `[1 Cola]`, con el rol `tab_active`; las demás, con un espacio a cada lado para que nada se mueva al cambiar de vista. Siempre se muestra el número, que es el atajo. Si los títulos completos no entran (o en modo compacto) van abreviados (`Bibl.`, `Dir.`, `Búsq.`, `Playl.`, `Sal.`). Con las letras desactivadas no hay pestaña `7`.
- A la derecha de las pestañas, si entra: `cola: <cantidad> temas  <duración total>`. Si alguna duración es desconocida, `cola: 1229 temas` sin duración.

### 3.4 Panel "ahora suena"

Fila por fila:

1. **Regla que abre el panel:** glifo de estado + palabra (`▶ reproduciendo`, `⏸ en pausa`, `■ detenido`) y, a la derecha, el formato de audio (abajo).
2. **Tema:** `♪ <título>` a la izquierda; `<artista> - <álbum> (<año>)` alineado a la derecha. Si no entra, se trunca primero el álbum, después el artista y por último el título, siempre con `ellipsis`. Si falta el año, sin paréntesis. Con `[lyrics] now_playing_line = true`, después del título va la línea actual de la letra sincronizada (`♪ Time · Ticking away the moments`); se recorta después del álbum y antes del artista.
3. **Barra del tema** (`tema`): progreso del tema actual. Derecha: `2:13 / 6:53`.
4. **Barra del álbum** (`álbum`, opcional con `album_progress = true`): el álbum en curso, **un segmento por tema**, separados por `┃`/`|`. Derecha: `tema 4/10   9:52 / 42:51`.
5. **Barra de la cola** (`cola`, opcional con `queue_progress = true`): la cola completa, **un segmento por álbum**, separados por `┃`/`|`, con ventana alrededor del álbum actual. Derecha: `álbum 12/87  8:11:05 / 65:04:13`.
6. **Volumen y modos:** medidor de volumen + porcentaje; los cuatro modos con su tecla delante (`z ● aleatorio`, en ASCII `z[x] aleatorio`); a la derecha `pos 304/1229`.

Las barras tienen **el mismo ancho y empiezan en la misma columna**, así se leen como una sola columna de progreso a distintas escalas. Las etiquetas (`tema`, `álbum`, `cola`) ocupan un ancho fijo y el texto de la derecha se alinea a la derecha. Las barras del álbum y de la cola están ocultas por defecto: el panel completo tiene 3 filas más una por barra activa.

En pausa solo cambia el título del bloque y la barra deja de avanzar:

```text
├─ ⏸ en pausa ─────────────────────────────────────────────────────────────────────────────────────┤
│ ♪ Time                                             Pink Floyd - The Dark Side of the Moon (1973) │
```

Detenido: las tres barras vacías, tiempos `--:--`, y la fila 2 muestra el tema que sonaría al dar play (si MPD informa uno) o `sin reproducción`.

#### Formato de audio y bitrate

El formato va a la derecha del título de la regla del panel, en la misma línea del borde. Así no ocupa filas nuevas:

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
- En modo compacto (sección 7) no se muestra.
- Colores según la calidad del audio: sección 6.5.

### 3.5 Línea inferior

- Por defecto: los atajos más útiles de la vista activa (`Enter reproducir   d borrar   o ir al actual   / buscar   ? ayuda`).
- **DRY:** las teclas salen de la tabla `BINDINGS`. Cada vista declara solo la lista de `Action` que quiere sugerir; el texto de la tecla se busca en `BINDINGS`, nunca se escribe a mano.
- Un mensaje (error o confirmación de una acción) reemplaza a los atajos durante `MESSAGE_DURATION` (constante, 5 s) con su prefijo: `✖ No se pudo conectar: ...` / `(x) No se pudo conectar: ...`. Si ya existe una línea de mensajes, se reutiliza.

---

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
| `width` menor que `BAR_MIN_WIDTH` (constante, 10) | No se dibuja la barra (sección 7) |

Parámetros (constantes con nombre, no números sueltos): `ALBUM_BAR_MIN_CELLS = 1`, `QUEUE_BAR_MIN_CELLS = 3`, `QUEUE_BAR_WINDOW_RADIUS = 3`, `BAR_MIN_WIDTH = 10`.

**Diseño interno:** la función de reparto devuelve celdas abstractas, no texto:

```rust
pub enum BarCell { Filled, Head, Empty, Separator, More, Unknown }
```

Un segundo paso convierte cada `BarCell` en un `Span` con el glifo de `Glyphs` y el estilo de su rol de color (sección 6.5): las celdas llenas toman el degradado `progress` según su posición en la barra. Así el algoritmo se testea sin ratatui ni glifos, y Unicode/ASCII y los colores son solo una tabla de traducción.

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
- **Columnas:** marcadores · `pos` (solo en la cola) · `#` (número de pista, sin el total: `3/12` → `3`) · Título · Álbum · `dur`. El artista ya está en la separadora, así que no tiene columna. Por debajo de 80 columnas se oculta Álbum y su espacio pasa a Título.

### 4.3 Medidores

Un medidor muestra un valor sobre un máximo en `n` celdas: `celdas_llenas = round(valor × n / máximo)`. Misma idea que la barra: devuelve `MeterCell::{Filled, Empty}` y se traduce a glifos.

| Uso | Celdas | Ejemplo Unicode | Ejemplo ASCII |
|---|---|---|---|
| Volumen | 10 | `■■■■■■■□□□  70%` | `[#######---]  70%` |
| Volumen sin mixer (`volume: -1`) | 10 | `□□□□□□□□□□  n/d` | `[----------]  n/d` |
| Espera de la reconexión | 8 | `■■■□□□□□` | `[###-----]` |
| Duración relativa (columna Tema de Biblioteca) | 6 | `■■■■■□` | `[#####-]` |

La duración relativa compara cada tema con el **más largo del álbum mostrado**; con duración desconocida, la celda queda vacía (espacios).

### 4.4 Indicadores de estado

| Indicador | Unicode | ASCII | Regla |
|---|---|---|---|
| Reproducción | `▶` / `⏸` / `■` | `\|>` / `\|\|` / `[]` | Título del panel y fila del tema actual |
| Modos | `z ● aleatorio   r ○ repetir   y ◐ single   R ○ consumir` | `z[x] aleatorio  r[ ] repetir  y[1] single  R[ ] consumir` | La tecla va delante: enseña el atajo. `◐`/`1` = single "una vez". `consume oneshot` cuenta como activo |
| Conexión | `● host` / `✖ sin conexión, reintento en 5 s ■■■□□□□□` | `(*) host` / `(x) sin conexión, reintento en 5 s [###-----]` | Encabezado, a la derecha (sección 3.3) |

### 4.5 Navegación

- **Barra de desplazamiento** en el borde derecho de toda lista que no entre en pantalla: `▲ │ █ ▼` / `^ : # v`. Si la lista entra completa, no se dibuja.
- **Contador de posición** en el panel (`pos 304/1229`) y en el título de columnas o paneles (`Artista (223)`, `Playlists (3)`).
- **Migas de pan** en Biblioteca y Directorios: `Biblioteca › Pink Floyd › The Dark Side of the Moon` / `/ › Rock › Pink Floyd › 1973 - ...`. Si no entran, se recortan **por la izquierda** con `ellipsis` (lo importante es dónde estoy, no de dónde vine).
- **Columna o panel con foco:** su título va entre corchetes (`[Álbum (15)]`) con el rol `tab_active`, y su selección con el fondo `selection` (video inverso sin colores de tema); las demás columnas muestran solo el `cursor`.

---

---

## 5. Vistas

Los mockups de esta sección muestran solo la zona de la vista (encabezado y panel son los de la sección 3). Están en Unicode; en ASCII cambian solo los glifos según las tablas 2.1 y 2.2.

### 5.1 Cola (`1`)

Ver sección 3.1. Columnas: marcadores · `pos` (posición en la cola, 1-based) · `#` (pista) · Título · Álbum · `dur`. Filas separadoras de álbum según 4.2. La tecla `o` lleva al tema actual y lo deja centrado verticalmente si se puede.

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
│▸        4 Time After Time                           She's So Unusual                  4:01       │
│── Foo Fighters - One by One (2002) ────────────────────────────────────────────── 1 tema  4:25 ──│
│         4 Times Like These                          One by One                        4:25       │
│── Muse - Absolution (2003) ────────────────────────────────────────────────────── 1 tema  3:57 ──│
│         3 Time Is Running Out                       Absolution                        3:57       │
│── Pink Floyd - The Dark Side of the Moon (1973) ───────────────────────────────── 1 tema  6:53 ──│
│         4 Time                                      The Dark Side of the Moon         6:53       │
```

- Fila 1: el texto buscado (con el cursor `_` mientras se escribe) y, a la derecha, los campos posibles con el activo entre paréntesis.
- Fila 2: `N resultados en M álbumes`. Sin resultados: `Sin resultados para "time"`.
- Resultados agrupados por tramos de álbum (4.2).
- El texto que coincide con la búsqueda va resaltado con el rol `match` (color, **video inverso y subrayado**: no solo color) dentro de Título, Álbum y la separadora.

### 5.5 Playlists (`5`)

```text
│ [Playlists (3)]            │ domingo: 5 temas  25:12                                             │
├────────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│ ▸ ≡ domingo                │ ── Pink Floyd - Wish You Were Here (1975) ──────── 2 temas  18:39 ──│
│   ≡ para correr            │     4 Wish You Were Here              Wish You Were Here     5:34   │
│   ≡ rock nacional          │     1 Shine On You Crazy Diamond      Wish You Were Here    13:05   │
│                            │ ── Radiohead - In Rainbows (2007) ───────────────── 3 temas  6:33 ──│
│                            │     3 Nude                            In Rainbows            4:15   │
│                            │     7 Reckoner                        In Rainbows            4:50   │
│                            │    10 Videotape                       In Rainbows            4:39   │
```

- Panel izquierdo: nombres con la insignia `playlist`. Panel derecho: contenido de la seleccionada, agrupado por tramos.
- Título del panel derecho: `<nombre>: <cantidad> temas  <duración>`. No se piden los contenidos de todas las playlists para mostrar cantidades en el panel izquierdo (sería un comando por playlist).
- Sin playlists: `No hay playlists guardadas. S guarda la cola como playlist.` centrado.

### 5.6 Visual (`6`)

La vista **`6` Visual** entra en el ciclo de `Tab` / `Shift+Tab` y lee la salida fifo de MPD. El panel "ahora suena" sigue abajo, igual que en las demás vistas. Tiene **seis formas** que se recorren en ciclo apretando `6` otra vez. La primera es **Barras**; lo que se describe con ella (fila de título, falta del FIFO, modo compacto) vale para las seis.

Unicode (bloques de octavos `▁▂▃▄▅▆▇█`, 8 niveles por celda):

```text
┌─ mpctui ────────────────────────────────────────────────────────────────────── ● localhost:6600 ─┐
│  1 Cola   2 Bibl.   3 Dir.   4 Búsq.   5 Playl.  [6 Visual]  7 Letra   8 Sal.                    │
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
|  1 Cola   2 Bibl.   3 Dir.   4 Búsq.   5 Playl.  [6 Visual]  7 Letra   8 Sal.                    |
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
- **Color:** cada fila con el degradado `bars` según su altura (sección 6.5); en 16 colores, tres tramos (verde, amarillo y rojo). El color solo refuerza: la altura ya transmite el nivel (principio 2). Los picos van en `peak`.
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

Con terminal chica (modo compacto, sección 7) el visualizador sigue disponible: se oculta la fila de título y los ejes, y cada forma se adapta según la tabla «Modo compacto» de abajo.

#### Cómo se cambia de forma

- **`6` desde otra vista:** entra al visualizador con la **última forma usada** en esta sesión. Al arrancar mpctui es la de `visualizer_style` (defecto `bars`, sección 8.4).
- **`6` estando en el visualizador:** pasa a la siguiente forma, en ciclo:

  **Barras → Espejo estéreo → Onda → Cascada → Vectorscopio → Medidores → Barras…**

- **Indicador:** la fila de título muestra el nombre, `● ○ ○ ○ ○ ○` (`* . . . . .` en ASCII) con la forma activa marcada, y `n/6`.
- **Aviso:** al cambiar, la línea inferior muestra `· Visualizador: Cascada (6: siguiente forma)` durante `MESSAGE_DURATION`.
- **FIFO de 1 canal:** Espejo estéreo y Vectorscopio necesitan dos canales, así que **salen del ciclo** (quedan 4 formas y el indicador muestra `n/4`). Medidores muestra una sola barra `M`.
- **Historial:** al cambiar de forma se reinicia todo lo acumulado (picos, historial de la Cascada, escala automática). No se calcula nada para las formas que no se ven.
- **Ayuda:** `6  visualizador / siguiente forma`.
- La forma elegida **no** se guarda al salir de mpctui (YAGNI); para cambiar la de arranque está `visualizer_style`.

Implementación del atajo: `BINDINGS` asocia `6` a una acción `ShowVisualizer`. `App` la interpreta: si la vista activa no es el visualizador, cambia de vista; si ya lo es, `style = style.next(channels)`. Solo el visualizador se comporta así; las demás teclas de vista no cambian.

#### Resumen de las formas

| # | Forma | Qué muestra | Datos que usa | Costo | Sirve para |
|---|---|---|---|---|---|
| 1 | **Barras** | Espectro por bandas | FFT mono | 1 FFT por cuadro | Vista general |
| 2 | **Espejo estéreo** | Espectro del canal izquierdo hacia arriba y del derecho hacia abajo | FFT por canal | 2 FFT | Comparar los dos canales |
| 3 | **Onda** | Forma de onda (osciloscopio) | Muestras crudas, mono | Sin FFT | Ver ataques, timbre y recortes |
| 4 | **Cascada** | Espectrograma: el tiempo corre hacia la izquierda y la intensidad es sombreado | FFT mono + historial | 1 FFT | Ver ritmo y estructura de la canción |
| 5 | **Vectorscopio** | Imagen estéreo (goniómetro M/S) y correlación | Pares L/R | Sin FFT | Ancho estéreo, mono, problemas de fase |
| 6 | **Medidores** | Nivel RMS y pico por canal en dBFS, y correlación | Muestras por canal | Sin FFT | Niveles reales |

#### Espejo estéreo

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

#### Onda (osciloscopio)

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
- **Unicode — braille:** cada celda es una matriz de 2 × 4 puntos (`U+2800`–`U+28FF`), o sea el doble de resolución horizontal y cuatro veces la vertical. El braille tiene ancho "neutral" en Unicode (no ambiguo), así que no sufre el problema del ancho ambiguo (sección 2.4), pero hay fuentes que no lo traen: otro caso para ASCII.
- **ASCII:** 3 sub-niveles por celda (`'` arriba, `-` medio, `.` abajo) y `|` para unir saltos verticales de más de una celda.
- **Escala fija** (`WAVE_GAIN` = 1,0): un tema bajo se ve chico a propósito. Etiquetas `+1`, `0`, `-1` a la izquierda.
- **Color:** cada columna con el degradado `wave` según su amplitud máxima; desde `CLIP_THRESHOLD` (0,98) va en `error`: **se ven los recortes** (sección 6.5).

#### Cascada (espectrograma)

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
- **Color:** el degradado `heat` según la intensidad de cada celda (sección 6.5). El sombreado ya la transmite.

#### Vectorscopio

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
- **Color:** cada celda con el degradado `heat` según cuántos puntos tiene; el marcador de la correlación según su valor (sección 6.5).

#### Medidores

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
- **Color por zona:** el degradado `volume` repartido por tramos: hasta −12 el primer tercio (verde), hasta −3 el segundo (amarillo) y arriba de −3 el último (rojo) (sección 6.5). Lo que transmite el nivel es la longitud de la barra y el número.
- **A la derecha:** RMS y pico en números (`-9.4 dB  pico -3.1`).
- **Correlación** debajo, con el mismo widget que el Vectorscopio (DRY). Con FIFO mono no se muestra.

#### Modo compacto

| Forma | Cambios en terminal chica |
|---|---|
| Barras | Barras de 1 columna, sin separación ni eje |
| Espejo estéreo | Barras de 1 columna; el eje central queda como línea sin etiquetas |
| Onda | Sin etiquetas de escala |
| Cascada | Sin ejes |
| Vectorscopio | Sin correlación ni letras |
| Medidores | Barras de 1 fila; sin escala ni correlación |

### 5.7 Letra (`7`)

La letra del tema que suena, sin dependencias de la vista Visual. Una sincronizada mantiene la línea actual centrada (`▸` a la izquierda, en `accent` y negrita); las ya cantadas van en `muted` y las siguientes se apagan con la distancia. Con las teclas de navegación se elige otra línea (queda centrada e invertida) y `Enter` reproduce desde ella; `h` vuelve a seguir la canción. Una letra sin sincronizar se recorre con las mismas teclas. En la fila de título: `Letra: <tema> — <artista>` y, a la derecha, `sincronizada · caché` (o las teclas sugeridas si hay una línea elegida). Mientras busca, o si no hay letra, lo dice en el centro; `Ctrl+r` reintenta una búsqueda que falló. Con `[lyrics] enabled = false` la vista no existe.

### 5.8 Salidas (`8`)

Particiones a la izquierda y salidas de la elegida a la derecha, con foco por columna (`l`/`h`). `◆` marca la partición que usa mpctui (`rol accent`); `[x]`/`[ ]` (`●`/`○`) las salidas activas e inactivas. `Enter` usa la partición o activa/desactiva la salida, `n` crea una partición, `m` mueve la salida a otra y `d` borra la partición (confirma). Con una partición distinta de `default`, el encabezado agrega `· cocina` junto al servidor.

### 5.9 Ayuda (`?`)

Superpuesta y centrada (limpiar el fondo con `Clear`). Dos columnas agrupadas por acción. **Debe entrar en 30 filas** (lección aprendida de `AGENTS.md`) y sale de `help_sections()` / `BINDINGS`: el mockup es ilustrativo.

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

### 5.10 Diálogos

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

Las confirmaciones (siempre de acciones destructivas) llevan el borde y el título en `warning`; el texto libre y los selectores, en `accent` (sección 6.5).

---

## 6. Color

Cada tipo de dato tiene su propio color (artista, álbum, duración, estado, calidad del audio, niveles), hay cuatro temas y la cantidad de colores se adapta a la terminal.

### 6.1 Principios

1. **Color con significado, no decoración.** Cada color es un **rol** (`artist`, `playing`, `error`…). Las vistas piden roles, nunca colores.
2. **El texto común queda con el color de la terminal** (`Color::Reset`), y **el fondo no se pinta**: mpctui se integra con el esquema de la terminal. Los temas colorean lo que tiene significado.
3. **Profundidad con fallback**, igual que Unicode → ASCII: truecolor → 256 colores → 16 colores ANSI → sin color. El mismo tema se ve lo mejor posible en cada terminal.
4. **Sin color sigue funcionando.** Con `NO_COLOR` o `colors = "none"` quedan solo los modificadores (negrita, video inverso, subrayado) y los glifos. Todo lo que hoy se distingue sin color se sigue distinguiendo.
5. **Contraste mínimo** contra el fondo de referencia: 4,5:1 para los roles de texto y 3:1 para los gráficos (barras, reglas). Lo verifica un test (sección 9.4).
6. **Un único lugar con colores:** `src/tui/theme/palettes.rs` es el único archivo con valores hexadecimales o nombres ANSI, como `glyphs.rs` lo es de los glifos y `src/i18n/` de los textos.

### 6.2 Profundidad de color

#### Configuración

```toml
# ~/.config/mpctui/config.toml
theme  = "terminal"   # terminal (defecto) | midnight | vinyl | neon
colors = "auto"       # auto (defecto) | truecolor | 256 | 16 | none
```

| Variable | Ejemplo | Efecto |
|---|---|---|
| `MPCTUI_COLORS` | `256` | Fuerza la profundidad (mismos valores que `colors`) |
| `NO_COLOR` | `1` | Estándar [no-color.org](https://no-color.org): con `colors = "auto"`, cualquier valor no vacío equivale a `none`. Un `colors` explícito en el archivo o en `MPCTUI_COLORS` tiene prioridad |

Precedencia, igual que el resto: defecto < archivo < `MPCTUI_COLORS`. `NO_COLOR` solo influye en la resolución de `auto`.

#### Resolución de `auto`

Función pura `resolve_color_depth(setting, environment) -> ColorDepth` en `src/config.rs`, en este orden:

1. `NO_COLOR` no vacío → **none**.
2. `TERM` = `dumb` → **none**.
3. `COLORTERM` = `truecolor` o `24bit` → **truecolor**.
4. `TERM` contiene `256color` → **256**.
5. Cualquier otro caso (incluido `TERM=linux`) → **16**.

Fail fast: un valor desconocido en `colors`, `theme` o `MPCTUI_COLORS` no arranca y se explica con `Message::Config`, como los demás errores de configuración.

#### Qué hace cada profundidad

| Profundidad | Colores de los temas | Degradados | Selección |
|---|---|---|---|
| **truecolor** | Exactos (`Color::Rgb`) | Continuos, interpolados en OKLab (sección 6.6) | Fondo `selection` del tema |
| **256** | El índice más cercano de la paleta xterm (`Color::Indexed`), calculado una vez al arrancar | Interpolados y cuantizados a 256 | Fondo `selection` cuantizado |
| **16** | **Siempre el tema `terminal`**, sea cual sea `theme` (sección 6.4) | Escalonados: un color por tramo | Video inverso |
| **none** | Ninguno | Ninguno: solo glifos | Video inverso |

¿Por qué con 16 colores todos los temas usan `terminal`? Porque en ese modo los colores son los de la paleta de la terminal, no los del tema: "magenta" puede ser cualquier cosa. Lo honesto es una sola asignación de roles pensada para ANSI. Al arrancar con `theme = "neon"` y 16 colores, la línea inferior lo avisa una vez: `· Tema neon: requiere 256 colores, se usa terminal`.

---

### 6.3 Roles

Las vistas piden un `Role`; el tema resuelto devuelve un `Style` (color + modificadores). Los modificadores son parte del rol, así se mantienen sin color.

| Rol | Dónde se usa | Modificador |
|---|---|---|
| `text` | Texto común (títulos de temas, números importantes) | — (color de la terminal) |
| `text_strong` | Título del tema en el panel, nombre de la forma del visualizador | negrita |
| `muted` | Texto secundario: posiciones, número de pista, etiquetas, descripciones de atajos, contadores | — |
| `subtle` | Bordes, reglas, partes vacías de barras, riel de la barra de desplazamiento, `○` | — |
| `accent` | Teclas en la línea inferior y la ayuda, cursor `▸`, `mpctui`, `♪`, cursor de la barra de desplazamiento, página activa del visualizador, borde del panel con foco | negrita en teclas |
| `accent_alt` | Cabeza `●` de las barras de progreso, títulos de sección de la ayuda | negrita |
| `tab_active` | Pestaña activa, columna o panel con foco (título) | `accent` + video inverso + negrita |
| `playing` | `▶`, fila del tema que suena, regla de la separadora del álbum que suena, `▶ reproduciendo` | negrita en la fila |
| `paused` | `⏸`, `⏸ en pausa` | negrita |
| `artist` | Artista: separadoras, panel, columna Artista de la Biblioteca | negrita en separadoras |
| `album` | Álbum: separadoras, columna Álbum, panel | negrita en separadoras |
| `duration` | Columna de duración | — |
| `directory` | Directorios y su `▪` (convención de `ls`) | negrita |
| `playlist` | Playlists y su `≡`, `.m3u` | — |
| `match` | Coincidencias de búsqueda | color + video inverso + subrayado |
| `selection` | Fila seleccionada (fondo) | en 16/none: video inverso |
| `success` | `●` de conexión, modos activos, salidas activas, avisos de éxito | — |
| `warning` | `◐` (single una vez, spinner de actualización), medidor de la cuenta regresiva, borde de diálogos destructivos | — |
| `error` | `✖`, errores, recortes (Onda), correlación negativa | negrita |
| `info` | Avisos `·` en la línea inferior | — |
| `lossless` / `lossy` / `hires` | Formato de audio (sección 6.5) | negrita en el códec |
| `peak` | Picos del visualizador, pico retenido de los Medidores | — |

Y siete **degradados** (`GradientRole`): `progress`, `volume`, `bars`, `heat`, `mirror_left`, `mirror_right` y `wave` (sección 6.6).

---

### 6.4 Temas

#### `terminal` (defecto, 16 colores)

Usa los nombres ANSI, así que toma la paleta de la terminal del usuario. Es el tema por defecto y el que usan todos los demás con 16 colores.

| Rol | Color ANSI | Rol | Color ANSI |
|---|---|---|---|
| `accent` | `Cyan` | `directory` | `LightBlue` + negrita |
| `accent_alt` | `Magenta` | `playlist` | `Magenta` |
| `muted` | `DarkGray` | `match` | `Yellow` + inverso |
| `subtle` | `DarkGray` | `success` | `Green` |
| `playing` | `Green` | `warning` | `Yellow` |
| `paused` | `Yellow` | `error` | `LightRed` |
| `artist` | `LightBlue` | `info` | `Cyan` |
| `album` | `Magenta` | `lossless` / `lossy` / `hires` | `Green` / `Yellow` / `Magenta` |
| `duration` | (texto) | `peak` | `White` |

Degradados escalonados: `progress` = `Cyan`; `volume` y `bars` = `Green` → `Yellow` → `Red`; `heat` = `DarkGray` → `Blue` → `Magenta` → `Red` → `Yellow`; `mirror_left` = `Cyan`; `mirror_right` = `Magenta`; `wave` = `Cyan` → `Magenta` → `Red`.

#### `midnight` (frío)

Azul cielo, aguamarina y lavanda.

| Rol | Hex | Rol | Hex |
|---|---|---|---|
| `accent` | `#5fb3f9` | `directory` | `#5fb3f9` |
| `accent_alt` | `#b48eff` | `playlist` | `#c9a7f5` |
| `muted` | `#8a93a8` | `match` | `#ffd166` |
| `subtle` | `#3d4456` | `selection` | `#252c42` |
| `playing` | `#5ee0a0` | `success` | `#5ee0a0` |
| `paused` | `#f2c46d` | `warning` | `#f2c46d` |
| `artist` | `#7fd3e6` | `error` | `#ff6b81` |
| `album` | `#c9a7f5` | `info` | `#7fd3e6` |
| `duration` | `#9fb4d1` | `lossless` / `lossy` / `hires` | `#5ee0a0` / `#f2c46d` / `#c9a7f5` |
| `peak` | `#e8f1ff` | | |

Degradados: `progress` `#5fb3f9 → #b48eff`; `volume` `#5ee0a0 → #f2c46d → #ff6b81`; `bars` `#2f7de1 → #5fb3f9 → #b48eff → #f0a6ff`; `heat` `#16182e → #2f3a8f → #6b4fd6 → #5fb3f9 → #e8f6ff`; `mirror_left` `#2f7de1 → #5fb3f9`; `mirror_right` `#7b5bd6 → #c9a7f5`; `wave` `#5fb3f9 → #b48eff → #ff6b81`.

#### `vinyl` (cálido)

Ámbar, naranja y el verde de los vúmetros de un equipo hi-fi.

| Rol | Hex | Rol | Hex |
|---|---|---|---|
| `accent` | `#ffb454` | `directory` | `#ffb454` |
| `accent_alt` | `#ff7a45` | `playlist` | `#f29e74` |
| `muted` | `#a8987f` | `match` | `#ffd580` |
| `subtle` | `#4a4038` | `selection` | `#3a2e24` |
| `playing` | `#b8cc52` | `success` | `#b8cc52` |
| `paused` | `#ffd580` | `warning` | `#ffd580` |
| `artist` | `#e8b98a` | `error` | `#f0544c` |
| `album` | `#f29e74` | `info` | `#8ec0a8` |
| `duration` | `#b8a58c` | `lossless` / `lossy` / `hires` | `#b8cc52` / `#ffb454` / `#ff8fb1` |
| `peak` | `#fff1d6` | | |

Degradados: `progress` `#ffb454 → #ff7a45`; `volume` `#b8cc52 → #ffb454 → #f0544c`; `bars` `#8a9a3a → #b8cc52 → #ffb454 → #f0544c`; `heat` `#1a1410 → #5a1f1a → #c2421f → #ffb454 → #fff1c4`; `mirror_left` `#c98a2e → #ffb454`; `mirror_right` `#c0512b → #ff7a45`; `wave` `#ffb454 → #ff7a45 → #f0544c`.

#### `neon` (synthwave)

Magenta y cian eléctricos sobre violeta.

| Rol | Hex | Rol | Hex |
|---|---|---|---|
| `accent` | `#ff4fd8` | `directory` | `#29e7ff` |
| `accent_alt` | `#29e7ff` | `playlist` | `#ff4fd8` |
| `muted` | `#9a90c2` | `match` | `#ffe156` |
| `subtle` | `#3d335c` | `selection` | `#3a1f4d` |
| `playing` | `#29e7ff` | `success` | `#3dffa8` |
| `paused` | `#ffe156` | `warning` | `#ffe156` |
| `artist` | `#a99bff` | `error` | `#ff3b6b` |
| `album` | `#ff8fd8` | `info` | `#29e7ff` |
| `duration` | `#b3a8e0` | `lossless` / `lossy` / `hires` | `#3dffa8` / `#ffe156` / `#ff4fd8` |
| `peak` | `#ffffff` | | |

Degradados: `progress` `#ff4fd8 → #29e7ff`; `volume` `#3dffa8 → #ffe156 → #ff3b6b`; `bars` `#6a2cff → #ff4fd8 → #ff9ad5 → #29e7ff`; `heat` `#0d0221 → #3b0f70 → #b1207c → #ff4fd8 → #29e7ff`; `mirror_left` `#b1207c → #ff4fd8`; `mirror_right` `#0f8fb0 → #29e7ff`; `wave` `#29e7ff → #ff4fd8 → #ff3b6b`.

#### Fondo de referencia

Los temas truecolor están pensados para terminales **oscuras**. El contraste se mide contra `REFERENCE_BACKGROUND = #1e1e24`: todos los roles de texto de los tres temas superan 4,5:1 (el más bajo es 4,8:1). En una terminal clara conviene `terminal`, que se adapta solo. Pintar el fondo del tema o tener temas claros queda fuera de alcance (sección 10).

---

### 6.5 Dónde va el color

#### Encabezado

| Elemento | Rol |
|---|---|
| `mpctui` | `accent` + negrita |
| `●` conectado / `✖` sin conexión | `success` / `error` |
| Host, `queue: 135 songs`, reloj | `muted`; los números, `text` |
| Medidor de la cuenta regresiva, `◐ actualizando` | `warning` |
| Nombre de la partición en uso | `accent_alt` |
| Pestaña activa | `tab_active` (en truecolor: fondo `accent`, texto oscuro, negrita) |
| Pestañas inactivas | `muted` |

#### Listas de temas (Cola, Búsqueda, Playlists)

Separadora de álbum, coloreada por partes:

```text
── King Crimson - In the Court of the Crimson King (1969) ─────────── 5 songs  43:52 ──
   └ artist ───┘   └ album ──────────────────────┘ └muted┘└ subtle ┘ └ muted ──────┘
```

- La **regla** (`──`) de la separadora del **álbum que suena** usa `playing` en lugar de `subtle`: se ve de un vistazo qué grupo está sonando, aunque el tema quede fuera de pantalla.
- Columnas de cada fila: `pos` y `#` en `muted`, título en `text`, álbum en `album`, duración en `duration`.
- **Tema que suena:** toda la fila en `playing` + negrita, con `▶`/`⏸` (`paused` cuando está en pausa).
- **Selección:** fondo `selection` en truecolor y 256, video inverso en 16 y sin color. El `▸` del cursor va en `accent` con video inverso, para que se vea aunque la fila tenga fondo.
- **Barra de desplazamiento:** cursor en `accent`, riel en `subtle`, flechas en `muted`.

#### Otras vistas

| Vista | Elemento | Rol |
|---|---|---|
| Biblioteca | Columna Artista / Álbum | `artist` / `album` |
| | Duración relativa `■■■□□□` | celdas llenas con el degradado `progress` por posición; vacías en `subtle` |
| | Título de la columna con foco | `tab_active` |
| | Migas de pan | separadores `›` en `subtle`, el último tramo en `text` + negrita, el resto `muted` |
| Directorios | `▪ Carpeta/` | `directory` (glifo y nombre) |
| | `♪ archivo` | glifo `accent`, nombre `text`, duración `duration` |
| | `≡ lista.m3u` | `playlist` |
| Búsqueda | Texto buscado | `accent` |
| | Campo activo `(all)` | `accent` + negrita; los demás `muted` |
| | Coincidencias | `match` |
| Playlists | Nombres y `≡` | `playlist` |
| Letra | Línea actual | `accent` + negrita, con `▸` |
| | Líneas siguientes | se apagan con la distancia: en truecolor y 256, mezcla de `text` a `muted` en `LYRICS_FADE_LINES` (6) líneas; en 16, `text` las 2 siguientes y `muted` el resto |
| | Líneas ya cantadas | `muted` |
| Salidas | `●` activa / `○` inactiva | `success` / `subtle` |
| | Plugin | `muted` |
| | `◆` partición en uso | `accent` |
| Ayuda | Títulos de sección | `accent_alt` + negrita |
| | Teclas / descripciones | `accent` + negrita / `text` |
| | Borde | `accent` (es la ventana con foco) |
| Diálogos | Borde y título de confirmaciones destructivas (vaciar cola, borrar) | `warning` |
| | Borde de texto libre (guardar playlist, nueva partición) | `accent` |
| | Error de validación | `error` |
| Todas | Borde del panel con foco (Biblioteca, Playlists, Salidas) | `accent`; los demás `subtle` |
| Línea inferior | Teclas / descripciones | `accent` + negrita / `muted` |
| | Avisos / errores | `info` / `error` |

#### Panel "ahora suena"

- **Título del borde:** `▶ reproduciendo` en `playing`, `⏸ en pausa` en `paused`, `■ detenido` en `muted`.
- **Formato de audio** según la calidad:

| Caso | Regla | Color |
|---|---|---|
| Sin pérdida | Códec `FLAC`, `ALAC`, `WAV`, `AIFF`, `APE`, `WV`, `DSF`, `DFF` | Códec en `lossless` + negrita |
| Con pérdida | Códec `MP3`, `OGG`, `OPUS`, `AAC`, `MPC`, `WMA` | Códec en `lossy` + negrita |
| Desconocido | Sin extensión, `M4A` (puede ser AAC o ALAC) u otra | Códec en `muted` |
| Alta resolución | Códec **sin pérdida** y (frecuencia > 48 kHz, o ≥ 24 bit, o flotante, o DSD) | Frecuencia y bits en `hires` |
| Resto | | `muted` |

  > **Ojo:** MPD decodifica MP3 a 24 bit (en las capturas: `MP3  44.1 kHz  24 bit`). Por eso "alta resolución" exige un códec sin pérdida: si no, todo MP3 se marcaría como hi-res.

- **Título del tema:** `♪` en `accent`, título en `text_strong`, la línea de la letra en `muted`, artista en `artist` y álbum en `album`.
- **Barras de progreso (tema, álbum, cola):**
  - Celdas llenas con el degradado `progress` **según su posición en la barra** (no según el avance): el color dice dónde está cada celda, así la barra no cambia de color al avanzar.
  - Cabeza `●` en `accent_alt` + negrita, vacías en `subtle`.
  - Separadores `┃` en `text` + negrita, para que el cambio de tema o de álbum resalte.
  - Tiempos en `text`; `track 3/5` y similares en `muted`.
- **Volumen:** cada celda llena con el degradado `volume` según su posición (verde → amarillo → rojo hacia el 100 %); vacías en `subtle`.
- **Modos:** tecla en `accent` + negrita; `●` en `success`, `○` en `subtle`, `◐` en `warning`; el nombre en `text` si está activo y en `muted` si no.

#### Visualizador

| Forma | Color |
|---|---|
| Fila de título | Nombre de la forma en `text_strong`; página activa `●` en `accent`, las demás `○` en `subtle`; datos de la derecha en `muted` |
| **Barras** | Cada fila con el degradado `bars` según su altura (`t = (fila desde abajo + 0,5) / alto`); en 16 colores, tres tramos. Picos en `peak` |
| **Espejo estéreo** | Mitad L con `mirror_left` y mitad R con `mirror_right`, ambos según la distancia al eje; etiquetas del eje en `muted` |
| **Onda** | Cada columna con el degradado `wave` según su amplitud máxima (\|x\|); una columna con \|x\| ≥ `CLIP_THRESHOLD` (0,98) va en `error`: **se ven los recortes**. Etiquetas `+1 0 -1` en `muted` |
| **Cascada** | El sombreado (`░▒▓█` o `.:-=+*#%@`) se mantiene y además cada celda se pinta con el degradado `heat` según su intensidad. Ejes en `muted` |
| **Vectorscopio** | Cada celda braille con el degradado `heat` según cuántos puntos tiene; guías L/R en `subtle`; letras M, L, R en `muted` |
| **Medidores** | Celdas por zona con el degradado `volume` según su posición en la escala (−60 → 0 dBFS); pico retenido en `peak`; marcas de la escala en `subtle`, números en `muted` |
| Correlación (Vectorscopio y Medidores) | Línea en `subtle`; marcador `●` en `error` si < 0, `warning` entre 0 y `CORRELATION_WIDE` (0,3), `success` arriba |

---

### 6.6 Degradados

- Un degradado es una lista de 1 a 5 colores (paradas) repartidos de forma pareja entre 0 y 1.
- **Truecolor:** interpolación lineal **en OKLab**, no en RGB: en RGB la mezcla de azul y amarillo pasa por gris y se ve sucia. La conversión sRGB ↔ OKLab son unas 30 líneas puras y testeables en `gradient.rs`.
- **256:** se interpola igual y el resultado se cuantiza al índice xterm más cercano.
- **16:** sin interpolación: `t` elige la parada `floor(t × paradas)` (por eso en `terminal` las barras quedan en tramos verde, amarillo y rojo).
- **none:** el degradado devuelve `Style::default()`.
- **Rendimiento:** el visualizador pide colores 20 veces por segundo para cientos de celdas. Cada degradado se **precalcula** al resolver el tema en una tabla de `GRADIENT_STEPS` (64) colores, y `at(t)` solo indexa.

---

### 6.7 Cambiar de tema en vivo (`t`)

- `t` pasa al tema siguiente en ciclo: `terminal → midnight → vinyl → neon → terminal`. La línea inferior muestra `· Tema: vinyl (t: siguiente)`.
- Con 16 colores o sin color, `t` no cambia nada y avisa: `· Los temas requieren 256 colores`.
- Igual que la forma del visualizador, **no se guarda al salir** (YAGNI); el de arranque se elige con `theme`.
- Se agrega solo en `BINDINGS` (`t` → `Action::NextTheme`), así la ayuda se actualiza sola. `t` es una tecla de lectura: se agrega a la lista de teclas permitidas contra el MPD real en `AGENTS.md` sección 6.7.

---

---

## 7. Tamaño de terminal (responsivo)

| Tamaño | Modo | Cambios |
|---|---|---|
| ≥ 100 × 28 | Completo | Todo lo descrito |
| ≥ 60 × 16 | Compacto | Panel de 4 filas: solo barra del tema; modos con letras (`z● r○ y◐ R○`); pestañas abreviadas; tablas sin la columna Álbum por debajo de 80 columnas |
| Menor | Aviso | Solo un mensaje con el mínimo y el tamaño actual |

Los umbrales son constantes con nombre (`FULL_MIN_WIDTH`, `FULL_MIN_HEIGHT`, `COMPACT_MIN_WIDTH`, `COMPACT_MIN_HEIGHT`). El modo se decide en cada dibujado a partir de `frame.area()`: es una función pura `layout_mode(width, height) -> LayoutMode`.

Compacto — Unicode (72 × 16):

```text
│ [1 Cola]  2 Bibl.   3 Dir.   4 Búsq.   5 Playl.   6 Visual   7 Letra │
│ [1 Cola]  2 Bibl.  3 Dir.  4 Búsq.  5 Playl.                         │
├──────────────────────────────────────────────────────────────────────┤
│      303   3 On the Run                                       3:45   │
│  ▶   304   4 Time                                             6:53   │
│      305   5 The Great Gig in the Sky                         4:44   │
│▸     306   6 Money                                            6:22   │
│      307   7 Us and Them                                      7:49   │
│      308   8 Any Colour You Like                              3:26   │
├──────────────────────────────────────────────────────────────────────┤
│ ▶ ━━━━━━━━━━━━━━━━━●──────────────────────────────────── 2:13/6:53   │
│ ♪ Time - Pink Floyd                     vol 70%  tema 4/10  304/1229 │
│ z● r○ y◐ R○                                                          │
└──────────────────────────────────────────────────────────────────────┘
 ? ayuda
```

Compacto — ASCII:

```text
| [1 Cola]  2 Bibl.   3 Dir.   4 Búsq.   5 Playl.   6 Visual   7 Letra |
| [1 Cola]  2 Bibl.  3 Dir.  4 Búsq.  5 Playl.                         |
+----------------------------------------------------------------------+
|      303   3 On the Run                                       3:45   |
|  |>  304   4 Time                                             6:53   |
|      305   5 The Great Gig in the Sky                         4:44   |
|>     306   6 Money                                            6:22   |
|      307   7 Us and Them                                      7:49   |
|      308   8 Any Colour You Like                              3:26   |
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

## 8. Datos y configuración

YAGNI: se agrega solo lo que se muestra.

### 8.1 Datos del diseño

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

### 8.2 Datos de MPD

#### Formato de audio y bitrate

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

#### Actualización del bitrate

`bitrate` cambia cada segundo en archivos VBR, pero MPD no emite ningún evento `idle` por eso. Regla:

- Mientras el estado es *reproduciendo*, el `PlaybackService` vuelve a pedir `status` cada `STATUS_POLL_INTERVAL` (constante, 2 s), usando el tick que ya existe en el loop.
- Pausado o detenido: no se consulta nada.
- Es un comando de solo lectura, va por la conexión del propio servicio (no por la de `idle`) y de paso corrige la deriva de `PlaybackClock`.

#### Actualización de la base

- `status` incluye `updating_db: <id de trabajo>` solo mientras hay una actualización en curso. Se parsea como `Option<u32>` (estricto: un valor no numérico es `Error::Protocol`).
- `idle` informa `changed: update` cuando una actualización **empieza o termina**. Se mapea a `Subsystem::Update`. Al recibirlo, se vuelve a pedir `status` para saber cuál de las dos cosas pasó.
- `changed: database` sigue significando "la base cambió" y se procesa como hasta ahora.

Estado en la TUI: `AppState` guarda `database_update: Option<Instant>` (cuándo se vio por primera vez). El cálculo de "empezó / terminó" a partir de dos `status` sucesivos es lógica de negocio: va en un servicio, no en la vista.
#### Fuera de alcance

Disparar una actualización desde mpctui (`update`) no formaba parte de esto. El usuario lo pidió después: la tecla `u` envía `update` y el indicador sigue llegando por `idle`.

#### Cuenta regresiva de reconexión

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
#### Fuera de alcance

"Reintentar ahora" con una tecla sería útil con esperas de 30 s, pero es un atajo nuevo: se agrega solo si el usuario lo pide.

### 8.3 Lectura del FIFO y análisis

- MPD escribe PCM crudo, entrelazado, con signo, en el **orden de bytes nativo** (`i16::from_ne_bytes`), en el formato de la salida fifo de `mpd.conf`.
- Se abre en modo no bloqueante con `tokio::net::unix::pipe::OpenOptions::new().open_receiver(ruta)`: abrir para lectura no bloquea aunque MPD todavía no esté escribiendo.
- Las lecturas pueden cortar un frame por la mitad (4 bytes en 16 bit estéreo): los bytes sobrantes se guardan para la lectura siguiente.
- Se guardan los últimos `ANALYSIS_FRAMES` (2048) frames **estéreo** en un búfer circular (`StereoWindow`). Las formas que trabajan en mono lo derivan de ahí; las que necesitan los dos canales (Espejo, Vectorscopio, Medidores) los usan por separado.
- Al abrir o al volver de una pausa, lo que haya acumulado en el pipe es viejo: se lee todo y se descarta todo menos lo último.
- **Fin de archivo (0 bytes):** MPD cerró su lado (se detuvo o se reinició). Se reabre usando el `Backoff` que ya existe en `src/service/` (DRY), sin mostrar error: la vista pasa a "sin señal".
- **Solo lectura:** leer el FIFO no cambia nada en MPD, así que **sí** se puede probar contra el servidor real (sección 9.4).

#### Análisis de la forma Barras

Pasos por cuadro, todos en funciones puras. Los pasos 1 a 6 forman el **analizador de bandas** que también usan Espejo (una vez por canal) y Cascada (DRY):

1. **Mono:** promedio de los canales, normalizado a `f32` en [-1, 1].
2. **Ventana de Hann** sobre los últimos `ANALYSIS_FRAMES` (2048) frames: unos 46 ms a 44,1 kHz, con resolución de unos 21,5 Hz por bin.
3. **FFT real** con el crate `rustfft` (dependencia nueva, justificada: implementar una FFT a mano va contra KISS).
4. **Magnitud por bin:** `|X| × 2 / suma_de_la_ventana`.
5. **Bandas logarítmicas** entre `SPECTRUM_MIN_HZ` (40) y `SPECTRUM_MAX_HZ` (16000): `borde_i = min × (max/min)^(i/bandas)`. Cada banda toma el **máximo** de los bins que caen en su rango; si una banda grave no contiene ningún bin, toma el bin más cercano a su centro.
6. **Decibeles y normalización:** `dB = 20·log10(magnitud + ε)`, `nivel = clamp((dB − FLOOR_DB) / (CEIL_DB − FLOOR_DB), 0, 1)` con `FLOOR_DB = -60` y `CEIL_DB = 0`. Son constantes para ajustar con música real usando el ejemplo `spectrum_smoke`.
7. **Suavizado:** si el nivel nuevo es mayor, se toma enseguida; si es menor, baja como mucho `FALL_PER_SECOND` (1,5 alturas completas por segundo) × tiempo transcurrido. Los picos, igual, con su propia caída.

### 8.4 Configuración

Todas las opciones están documentadas, con sus valores por defecto, en `config.toml.dist`. Precedencia: valor por defecto < `~/.config/mpctui/config.toml` < variables de entorno. Un nombre desconocido o un valor inválido no deja arrancar y se explica en el idioma configurado.

| Opción | Entorno | Valores | Efecto |
|---|---|---|---|
| `glyphs` | `MPCTUI_GLYPHS` | `auto`, `unicode`, `ascii` | Juego de glifos (sección 2.3) |
| `theme` | — | `terminal`, `midnight`, `vinyl`, `neon` | Tema de colores (sección 6.4) |
| `colors` | `MPCTUI_COLORS`, `NO_COLOR` | `auto`, `truecolor`, `256`, `16`, `none` | Profundidad de color (sección 6.2) |
| `language` | — | `en`, `es` | Idioma de la interfaz |
| `album_progress`, `queue_progress` | — | `true`/`false` | Barras opcionales del panel |
| `visualizer_fifo` | — | ruta | Salida fifo de MPD (defecto `/tmp/mpd.fifo`) |
| `visualizer_format` | — | `frecuencia:16:canales` | Debe coincidir con el `format` de la salida fifo |
| `visualizer_style` | — | `bars`, `mirror`, `wave`, `waterfall`, `vectorscope`, `meters` | Forma inicial |

- `visualizer_format` se valida al cargar (`PcmFormat`): solo 16 bits, 1 o 2 canales y entre 8000 y 192000 Hz. Con un FIFO de 1 canal, elegir `mirror` o `vectorscope` es error de configuración.
- `visualizer_fifo` vacío es un error. Un FIFO inexistente **no** lo es: se resuelve en tiempo de ejecución con la pantalla de ayuda de la sección 5.6, porque MPD puede crearlo después.

---

## 9. Implementación

### 9.1 Módulos

| Archivo | Responsabilidad |
|---|---|
| `src/tui/glyphs.rs` | `Glyphs`, `UNICODE` y `ASCII`. Único lugar con glifos literales |
| `src/tui/theme/` | `Theme` resuelto (`style(Role)`, `gradient(GradientRole, t)`); `roles.rs`, `spec.rs`, `palettes.rs` (**único lugar con colores**), `gradient.rs` (OKLab y tabla precalculada), `xterm256.rs` |
| `src/tui/appearance.rs` | `Appearance`: glifos, idioma, tema, barras opcionales, forma inicial; lo arma la factory |
| `src/tui/views/chrome.rs` | `Context` (glifos, textos, tema, modo, ancho, instante), bloques, reglas y divisores |
| `src/tui/views/gauges.rs` | Barra segmentada y medidores (`BarCell`, `MeterCell`) y su traducción a estilos |
| `src/tui/views/widgets.rs` | `render_list` + `RowSource`, tabla agrupada, cabecera fija |
| `src/tui/views/now_playing.rs` | Panel "ahora suena" |
| `src/tui/views/layout.rs` | `layout_mode(width, height)` |
| `src/tui/views/format.rs` | Duraciones, códec y calidad del audio, truncado por ancho, resaltado |
| `src/tui/views/pages/visual/` | Un archivo por forma más `columns.rs`, `canvas.rs`, `correlation.rs` |
| `src/tui/app/colors.rs` | `t`: tema siguiente en vivo |
| `src/domain/` | `timeline.rs`, `visual.rs`, `inputs.rs` (`PcmFormat`), `audio.rs` |
| `src/service/visual/`, `visualizer.rs` | Análisis puro y tarea del visualizador |
| `src/repository/fifo.rs` | `FifoPcmSource` |
| `src/config.rs` | `GlyphMode`, `ThemeId`, `ColorDepth`, `resolve_glyph_mode`, `resolve_color_depth` |
| `src/factory.rs` | Resuelve glifos y `Theme` una vez y los inyecta |

Las vistas reciben el `Context` desde `App`: ninguna vista elige juego de glifos, tema ni profundidad, ni lee el entorno.

### 9.2 Detalles de ratatui que rompen el fallback si se olvidan

- **Bordes:** `Block::bordered().border_set(glyphs.border)`. Para ASCII, un `border::Set` propio con `+`, `-`, `|`. El de Unicode es `border::PLAIN`.
- **`Tabs`:** el divisor por defecto es `│`. Poner `.divider(" ")` o un glifo de `Glyphs`.
- **`Scrollbar`:** setear explícitamente `begin_symbol`, `end_symbol`, `track_symbol` y `thumb_symbol` desde `Glyphs` (los de fábrica son Unicode).
- **No usar `Gauge`, `LineGauge`, `Sparkline` ni `BarChart`:** dibujan con bloques y líneas Unicode. Las barras se arman con `Paragraph` + la `Line` de `gauges.rs`.
- **`highlight_symbol`** de `List`/`Table`: `glyphs.cursor` seguido de un espacio, con `HighlightSpacing::Always` para que las columnas no se corran al seleccionar.
- **Tablas con separadoras:** ratatui no tiene celdas que ocupen varias columnas. `GroupedSongTable` dibuja solo las filas visibles como `Line`s, con columnas calculadas con `Layout`. Cumple además la lección de no clonar listas largas al dibujar.
- **Truncado:** siempre por ancho visible con `glyphs.ellipsis`, nunca cortando bytes (panic con UTF-8) ni con `…` literal.

### 9.3 Arquitectura del visualizador

Respeta las capas de `AGENTS.md`: el FIFO es una **fuente de datos**, así que va en `repository`. El análisis es lógica, así que va en `service`. La vista solo dibuja.

| Capa | Archivo | Contenido |
|---|---|---|
| domain | `src/domain/inputs.rs` | `PcmFormat` validado (frecuencia, 16 bit, canales) |
| domain | `src/domain/visual.rs` | `VisualizerStyle` (sección 9.3) y `VisualFrame`, un cuadro listo para dibujar |
| repository | `src/repository/traits.rs` | Trait angosto `PcmSource { fn read(&mut self, destination: &mut [u8]) -> impl Future<Output = Result<usize>> + Send; }` |
| repository | `src/repository/fifo.rs` | `FifoPcmSource`, abre y lee el pipe. **No** va en `mpd/`: no es el protocolo de MPD |
| service | `src/service/visual/` | Funciones puras: decodificar PCM, `StereoWindow` y un módulo por forma (sección 9.3) |
| service | `src/service/visualizer.rs` | `Visualizer<S: PcmSource>`: tarea que lee, analiza a `VISUALIZER_FPS` (20) y publica el último `VisualFrame` |
| tui | `src/tui/views/visual/` | Un archivo por forma, más los dibujos compartidos (sección 9.3) |
| tui | `src/tui/glyphs.rs` | Campos nuevos (sección 2.2) |
| factory | `src/factory.rs` | Arma `FifoPcmSource` + `Visualizer` con la config y lo inyecta |

Decisiones de concurrencia:

- El `Visualizer` publica en un **`tokio::sync::watch`**: solo importa el último cuadro, no hay cola que se acumule si la TUI va más lenta.
- La TUI le avisa al `Visualizer` qué forma está activa y de qué tamaño es el área por **otro `watch`** (`VisualRequest { style, width, height }`), porque la cantidad de bandas, de columnas y de puntos depende de eso. El servicio calcula **solo** lo que necesita la forma activa.
- La tarea **solo corre mientras la vista 6 está activa.** Al entrar se lanza; al salir se cancela (`JoinHandle::abort` o un `CancellationToken`). Fuera de esa vista no se lee el FIFO ni se gasta CPU.
- El loop `select!` de `App` agrega una rama para el `watch` y redibuja al recibir un cuadro. Las demás vistas no cambian su frecuencia de dibujado.
- `6` y la vista nueva se agregan **solo en la tabla `BINDINGS`**: la ayuda y la línea inferior se actualizan solas.

---
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

### 9.4 Tests

1. **Barras y listas:** unitarios de `gauges.rs` (sobre `BarCell`), de `timeline()` y de las separadoras (el resumen nunca se trunca).
2. **Glifos:** todo campo de `ASCII` es ASCII; en modo ASCII ningún render contiene U+2190–U+2BFF ni `…` (las letras con tilde están permitidas).
3. **Configuración:** `resolve_glyph_mode` y `resolve_color_depth` (`NO_COLOR`, `COLORTERM`, `TERM`), valores inválidos, `config.toml.dist` válido.
4. **Color:** contraste de cada tema (4,5:1 texto, 3:1 gráficos) contra `REFERENCE_BACKGROUND`; degradados (extremos, punto medio en OKLab, tabla = interpolación, escalonado con 16 colores); `nearest_index` de colores conocidos; con 16 colores el buffer solo tiene colores con nombre; sin color solo quedan modificadores; el texto no cambia entre temas y profundidades.
5. **Formato de audio:** parseo de `44100:16:2`, `48000:f:2`, `dsd64:2`; calidad (un MP3 de 24 bit no es alta resolución).
6. **Visualizador:** análisis de bandas, ciclo de formas, cada forma en puro; `FifoPcmSource` contra un FIFO real; render de las seis formas en ambos juegos y en compacto.
7. **Render con `TestBackend`** de cada vista y superposición, en ambos juegos y a 100 × 30, 72 × 16 y 40 × 10.
8. **Manual con tmux** (solo teclas de lectura; `6` y `t` son de lectura): `tmux capture-pane -p` para el texto y `-e` para los colores.

---

## 10. Fuera de alcance (YAGNI)

- Pintar el fondo con el color del tema y temas para terminales claras.
- Temas definidos por el usuario y paletas configurables para el visualizador; guardar el tema o la forma elegidos al salir.
- Glifos configurables uno por uno.
- Colores por género, año o artista; carátulas (sixel, kitty).
- Tecla para "reintentar ahora" la conexión; rotación automática de formas; persistencia tipo fósforo en Onda y Vectorscopio; formas radiales.
- Detectar la salida fifo automáticamente con `outputs`.

El avance por fases está en `PLAN.md`.

