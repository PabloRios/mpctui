//! `FifoPcmSource` contra un FIFO real creado con `mkfifo` en un directorio temporal.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use mpctui::repository::{Error, FifoPcmSource, PcmSource};

/// Bytes de un seno de 16 bits estéreo, sin importar los valores exactos.
const PCM_BYTES: usize = 64 * 1024;
const READ_BUFFER_BYTES: usize = 4 * 1024;

/// Directorio temporal propio de este test, borrado al terminar.
struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("mpctui-fifo-{}-{nanos}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sine_pcm() -> Vec<u8> {
    (0..PCM_BYTES / 4)
        .flat_map(|index| {
            let sample = ((index as f32 * 0.1).sin() * 16_000.0) as i16;
            let bytes = sample.to_ne_bytes();
            [bytes, bytes].concat()
        })
        .collect()
}

#[tokio::test]
async fn reads_what_the_writer_sends_until_end_of_file() {
    let directory = TemporaryDirectory::new();
    let path = directory.0.join("mpd.fifo");
    let created = Command::new("mkfifo").arg(&path).status().unwrap();
    assert!(created.success());

    let mut source = FifoPcmSource::new(&path);
    let pcm = sine_pcm();
    let expected = pcm.clone();
    // Abrir para escribir bloquea hasta que haya un lector: va en su propio hilo.
    let writer_path = path.clone();
    let writer = std::thread::spawn(move || {
        let mut fifo = std::fs::OpenOptions::new()
            .write(true)
            .open(writer_path)
            .unwrap();
        fifo.write_all(&pcm).unwrap();
    });

    let mut received = Vec::new();
    let mut buffer = vec![0; READ_BUFFER_BYTES];
    loop {
        let count = source.read(&mut buffer).await.unwrap();
        if count == 0 {
            break;
        }
        received.extend_from_slice(&buffer[..count]);
    }
    writer.join().unwrap();
    assert_eq!(received, expected);
}

#[tokio::test]
async fn a_missing_fifo_is_an_io_error() {
    let directory = TemporaryDirectory::new();
    let mut source = FifoPcmSource::new(directory.0.join("no-existe.fifo"));
    let mut buffer = [0; READ_BUFFER_BYTES];
    match source.read(&mut buffer).await {
        Err(Error::Io(error)) => assert_eq!(error.kind(), std::io::ErrorKind::NotFound),
        other => panic!("se esperaba NotFound, llegó {other:?}"),
    }
}
