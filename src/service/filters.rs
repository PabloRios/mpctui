//! Filtros de la biblioteca compartidos entre servicios.

use crate::domain::{Tag, TagFilter};

pub fn artist_filters(artist: &str) -> Vec<TagFilter> {
    vec![TagFilter::new(Tag::Artist, artist)]
}

pub fn album_filters(artist: &str, album: &str) -> Vec<TagFilter> {
    vec![
        TagFilter::new(Tag::Artist, artist),
        TagFilter::new(Tag::Album, album),
    ]
}
