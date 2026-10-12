//! Estado del análisis: lo que llegó del FIFO y lo acumulado por la forma activa. Solo
//! se calcula lo que necesita esa forma; al cambiar de forma se empieza de cero.

use std::time::Duration;

use tokio::time::Instant;

use super::bands::{FFT_SIZE, Smoother, SpectrumAnalyzer};
use super::levels::correlation;
use super::meters::Meters;
use super::pcm::{PcmDecoder, StereoWindow, left, mono, right};
use super::vectorscope::{VECTORSCOPE_FRAMES, Vectorscope};
use super::waterfall::Waterfall;
use super::wave::{WAVE_BUFFER_FRAMES, WAVE_FRAMES, wave};
use crate::domain::{
    PcmFormat, RMS_WINDOW, STEREO_CHANNELS, VisualContent, VisualFrame, VisualRequest,
    VisualizerStyle,
};

/// Sin datos durante este tiempo, se considera que no hay señal.
pub const SILENCE_TIMEOUT: Duration = Duration::from_millis(250);

/// Lo que acumula cada forma entre cuadros.
enum ShapeState {
    Bars(Smoother),
    Mirror { left: Smoother, right: Smoother },
    Wave,
    Waterfall(Waterfall),
    Vectorscope(Vectorscope),
    Meters(Meters),
}

impl ShapeState {
    fn new(style: VisualizerStyle) -> Self {
        match style {
            VisualizerStyle::Bars => ShapeState::Bars(Smoother::default()),
            VisualizerStyle::Mirror => ShapeState::Mirror {
                left: Smoother::default(),
                right: Smoother::default(),
            },
            VisualizerStyle::Wave => ShapeState::Wave,
            VisualizerStyle::Waterfall => ShapeState::Waterfall(Waterfall::default()),
            VisualizerStyle::Vectorscope => ShapeState::Vectorscope(Vectorscope::default()),
            VisualizerStyle::Meters => ShapeState::Meters(Meters::default()),
        }
    }

    fn style(&self) -> VisualizerStyle {
        match self {
            ShapeState::Bars(_) => VisualizerStyle::Bars,
            ShapeState::Mirror { .. } => VisualizerStyle::Mirror,
            ShapeState::Wave => VisualizerStyle::Wave,
            ShapeState::Waterfall(_) => VisualizerStyle::Waterfall,
            ShapeState::Vectorscope(_) => VisualizerStyle::Vectorscope,
            ShapeState::Meters(_) => VisualizerStyle::Meters,
        }
    }
}

pub struct Analysis {
    format: PcmFormat,
    decoder: PcmDecoder,
    window: StereoWindow,
    analyzer: SpectrumAnalyzer,
    shape: ShapeState,
    last_frame: Instant,
    last_data: Option<Instant>,
}

impl Analysis {
    pub fn new(format: PcmFormat, now: Instant) -> Self {
        Self {
            format,
            decoder: PcmDecoder::new(format),
            window: StereoWindow::new(window_frames(format)),
            analyzer: SpectrumAnalyzer::new(format.sample_rate()),
            shape: ShapeState::new(VisualizerStyle::default()),
            last_frame: now,
            last_data: None,
        }
    }

    pub fn receive(&mut self, bytes: &[u8], now: Instant) {
        self.decoder.decode(bytes, &mut self.window);
        self.last_data = Some(now);
    }

    /// Sin señal se olvida lo recibido: las barras caen con su velocidad de caída y
    /// las demás formas ven silencio.
    pub fn frame(&mut self, request: VisualRequest, now: Instant) -> VisualFrame {
        let signal = self
            .last_data
            .is_some_and(|last| now.saturating_duration_since(last) < SILENCE_TIMEOUT);
        if !signal {
            self.window.clear();
        }
        if self.shape.style() != request.style {
            self.shape = ShapeState::new(request.style);
        }
        let elapsed = now.saturating_duration_since(self.last_frame);
        self.last_frame = now;
        VisualFrame {
            content: self.content(request, elapsed),
            signal,
        }
    }

    fn content(&mut self, request: VisualRequest, elapsed: Duration) -> VisualContent {
        let analyzer = &self.analyzer;
        let window = &self.window;
        match &mut self.shape {
            ShapeState::Bars(smoother) => {
                let samples = mono(&window.latest(FFT_SIZE));
                smoother.update(&analyzer.levels(&samples, request.columns), elapsed);
                VisualContent::Bars(smoother.frame())
            }
            ShapeState::Mirror {
                left: left_smoother,
                right: right_smoother,
            } => {
                let pairs = window.latest(FFT_SIZE);
                left_smoother.update(&analyzer.levels(&left(&pairs), request.columns), elapsed);
                right_smoother.update(&analyzer.levels(&right(&pairs), request.columns), elapsed);
                VisualContent::Mirror {
                    left: left_smoother.frame(),
                    right: right_smoother.frame(),
                }
            }
            ShapeState::Wave => {
                let (columns, synced) =
                    wave(&mono(&window.latest(WAVE_BUFFER_FRAMES)), request.columns);
                VisualContent::Wave {
                    columns,
                    synced,
                    span: frames_duration(WAVE_FRAMES, self.format),
                }
            }
            ShapeState::Waterfall(waterfall) => {
                let due = waterfall.columns_due(elapsed);
                if due > 0 {
                    let levels = analyzer.levels(&mono(&window.latest(FFT_SIZE)), request.rows);
                    for _ in 0..due {
                        waterfall.push(levels.clone(), request.columns);
                    }
                }
                waterfall.keep_newest(request.columns);
                VisualContent::Waterfall {
                    columns: waterfall.columns(),
                }
            }
            ShapeState::Vectorscope(scope) => {
                let (points, correlation) =
                    scope.update(&window.latest(VECTORSCOPE_FRAMES), elapsed);
                VisualContent::Vectorscope {
                    points,
                    correlation,
                }
            }
            ShapeState::Meters(meters) => {
                let pairs = window.latest(rms_frames(self.format));
                let stereo = self.format.channels() >= STEREO_CHANNELS;
                let channels = if stereo {
                    vec![left(&pairs), right(&pairs)]
                } else {
                    vec![left(&pairs)]
                };
                VisualContent::Meters {
                    channels: meters.update(&channels, elapsed),
                    correlation: stereo.then(|| correlation(&pairs)),
                }
            }
        }
    }
}

/// Frames que cubre la ventana del RMS.
fn rms_frames(format: PcmFormat) -> usize {
    (u128::from(format.sample_rate()) * RMS_WINDOW.as_millis() / 1_000) as usize
}

/// Lo que necesita la forma que más pide.
fn window_frames(format: PcmFormat) -> usize {
    [
        FFT_SIZE,
        WAVE_BUFFER_FRAMES,
        VECTORSCOPE_FRAMES,
        rms_frames(format),
    ]
    .into_iter()
    .max()
    .unwrap_or(FFT_SIZE)
}

fn frames_duration(frames: usize, format: PcmFormat) -> Duration {
    Duration::from_secs_f64(frames as f64 / f64::from(format.sample_rate()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::BandFrame;
    use crate::service::visual::pcm::sine_pcm;

    const BANDS: usize = 16;

    fn stereo() -> PcmFormat {
        PcmFormat::parse("44100:16:2").unwrap()
    }

    fn request(style: VisualizerStyle) -> VisualRequest {
        VisualRequest {
            style,
            columns: BANDS,
            rows: BANDS,
        }
    }

    /// Un cuadro con una lectura de seno recién llegada.
    fn frame_with_sine(
        analysis: &mut Analysis,
        style: VisualizerStyle,
        now: Instant,
    ) -> VisualFrame {
        analysis.receive(&sine_pcm(1_000.0, 0.5, FFT_SIZE * 8), now);
        analysis.frame(request(style), now)
    }

    #[test]
    fn rms_window_is_three_hundred_milliseconds() {
        assert_eq!(rms_frames(stereo()), 13_230);
        assert_eq!(window_frames(stereo()), 13_230);
        assert_eq!(frames_duration(WAVE_FRAMES, stereo()).as_millis(), 23);
    }

    #[tokio::test(start_paused = true)]
    async fn every_style_computes_its_own_content() {
        let now = Instant::now();
        let mut analysis = Analysis::new(stereo(), now);
        for style in VisualizerStyle::ALL {
            let frame = frame_with_sine(&mut analysis, style, now + Duration::from_millis(100));
            assert_eq!(frame.content.style(), style);
            assert!(frame.signal);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn mirror_with_a_silent_right_channel_only_fills_the_top() {
        let now = Instant::now();
        let mut analysis = Analysis::new(stereo(), now);
        let left_only: Vec<u8> = sine_pcm(1_000.0, 0.5, FFT_SIZE * 2)
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|frame| [frame[0], frame[1], 0, 0])
            .collect();
        analysis.receive(&left_only, now);
        let frame = analysis.frame(request(VisualizerStyle::Mirror), now);
        let VisualContent::Mirror { left, right } = frame.content else {
            panic!("se esperaba el espejo");
        };
        assert!(left.levels.iter().any(|level| *level > 0.5));
        assert_eq!(
            right,
            BandFrame {
                levels: vec![0.0; BANDS],
                peaks: vec![0.0; BANDS],
            }
        );
    }

    #[tokio::test(start_paused = true)]
    async fn mono_meters_have_one_channel_and_no_correlation() {
        let now = Instant::now();
        let mut analysis = Analysis::new(PcmFormat::parse("44100:16:1").unwrap(), now);
        analysis.receive(&[0; 64], now);
        let frame = analysis.frame(request(VisualizerStyle::Meters), now);
        let VisualContent::Meters {
            channels,
            correlation,
        } = frame.content
        else {
            panic!("se esperaban los medidores");
        };
        assert_eq!(channels.len(), 1);
        assert_eq!(correlation, None);
    }

    #[tokio::test(start_paused = true)]
    async fn without_signal_the_waterfall_keeps_moving_with_empty_columns() {
        let start = Instant::now();
        let mut analysis = Analysis::new(stereo(), start);
        let style = VisualizerStyle::Waterfall;
        frame_with_sine(&mut analysis, style, start + Duration::from_millis(100));
        let later = start + Duration::from_secs(1);
        let frame = analysis.frame(request(style), later);
        assert!(!frame.signal);
        let VisualContent::Waterfall { columns } = frame.content else {
            panic!("se esperaba la cascada");
        };
        assert_eq!(columns.len(), BANDS.min(10));
        assert!(columns[0].iter().any(|level| *level > 0.0));
        assert!(columns.last().unwrap().iter().all(|level| *level == 0.0));
    }
}
