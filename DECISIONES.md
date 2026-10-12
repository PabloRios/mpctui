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
