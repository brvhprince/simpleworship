//! Theme packs (`.swpack`, see `sw_files::pack_file`): exporting themes and
//! lower thirds, with the pictures and videos behind them, and importing
//! them on another computer.
//!
//! Importing only adds: whatever this computer already has (matched by id)
//! is left alone, so importing the same pack twice changes nothing and
//! never overwrites the operator's own edits.

use std::collections::{BTreeSet, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use serde::Serialize;
use sw_core::lower_thirds::LowerThird;
use sw_core::{Id, MediaItem, ThemeRecord};
use sw_db::Database;
use sw_files::{PackContents, PackMedia, PackReader};
use ts_rs::TS;

use crate::error::AppResult;
use crate::media;
use crate::state::MediaFolders;

/// What an import added, for the message the operator sees.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct PackImported {
    /// Themes added to the library.
    pub themes_added: u32,
    /// Lower thirds added to the list.
    pub lower_thirds_added: u32,
    /// Themes and lower thirds left out because this computer already has
    /// them.
    pub already_here: u32,
    /// Fonts the pack's themes or lower thirds use that this computer does
    /// not have: they show in a stand-in font until installed.
    pub missing_fonts: Vec<String>,
}

/// Collects the themes and lower thirds with the given ids (in the
/// library's order), and the media their backgrounds use. Starter themes
/// travel as ordinary themes.
pub fn gather(
    database: &Database,
    theme_ids: &[Id],
    lower_third_ids: &[Id],
) -> AppResult<PackContents> {
    let themes: Vec<ThemeRecord> = database
        .themes()
        .library()?
        .themes
        .into_iter()
        .filter(|record| theme_ids.contains(&record.id))
        .map(|record| ThemeRecord {
            builtin: false,
            ..record
        })
        .collect();
    let lower_thirds: Vec<LowerThird> = crate::settings::load_lower_thirds(database)?
        .into_iter()
        .filter(|item| lower_third_ids.contains(&item.id))
        .collect();

    let mut media: Vec<MediaItem> = Vec::new();
    for reference in themes
        .iter()
        .filter_map(|record| record.theme.background.media())
    {
        let known = media.iter().any(|item| item.id == reference.id);
        if !known && let Some(item) = database.media().get(reference.id)? {
            media.push(item);
        }
    }
    Ok(PackContents {
        themes,
        lower_thirds,
        media,
    })
}

/// Writes a `.swpack` file with the media embedded. Written beside the
/// target and then renamed, so a failed export never leaves a broken file.
pub fn write(target: &Path, contents: &PackContents, folders: &MediaFolders) -> AppResult<()> {
    let mut sources = Vec::new();
    for item in &contents.media {
        let file = File::open(media::file_path(folders, &item.as_ref()))?;
        sources.push(PackMedia {
            file: item.file.clone(),
            data: Box::new(BufReader::new(file)),
        });
    }

    let mut temporary = target.as_os_str().to_owned();
    temporary.push(".saving");
    let temporary = Path::new(&temporary);
    let written = File::create(temporary)
        .map_err(Into::into)
        .and_then(|file| {
            let writer = sw_files::write_pack_to(BufWriter::new(file), contents, sources)?;
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

/// What of a pack is new to this computer.
#[derive(Debug, Default, PartialEq)]
struct NewToThisComputer {
    themes: Vec<ThemeRecord>,
    lower_thirds: Vec<LowerThird>,
    media: Vec<PackFile>,
    already_here: u32,
}

impl NewToThisComputer {
    /// Gives the media file at `index` the name `name` here, and points the
    /// new themes that show it at that name.
    fn rename_media(&mut self, index: usize, name: String) {
        let file = &mut self.media[index].item;
        for record in &mut self.themes {
            if let Some(reference) = record.theme.background.media_mut()
                && reference.id == file.id
            {
                reference.file.clone_from(&name);
            }
        }
        file.file = name;
    }
}

/// `wanted` if no file in `folder` has that name, or else the first of
/// `stem-1.ext`, `stem-2.ext`… that is free.
fn unused_name(folder: &Path, wanted: &str) -> String {
    if !folder.join(wanted).exists() {
        return wanted.to_owned();
    }
    let path = Path::new(wanted);
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let extension = path
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    (1..)
        .map(|n| format!("{stem}-{n}{extension}"))
        .find(|name| !folder.join(name).exists())
        .expect("some name is free")
}

/// A media file to copy out of a pack, and its record here.
#[derive(Debug, PartialEq)]
struct PackFile {
    /// Its name inside the pack.
    in_pack: String,
    /// Its record here, under its name here.
    item: MediaItem,
}

/// Picks what a pack adds: themes, lower thirds and media files whose ids
/// this computer does not have (media only if the pack carries the file),
/// each once even if the pack lists it twice.
///
/// Each media file gets its own id as its name here (as files imported
/// into the library do): a pack can then never name, and so overwrite, a
/// file this computer already has. Every new theme is pointed at the file
/// as it is named here, whether copied now or already in the library
/// (`media_here` gives a library file's name by its id).
fn new_to_this_computer(
    contents: PackContents,
    themes_here: &HashSet<Id>,
    lower_thirds_here: &HashSet<Id>,
    media_here: impl Fn(Id) -> AppResult<Option<String>>,
    files_in_pack: &[String],
) -> AppResult<NewToThisComputer> {
    let mut new = NewToThisComputer::default();
    let mut themes_seen = themes_here.clone();
    for record in contents.themes {
        if themes_seen.insert(record.id) {
            new.themes.push(ThemeRecord {
                builtin: false,
                ..record
            });
        } else {
            new.already_here += 1;
        }
    }
    let mut lower_thirds_seen = lower_thirds_here.clone();
    for item in contents.lower_thirds {
        if lower_thirds_seen.insert(item.id) {
            new.lower_thirds.push(item);
        } else {
            new.already_here += 1;
        }
    }
    let mut media_seen = HashSet::new();
    for item in contents.media {
        if !files_in_pack.contains(&item.file)
            || !media_seen.insert(item.id)
            || media_here(item.id)?.is_some()
        {
            continue;
        }
        let local = local_file_name(item.id, &item.file);
        new.media.push(PackFile {
            in_pack: item.file.clone(),
            // Its folder in the Media tab belongs to the other computer.
            item: MediaItem {
                file: local,
                collection: None,
                ..item
            },
        });
    }

    for record in &mut new.themes {
        let Some(reference) = record.theme.background.media_mut() else {
            continue;
        };
        let copied = new.media.iter().find(|file| file.item.id == reference.id);
        if let Some(file) = copied {
            reference.file.clone_from(&file.item.file);
        } else if let Some(file) = media_here(reference.id)? {
            reference.file = file;
        }
    }
    Ok(new)
}

/// The name a media file gets here: its id, and the extension it came with
/// (letters and digits only).
fn local_file_name(id: Id, file_in_pack: &str) -> String {
    let extension: String = Path::new(file_in_pack)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    if extension.is_empty() {
        id.to_string()
    } else {
        format!("{id}.{extension}")
    }
}

/// Copies one media file out of the pack. A partly copied file is removed,
/// so a failed import leaves nothing behind for a retry to trip over.
fn copy_out<R: std::io::Read + std::io::Seek>(
    reader: &mut PackReader<R>,
    in_pack: &str,
    destination: &Path,
) -> AppResult<()> {
    let copied = (|| -> AppResult<()> {
        let mut out = BufWriter::new(File::create(destination)?);
        reader.copy_media(in_pack, &mut out)?;
        out.into_inner()
            .map_err(|error| error.into_error())?
            .sync_all()?;
        Ok(())
    })();
    if copied.is_err() {
        let _ = std::fs::remove_file(destination);
    }
    copied
}

/// Imports a `.swpack` file: adds the themes, lower thirds and media files
/// this computer lacks. Fonts are checked against `fonts_here`.
///
/// The caller holds the library's lock (`database`) throughout, so a
/// lower-third save from the operator window cannot land between reading
/// the list and writing it back.
pub fn read(
    path: &Path,
    database: &Database,
    folders: &MediaFolders,
    fonts_here: &[String],
) -> AppResult<PackImported> {
    let mut reader = PackReader::new(BufReader::new(File::open(path)?))?;
    let contents = reader.contents()?;
    let missing_fonts = missing_fonts(&contents, fonts_here);

    let themes_here: HashSet<Id> = database
        .themes()
        .library()?
        .themes
        .iter()
        .map(|r| r.id)
        .collect();
    let mut lower_thirds = crate::settings::load_lower_thirds(database)?;
    let lower_thirds_here: HashSet<Id> = lower_thirds.iter().map(|item| item.id).collect();
    let mut new = new_to_this_computer(
        contents,
        &themes_here,
        &lower_thirds_here,
        |id| Ok(database.media().get(id)?.map(|item| item.file)),
        &reader.media_files(),
    )?;

    // Files first, so a theme never points at a picture not yet copied.
    for index in 0..new.media.len() {
        // A file in the folder may already have this name (one brought by
        // an older schedule keeps the name it had): never write over it.
        let wanted = &new.media[index].item.file;
        let name = unused_name(&folders.media, wanted);
        if &name != wanted {
            new.rename_media(index, name);
        }
        let file = &new.media[index];
        let destination: PathBuf = folders.media.join(&file.item.file);
        copy_out(&mut reader, &file.in_pack, &destination)?;
        database.media().save(&file.item)?;
    }
    for record in &new.themes {
        database.themes().save(record)?;
    }
    if !new.lower_thirds.is_empty() {
        lower_thirds.extend(new.lower_thirds.iter().cloned());
        crate::settings::save_lower_thirds(database, &lower_thirds)?;
    }

    Ok(PackImported {
        themes_added: u32::try_from(new.themes.len()).unwrap_or(u32::MAX),
        lower_thirds_added: u32::try_from(new.lower_thirds.len()).unwrap_or(u32::MAX),
        already_here: new.already_here,
        missing_fonts,
    })
}

/// CSS generic families, which every computer has.
const GENERIC_FAMILIES: &[&str] = &[
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
];

/// The font a CSS font list asks for first, e.g. `Open Sans` from
/// `"Open Sans", Arial, sans-serif`; `None` for an empty list or a generic
/// family.
fn wanted_font(list: &str) -> Option<String> {
    let first = list
        .split(',')
        .next()?
        .trim()
        .trim_matches(['"', '\''])
        .trim();
    let generic = GENERIC_FAMILIES
        .iter()
        .any(|generic| generic.eq_ignore_ascii_case(first));
    (!first.is_empty() && !generic).then(|| first.to_owned())
}

/// The fonts the pack uses that are not in `fonts_here`, alphabetically.
fn missing_fonts(contents: &PackContents, fonts_here: &[String]) -> Vec<String> {
    let themes = contents.themes.iter().flat_map(|record| {
        let theme = &record.theme;
        [
            &theme.text.font_family,
            &theme.footer.font_family,
            &theme.caption.font_family,
        ]
    });
    let lower_thirds = contents
        .lower_thirds
        .iter()
        .map(|item| &item.look.font_family);
    let used: BTreeSet<String> = themes
        .chain(lower_thirds)
        .filter_map(|list| wanted_font(list))
        .collect();
    used.into_iter()
        .filter(|font| {
            !fonts_here
                .iter()
                .any(|here| here.eq_ignore_ascii_case(font))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sw_core::lower_thirds::LowerThirdKind;
    use sw_core::{MediaKind, MediaScaling, Theme};

    fn theme(name: &str, font: &str) -> ThemeRecord {
        let mut theme = Theme {
            name: name.into(),
            ..Theme::default()
        };
        theme.text.font_family = font.into();
        ThemeRecord {
            id: Id::new(),
            theme,
            builtin: true,
        }
    }

    fn picture(file: &str) -> MediaItem {
        MediaItem {
            id: Id::new(),
            kind: MediaKind::Image,
            name: "Sunrise".into(),
            file: file.into(),
            width: None,
            height: None,
            duration_ms: None,
            size_bytes: 0,
            created_at: 0,
            collection: Some(Id::new()),
            scaling: MediaScaling::Automatic,
        }
    }

    #[test]
    fn adds_only_what_this_computer_lacks() {
        let mine = theme("Mine", "Lato");
        let theirs = theme("Theirs", "Lato");
        let lower_third = LowerThird::new(LowerThirdKind::Social, "@grace", "");
        let carried = picture("a.jpg");
        let not_carried = picture("b.jpg");
        let contents = PackContents {
            themes: vec![mine.clone(), theirs.clone()],
            lower_thirds: vec![lower_third.clone()],
            media: vec![carried.clone(), not_carried],
        };

        let new = new_to_this_computer(
            contents,
            &HashSet::from([mine.id]),
            &HashSet::from([lower_third.id]),
            |_| Ok(None),
            &["a.jpg".to_owned()],
        )
        .unwrap();

        assert_eq!(new.themes.len(), 1);
        assert_eq!(new.themes[0].id, theirs.id);
        assert!(
            !new.themes[0].builtin,
            "an imported starter theme is an ordinary theme"
        );
        assert!(new.lower_thirds.is_empty());
        assert_eq!(new.already_here, 2);
        assert_eq!(new.media.len(), 1, "only files the pack carries");
        assert_eq!(new.media[0].item.id, carried.id);
        assert_eq!(
            new.media[0].item.collection, None,
            "the other computer's folders stay there"
        );
    }

    #[test]
    fn media_this_computer_has_is_not_copied_again() {
        let carried = picture("a.jpg");
        let contents = PackContents {
            media: vec![carried.clone()],
            ..PackContents::default()
        };
        let new = new_to_this_computer(
            contents,
            &HashSet::new(),
            &HashSet::new(),
            |id| Ok((id == carried.id).then(|| "a.jpg".to_owned())),
            &["a.jpg".to_owned()],
        )
        .unwrap();
        assert!(new.media.is_empty());
    }

    #[test]
    fn reads_the_font_a_css_list_asks_for() {
        assert_eq!(
            wanted_font(r#""Open Sans", Arial, sans-serif"#).as_deref(),
            Some("Open Sans")
        );
        assert_eq!(wanted_font("Nyala").as_deref(), Some("Nyala"));
        assert_eq!(wanted_font("sans-serif"), None);
        assert_eq!(wanted_font(""), None);
    }

    #[test]
    fn names_the_fonts_this_computer_lacks() {
        let mut lower_third = LowerThird::new(LowerThirdKind::Name, "Pastor", "");
        lower_third.look.font_family = "Gotham".into();
        let contents = PackContents {
            themes: vec![
                theme("A", r#""Lato", sans-serif"#),
                theme("B", "Bebas Neue"),
            ],
            lower_thirds: vec![lower_third],
            media: Vec::new(),
        };
        let here = ["lato".to_owned(), "Montserrat".to_owned()];
        assert_eq!(
            missing_fonts(&contents, &here),
            vec!["Bebas Neue", "Gotham"]
        );
    }

    #[test]
    fn a_pack_listing_something_twice_adds_it_once() {
        let record = theme("Twice", "Lato");
        let lower_third = LowerThird::new(LowerThirdKind::Name, "Pastor", "");
        let file = picture("a.jpg");
        let contents = PackContents {
            themes: vec![record.clone(), record],
            lower_thirds: vec![lower_third.clone(), lower_third],
            media: vec![file.clone(), file],
        };
        let new = new_to_this_computer(
            contents,
            &HashSet::new(),
            &HashSet::new(),
            |_| Ok(None),
            &["a.jpg".to_owned()],
        )
        .unwrap();
        assert_eq!(
            (new.themes.len(), new.lower_thirds.len(), new.media.len()),
            (1, 1, 1)
        );
    }

    #[test]
    fn a_pack_cannot_name_a_file_this_computer_has() {
        // A crafted pack reuses the name of one of this computer's files.
        let ours = picture("ours.jpg");
        let mut theirs = picture("ours.jpg");
        theirs.id = Id::new();
        let mut record = theme("Crafted", "Lato");
        record.theme.background = sw_core::Background::Image {
            media: theirs.as_ref(),
            fit: sw_core::MediaFit::Cover,
            file_fit: None,
            opacity: 1.0,
        };
        let contents = PackContents {
            themes: vec![record],
            lower_thirds: Vec::new(),
            media: vec![theirs.clone()],
        };
        let new = new_to_this_computer(
            contents,
            &HashSet::new(),
            &HashSet::new(),
            |id| Ok((id == ours.id).then(|| "ours.jpg".to_owned())),
            &["ours.jpg".to_owned()],
        )
        .unwrap();

        let file = &new.media[0];
        assert_eq!(file.in_pack, "ours.jpg");
        assert_eq!(
            file.item.file,
            format!("{}.jpg", theirs.id),
            "named by its own id"
        );
        assert_eq!(
            new.themes[0]
                .theme
                .background
                .media()
                .map(|media| media.file.clone()),
            Some(file.item.file.clone()),
            "the theme points at the renamed file"
        );
    }

    #[test]
    fn a_theme_using_a_picture_already_here_points_at_its_file_here() {
        // An earlier pack brought the picture, renamed to its id here; a
        // second pack has another theme on it, under its old name.
        let sunrise = picture("sunrise.jpg");
        let here = format!("{}.jpg", sunrise.id);
        let mut record = theme("Second", "Lato");
        record.theme.background = sw_core::Background::Image {
            media: sunrise.as_ref(),
            fit: sw_core::MediaFit::Cover,
            file_fit: None,
            opacity: 1.0,
        };
        let contents = PackContents {
            themes: vec![record],
            lower_thirds: Vec::new(),
            media: vec![sunrise.clone()],
        };
        let new = new_to_this_computer(
            contents,
            &HashSet::new(),
            &HashSet::new(),
            |id| Ok((id == sunrise.id).then(|| here.clone())),
            &["sunrise.jpg".to_owned()],
        )
        .unwrap();
        assert!(new.media.is_empty(), "not copied again");
        assert_eq!(
            new.themes[0]
                .theme
                .background
                .media()
                .map(|media| media.file.clone()),
            Some(here)
        );
    }

    #[test]
    fn local_names_keep_only_a_plain_extension() {
        let id = Id::new();
        assert_eq!(local_file_name(id, "x.JPG"), format!("{id}.jpg"));
        assert_eq!(local_file_name(id, "x.jp-g!"), format!("{id}.jpg"));
        assert_eq!(local_file_name(id, "x"), id.to_string());
    }

    #[test]
    fn an_imported_file_never_writes_over_one_with_the_same_name() {
        let folder = std::env::temp_dir().join(format!("sw-pack-names-{}", Id::new()));
        let (ours, our_folders) = library(&folder.join("ours"));
        let (theirs, their_folders) = library(&folder.join("theirs"));

        // Their library: a picture brought by an older schedule, under a
        // name made from another id, the very id our pack's picture has.
        let sunrise = picture("sunrise.jpg");
        let mut older = picture("older.jpg");
        older.file = format!("{}.jpg", sunrise.id);
        std::fs::write(their_folders.media.join(&older.file), b"their picture").unwrap();
        theirs.media().save(&older).unwrap();

        std::fs::write(our_folders.media.join("sunrise.jpg"), b"our picture").unwrap();
        ours.media().save(&sunrise).unwrap();
        let mut record = theme("Sunrise", "Lato");
        record.builtin = false;
        record.theme.background = sw_core::Background::Image {
            media: sunrise.as_ref(),
            fit: sw_core::MediaFit::Cover,
            file_fit: None,
            opacity: 1.0,
        };
        ours.themes().save(&record).unwrap();
        let file = folder.join("Sunrise.swpack");
        write(
            &file,
            &gather(&ours, &[record.id], &[]).unwrap(),
            &our_folders,
        )
        .unwrap();

        read(&file, &theirs, &their_folders, &[]).unwrap();

        assert_eq!(
            std::fs::read(their_folders.media.join(&older.file)).unwrap(),
            b"their picture",
            "left as it was"
        );
        let copied = theirs.media().get(sunrise.id).unwrap().unwrap();
        assert_eq!(copied.file, format!("{}-1.jpg", sunrise.id));
        assert_eq!(
            std::fs::read(their_folders.media.join(&copied.file)).unwrap(),
            b"our picture"
        );
        let imported = theirs
            .themes()
            .library()
            .unwrap()
            .get(record.id)
            .cloned()
            .unwrap();
        assert_eq!(
            imported
                .theme
                .background
                .media()
                .map(|media| media.file.clone()),
            Some(copied.file)
        );
        std::fs::remove_dir_all(&folder).unwrap();
    }

    /// A library with its own media folder, in a temporary directory.
    fn library(folder: &Path) -> (Database, MediaFolders) {
        let folders = MediaFolders {
            media: folder.join("media"),
            thumbnails: folder.join("thumbnails"),
        };
        std::fs::create_dir_all(&folders.media).unwrap();
        (Database::open_in_memory().unwrap(), folders)
    }

    #[test]
    fn a_pack_carries_a_theme_its_picture_and_a_lower_third_to_another_computer() {
        let folder = std::env::temp_dir().join(format!("sw-pack-test-{}", Id::new()));
        let (ours, our_folders) = library(&folder.join("ours"));
        let (theirs, their_folders) = library(&folder.join("theirs"));

        // Our library: a theme on a picture, and a lower third.
        let sunrise = picture("sunrise.jpg");
        std::fs::write(our_folders.media.join("sunrise.jpg"), b"jpeg bytes").unwrap();
        ours.media().save(&sunrise).unwrap();
        let mut record = theme("Sunrise", "Lato");
        record.builtin = false;
        record.theme.background = sw_core::Background::Image {
            media: sunrise.as_ref(),
            fit: sw_core::MediaFit::Cover,
            file_fit: None,
            opacity: 1.0,
        };
        ours.themes().save(&record).unwrap();
        let momo = LowerThird::new(
            LowerThirdKind::Offering,
            "MoMo: 024 123 4567",
            "Grace Chapel",
        );
        crate::settings::save_lower_thirds(&ours, std::slice::from_ref(&momo)).unwrap();

        let file = folder.join("Grace.swpack");
        let contents = gather(&ours, &[record.id], &[momo.id]).unwrap();
        write(&file, &contents, &our_folders).unwrap();

        let fonts = ["Lato".to_owned()];
        // A new library starts with the starter lower thirds.
        let starters = crate::settings::load_lower_thirds(&theirs).unwrap().len();
        let first = read(&file, &theirs, &their_folders, &fonts).unwrap();
        assert_eq!(
            first,
            PackImported {
                themes_added: 1,
                lower_thirds_added: 1,
                already_here: 0,
                missing_fonts: Vec::new(),
            }
        );
        assert!(theirs.themes().library().unwrap().get(record.id).is_some());
        let list = crate::settings::load_lower_thirds(&theirs).unwrap();
        assert_eq!(list.len(), starters + 1);
        assert_eq!(list.last(), Some(&momo), "added after their own");
        let copied = theirs.media().get(sunrise.id).unwrap().unwrap();
        assert_eq!(
            copied.file,
            format!("{}.jpg", sunrise.id),
            "named by its id here"
        );
        assert_eq!(
            std::fs::read(their_folders.media.join(&copied.file)).unwrap(),
            b"jpeg bytes"
        );
        let theme = theirs
            .themes()
            .library()
            .unwrap()
            .get(record.id)
            .cloned()
            .unwrap();
        assert_eq!(
            theme
                .theme
                .background
                .media()
                .map(|media| media.file.clone()),
            Some(copied.file),
            "the theme shows the copied file"
        );

        // The same pack again adds nothing.
        let again = read(&file, &theirs, &their_folders, &fonts).unwrap();
        assert_eq!(
            (
                again.themes_added,
                again.lower_thirds_added,
                again.already_here
            ),
            (0, 0, 2)
        );
        assert_eq!(
            crate::settings::load_lower_thirds(&theirs).unwrap().len(),
            starters + 1
        );

        std::fs::remove_dir_all(&folder).unwrap();
    }
}
