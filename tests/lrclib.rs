//! `LrclibProvider` contra un servidor HTTP local que responde como LRCLIB.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use mpctui::domain::{FetchedLyrics, TrackIdentity};
use mpctui::repository::{Error, LrclibProvider, LyricsProvider};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

const LOOPBACK: &str = "127.0.0.1";

/// Respuesta del servidor: código HTTP y cuerpo JSON.
type Reply = (u16, &'static str);

/// Atiende cada pedido con `respond(ruta con la consulta)` y anota las rutas pedidas.
async fn fake_lrclib(
    respond: impl Fn(&str) -> Reply + Send + 'static,
) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&requests);
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let (reader, mut writer) = stream.into_split();
            let mut lines = BufReader::new(reader).lines();
            let request_line = lines.next_line().await.unwrap().unwrap_or_default();
            while let Some(header) = lines.next_line().await.unwrap() {
                if header.is_empty() {
                    break;
                }
            }
            let target = request_line
                .split(' ')
                .nth(1)
                .unwrap_or_default()
                .to_owned();
            let (status, body) = respond(&target);
            log.lock().unwrap().push(target);
            let response = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            writer.write_all(response.as_bytes()).await.unwrap();
        }
    });
    (base_url, requests)
}

fn time() -> TrackIdentity {
    TrackIdentity::new(
        "Pink Floyd",
        "Time",
        Some("The Dark Side of the Moon"),
        Some(Duration::from_secs(413)),
    )
    .unwrap()
}

const SYNCED: &str =
    r#"{"duration":413.0,"instrumental":false,"plainLyrics":"a","syncedLyrics":"[00:01.00]a"}"#;

#[tokio::test]
async fn an_exact_match_is_used_first() {
    let (base_url, requests) = fake_lrclib(|_| (200, SYNCED)).await;
    let mut provider = LrclibProvider::new(base_url).unwrap();
    let lyrics = provider.fetch(&time()).await.unwrap();
    assert_eq!(lyrics, FetchedLyrics::Text("[00:01.00]a".into()));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("/api/get?"), "{requests:?}");
    assert!(requests[0].contains("track_name=Time"));
    assert!(
        requests[0].contains("artist_name=Pink+Floyd"),
        "{requests:?}"
    );
    assert!(requests[0].contains("duration=413"));
}

#[tokio::test]
async fn without_an_exact_match_the_search_picks_a_similar_duration() {
    let (base_url, requests) = fake_lrclib(|target| {
        if target.starts_with("/api/get") {
            (404, r#"{"code":404,"name":"TrackNotFound"}"#)
        } else {
            (
                200,
                r#"[{"duration":300.0,"instrumental":false,"plainLyrics":"otra","syncedLyrics":null},
                    {"duration":412.0,"instrumental":false,"plainLyrics":"esta","syncedLyrics":null}]"#,
            )
        }
    })
    .await;
    let mut provider = LrclibProvider::new(base_url).unwrap();
    let lyrics = provider.fetch(&time()).await.unwrap();
    assert_eq!(lyrics, FetchedLyrics::Text("esta".into()));
    let requests = requests.lock().unwrap();
    assert!(requests[1].starts_with("/api/search?"), "{requests:?}");
}

#[tokio::test]
async fn without_album_it_only_searches() {
    let (base_url, requests) = fake_lrclib(|_| (200, "[]")).await;
    let mut provider = LrclibProvider::new(base_url).unwrap();
    let track = TrackIdentity::new("Pink Floyd", "Time", None, None).unwrap();
    assert_eq!(
        provider.fetch(&track).await.unwrap(),
        FetchedLyrics::Missing
    );
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("/api/search?"));
}

#[tokio::test]
async fn server_errors_are_reported() {
    let (base_url, _) = fake_lrclib(|_| (500, "{}")).await;
    let mut provider = LrclibProvider::new(base_url).unwrap();
    match provider.fetch(&time()).await {
        Err(Error::Http(message)) => assert!(message.contains("500"), "{message}"),
        other => panic!("se esperaba un error HTTP, llegó {other:?}"),
    }
}
