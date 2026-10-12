//! LRCLIB (lrclib.net): letras gratuitas y sin clave, muchas sincronizadas. No es parte
//! del protocolo de MPD. Primero se pide la coincidencia exacta (`/api/get`, que exige
//! álbum y duración) y, si no hay, se busca (`/api/search`) y se elige el resultado con
//! una duración parecida, prefiriendo los sincronizados.

use std::time::Duration;

use serde::Deserialize;

use super::error::{Error, HttpFailure, Result};
use super::traits::LyricsProvider;
use crate::domain::{FetchedLyrics, TrackIdentity};

pub const LRCLIB_URL: &str = "https://lrclib.net";
/// Nombre del servicio en los errores.
const SERVICE_NAME: &str = "LRCLIB";
const GET_PATH: &str = "/api/get";
const SEARCH_PATH: &str = "/api/search";
/// LRCLIB pide que cada cliente se identifique.
const USER_AGENT: &str = concat!("mpctui/", env!("CARGO_PKG_VERSION"));
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Diferencia de duración aceptada entre el tema y un resultado.
const DURATION_TOLERANCE: Duration = Duration::from_secs(2);

/// Parámetros de la API.
mod parameters {
    pub const TRACK: &str = "track_name";
    pub const ARTIST: &str = "artist_name";
    pub const ALBUM: &str = "album_name";
    pub const DURATION: &str = "duration";
}

/// Un registro de LRCLIB; los campos que no se usan no se leen.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Record {
    /// En segundos.
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    plain_lyrics: Option<String>,
    #[serde(default)]
    synced_lyrics: Option<String>,
}

impl Record {
    fn synced(&self) -> Option<&str> {
        non_blank(self.synced_lyrics.as_deref())
    }

    /// La sincronizada si hay; si no, el texto.
    fn lyrics(&self) -> FetchedLyrics {
        if self.instrumental {
            return FetchedLyrics::Instrumental;
        }
        self.synced()
            .or(non_blank(self.plain_lyrics.as_deref()))
            .map_or(FetchedLyrics::Missing, |text| {
                FetchedLyrics::Text(text.to_owned())
            })
    }

    /// Sin duración en alguno de los dos, se acepta.
    fn matches_duration(&self, duration: Option<Duration>) -> bool {
        let (Some(expected), Some(seconds)) = (duration, self.duration) else {
            return true;
        };
        Duration::try_from_secs_f64(seconds)
            .is_ok_and(|found| found.abs_diff(expected) <= DURATION_TOLERANCE)
    }
}

fn non_blank(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.trim().is_empty())
}

/// Entre los resultados con una duración parecida: primero uno sincronizado y, si no,
/// cualquiera con letra (o instrumental).
fn choose(records: &[Record], duration: Option<Duration>) -> FetchedLyrics {
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|record| record.matches_duration(duration))
        .collect();
    candidates
        .iter()
        .find(|record| record.synced().is_some() && !record.instrumental)
        .or_else(|| {
            candidates
                .iter()
                .find(|record| record.lyrics() != FetchedLyrics::Missing)
        })
        .map_or(FetchedLyrics::Missing, |record| record.lyrics())
}

pub struct LrclibProvider {
    client: reqwest::Client,
    /// `https://lrclib.net`; los tests usan un servidor local.
    base_url: String,
}

impl LrclibProvider {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(http_error)?;
        Ok(Self {
            client,
            base_url: base_url.into(),
        })
    }

    /// `None` si no hay coincidencia exacta o si faltan el álbum o la duración.
    async fn exact(&self, track: &TrackIdentity) -> Result<Option<Record>> {
        let (Some(album), Some(duration)) = (track.album(), track.duration()) else {
            return Ok(None);
        };
        let query = [
            (parameters::TRACK, track.title().to_owned()),
            (parameters::ARTIST, track.artist().to_owned()),
            (parameters::ALBUM, album.to_owned()),
            (
                parameters::DURATION,
                duration.as_secs_f64().round().to_string(),
            ),
        ];
        let response = self.get(GET_PATH, &query).await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        checked(response)?
            .json()
            .await
            .map(Some)
            .map_err(http_error)
    }

    async fn search(&self, track: &TrackIdentity) -> Result<Vec<Record>> {
        let query = [
            (parameters::TRACK, track.title().to_owned()),
            (parameters::ARTIST, track.artist().to_owned()),
        ];
        let response = self.get(SEARCH_PATH, &query).await?;
        checked(response)?.json().await.map_err(http_error)
    }

    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<reqwest::Response> {
        self.client
            .get(format!("{}{path}", self.base_url))
            .query(query)
            .send()
            .await
            .map_err(http_error)
    }
}

impl LyricsProvider for LrclibProvider {
    async fn fetch(&mut self, track: &TrackIdentity) -> Result<FetchedLyrics> {
        if let Some(record) = self.exact(track).await? {
            let lyrics = record.lyrics();
            if lyrics != FetchedLyrics::Missing {
                return Ok(lyrics);
            }
        }
        Ok(choose(&self.search(track).await?, track.duration()))
    }
}

/// Una respuesta que no es de éxito es un error.
fn checked(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(Error::Http {
            service: SERVICE_NAME,
            failure: HttpFailure::Status(status.as_u16()),
        })
    }
}

fn http_error(error: reqwest::Error) -> Error {
    let failure = if error.is_timeout() {
        HttpFailure::Timeout
    } else if error.is_connect() {
        HttpFailure::Connect
    } else if error.is_decode() {
        HttpFailure::InvalidResponse
    } else {
        HttpFailure::Other(error.to_string())
    };
    Error::Http {
        service: SERVICE_NAME,
        failure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(seconds: f64, synced: Option<&str>, plain: Option<&str>) -> Record {
        Record {
            duration: Some(seconds),
            instrumental: false,
            plain_lyrics: plain.map(str::to_owned),
            synced_lyrics: synced.map(str::to_owned),
        }
    }

    const DURATION: Option<Duration> = Some(Duration::from_secs(200));

    #[test]
    fn synced_lyrics_win_over_plain_ones() {
        let records = [
            record(200.0, None, Some("texto")),
            record(201.5, Some("[00:01]x"), Some("texto")),
        ];
        assert_eq!(
            choose(&records, DURATION),
            FetchedLyrics::Text("[00:01]x".into())
        );
    }

    #[test]
    fn results_with_another_duration_are_ignored() {
        let records = [
            record(260.0, Some("[00:01]otra versión"), None),
            record(199.0, None, Some("texto")),
        ];
        assert_eq!(
            choose(&records, DURATION),
            FetchedLyrics::Text("texto".into())
        );
        assert_eq!(choose(&records[..1], DURATION), FetchedLyrics::Missing);
        assert_eq!(
            choose(&records[..1], None),
            FetchedLyrics::Text("[00:01]otra versión".into())
        );
    }

    #[test]
    fn blank_lyrics_count_as_missing_and_instrumentals_are_kept() {
        assert_eq!(
            record(200.0, Some(" "), Some("")).lyrics(),
            FetchedLyrics::Missing
        );
        let instrumental = Record {
            instrumental: true,
            ..record(200.0, None, None)
        };
        assert_eq!(
            choose(&[instrumental], DURATION),
            FetchedLyrics::Instrumental
        );
    }

    #[test]
    fn records_parse_from_the_api_json() {
        let json = r#"{"id":1,"trackName":"Time","artistName":"Pink Floyd","albumName":"x",
            "duration":413.0,"instrumental":false,"plainLyrics":"a","syncedLyrics":null}"#;
        let parsed: Record = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.duration, Some(413.0));
        assert_eq!(parsed.lyrics(), FetchedLyrics::Text("a".into()));
    }
}
