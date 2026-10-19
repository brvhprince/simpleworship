//! What a saved schedule carries with it, so it looks the same on another
//! computer: copies of the themes its items use, and (optionally) the media
//! files they show.
//!
//! Saving gathers both from the library. Opening adds whatever this
//! computer's library is missing, matched by id, and leaves everything it
//! already has alone.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use sw_core::{ItemContent, MediaItem, MediaRef, Schedule, ThemeLibrary, ThemeRecord};
use sw_db::Database;
use sw_files::{MediaSource, ScheduleContents, ScheduleReader};

use crate::error::AppResult;
use crate::media;
use crate::state::MediaFolders;

/// Collects the themes and media a schedule uses.
pub fn gather(
    schedule: &Schedule,
    library: &ThemeLibrary,
    database: &Database,
) -> AppResult<ScheduleContents> {
    let mut themes: HashMap<_, ThemeRecord> = HashMap::new();
    let mut media_refs: Vec<MediaRef> = Vec::new();

    for item in &schedule.items {
        // The item's own theme (the library version if there is one), and
        // a song's own theme.
        let item_theme = item.look.theme.as_ref().map(|copy| {
            library
                .get(copy.id)
                .cloned()
                .unwrap_or_else(|| copy.clone())
        });
        let song_theme = match &item.content {
            ItemContent::Song { song } => song.theme_id.and_then(|id| library.get(id)).cloned(),
            _ => None,
        };
        for record in item_theme.into_iter().chain(song_theme) {
            media_refs.extend(record.theme.background.media().cloned());
            themes.insert(record.id, record);
        }

        if let Some(background) = &item.look.background {
            media_refs.extend(background.media().cloned());
        }
        if let ItemContent::Media { media } = &item.content {
            media_refs.push(media.as_ref());
        }
    }

    let mut media = Vec::new();
    for reference in media_refs {
        let known = media.iter().any(|item: &MediaItem| item.id == reference.id);
        if !known && let Some(item) = database.media().get(reference.id)? {
            media.push(item);
        }
    }

    Ok(ScheduleContents {
        schedule: schedule.clone(),
        themes: themes.into_values().collect(),
        media,
    })
}

/// Writes a `.sws` file, embedding the media files when `include_media`.
/// The file is written next to the target first and then renamed, so a
/// failed save never leaves a broken schedule behind.
pub fn write(
    target: &Path,
    contents: &ScheduleContents,
    folders: &MediaFolders,
    include_media: bool,
) -> AppResult<()> {
    let mut sources = Vec::new();
    if include_media {
        for item in &contents.media {
            let file = File::open(media::file_path(folders, &item.as_ref()))?;
            sources.push(MediaSource {
                file: item.file.clone(),
                data: Box::new(BufReader::new(file)),
            });
        }
    }

    let mut temporary = target.as_os_str().to_owned();
    temporary.push(".saving");
    let temporary = Path::new(&temporary);

    let written = File::create(temporary)
        .map_err(Into::into)
        .and_then(|file| {
            let writer = sw_files::write_schedule_to(BufWriter::new(file), contents, sources)?;
            writer
                .into_inner()
                .map_err(|error| error.into_error())?
                .sync_all()?;
            Ok(())
        })
        .and_then(|()| Ok(std::fs::rename(temporary, target)?));
    if written.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    written
}

/// Reads a `.sws` file. Themes and media the library lacks are added to it.
/// Returns the schedule, and whether any themes were added (so the engine
/// needs the new library).
pub fn read(
    path: &Path,
    database: &Database,
    folders: &MediaFolders,
) -> AppResult<(Schedule, bool)> {
    let mut reader = ScheduleReader::new(BufReader::new(File::open(path)?))?;
    let contents = reader.contents()?;

    let library = database.themes().library()?;
    let mut themes_added = false;
    for record in &contents.themes {
        if library.get(record.id).is_none() {
            database.themes().save(record)?;
            themes_added = true;
        }
    }

    let embedded = reader.media_files();
    for item in &contents.media {
        let missing = database.media().get(item.id)?.is_none();
        if missing && embedded.contains(&item.file) {
            let destination = folders.media.join(&item.file);
            let mut out = BufWriter::new(File::create(&destination)?);
            reader.copy_media(&item.file, &mut out)?;
            database.media().save(item)?;
        }
    }

    Ok((contents.schedule, themes_added))
}
