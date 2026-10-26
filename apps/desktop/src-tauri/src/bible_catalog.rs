//! More Bibles from eBible.org: its catalogue of translations that may be
//! shared freely, and downloading one into the library.
//!
//! eBible.org publishes a list of every translation it has
//! (`translations.csv`) and each one's text as a zip of USFM files, the
//! same kind of file the bundled KJV and WEB come in. Only translations it
//! marks as redistributable and downloadable are offered. The list is kept
//! in the app's data folder, so the window still opens without internet,
//! and fetched again when it is a week old or the operator asks.
//!
//! A downloaded Bible is imported like any other, with the catalogue's name
//! and copyright line, and remembers where it came from (`ebible:<id>`), so
//! the catalogue can show it as installed.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use sw_core::BibleInfo;
use sw_db::Database;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::bibles;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// eBible.org's list of translations.
const CATALOG_URL: &str = "https://ebible.org/Scriptures/translations.csv";
/// The file the list is kept in, in the app's data folder.
const CATALOG_FILE: &str = "bible-catalog.csv";
/// A list older than this is fetched again when the window opens.
const CATALOG_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// How long fetching the list may take before the kept copy is used.
const CATALOG_TIMEOUT: Duration = Duration::from_secs(20);
/// What `source` starts with for a Bible from eBible.org.
const SOURCE_PREFIX: &str = "ebible:";

/// Sent while a Bible downloads. Keep in sync with
/// `apps/desktop/src/lib/bible-catalog.svelte.ts`.
pub const DOWNLOAD_EVENT: &str = "bibles://download";

/// How much of the Bible a translation has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub enum Coverage {
    /// Old and New Testaments.
    Bible,
    /// The whole New Testament.
    NewTestament,
    /// The whole Old Testament.
    OldTestament,
    /// Some books only.
    Portions,
}

/// A translation that can be downloaded.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct CatalogBible {
    /// eBible.org's id for it, e.g. `"twi"`.
    pub id: String,
    /// The ISO 639-3 code of its language, e.g. `"twi"`, `"eng"`.
    pub language_code: String,
    /// Its language in that language, e.g. "Akuapem Twi".
    pub language: String,
    /// Its language in English, e.g. "Twi".
    pub language_english: String,
    /// Its name, e.g. "Akuapem Twi Bible".
    pub name: String,
    /// Its full title, in its own language.
    pub title: String,
    /// The short name it gets in the library, e.g. "TWI".
    pub abbreviation: String,
    /// The copyright line, shown with its verses.
    pub copyright: String,
    /// How much of the Bible it has.
    pub coverage: Coverage,
    /// Written right to left (Arabic, Hebrew…).
    pub right_to_left: bool,
    /// Already in the library.
    pub installed: bool,
}

/// The catalogue, as the window shows it.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct BibleCatalog {
    /// The translations, by language, then name.
    pub bibles: Vec<CatalogBible>,
    /// The list could not be fetched just now: this is the copy kept from
    /// before, which may be out of date.
    pub offline: bool,
}

/// How far a download has got, sent as [`DOWNLOAD_EVENT`].
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct BibleDownload {
    /// The translation's id.
    pub id: String,
    /// From 0 to 1 while downloading; 1 while it is being imported.
    pub progress: f32,
}

/// The translations that can be downloaded. Fetches the list from
/// eBible.org when `refresh` is set or the kept copy is a week old; without
/// internet, uses the kept copy.
#[tauri::command]
pub async fn bible_catalog(app: AppHandle, refresh: bool) -> AppResult<BibleCatalog> {
    let path = catalog_path(&app)?;
    let fresh = !refresh && is_recent(&path);
    let (text, offline) = if fresh {
        (std::fs::read_to_string(&path)?, false)
    } else {
        match fetch_catalog().await {
            Ok(text) => {
                crate::files::write_atomically(&path, text.as_bytes())?;
                (text, false)
            }
            Err(error) => {
                log::warn!("bibles: could not fetch eBible.org's catalogue: {error}");
                match std::fs::read_to_string(&path) {
                    Ok(text) => (text, true),
                    Err(_) => return Err(error),
                }
            }
        }
    };

    let installed = installed_ids(&app)?;
    let mut bibles = parse_catalog(&text);
    for bible in &mut bibles {
        bible.installed = installed.contains(&bible.id);
    }
    Ok(BibleCatalog { bibles, offline })
}

/// Downloads the translation `id` from eBible.org and imports it, reporting
/// progress as [`DOWNLOAD_EVENT`]. Resolves to the new Bible.
#[tauri::command]
pub async fn bible_catalog_install(app: AppHandle, id: String) -> AppResult<BibleInfo> {
    // Only what the kept catalogue offers: this also keeps `id` to the
    // letters, digits and dashes eBible.org uses, so it is safe in a URL
    // and a file name.
    let text = std::fs::read_to_string(catalog_path(&app)?)?;
    let entry = parse_catalog(&text)
        .into_iter()
        .find(|bible| bible.id == id)
        .ok_or_else(|| AppError::Download("that Bible is not in eBible.org's list".into()))?;
    if installed_ids(&app)?.contains(&entry.id) {
        return Err(AppError::Download(format!(
            "{} is already installed",
            entry.name
        )));
    }

    let folder = app.path().app_cache_dir()?;
    std::fs::create_dir_all(&folder)?;
    let zip = folder.join(format!("{}_usfm.zip", entry.id));
    let result = match download(&app, &entry, &zip).await {
        Ok(()) => {
            let import = {
                let app = app.clone();
                let zip = zip.clone();
                move || install(&app, &zip, &entry)
            };
            tauri::async_runtime::spawn_blocking(import)
                .await
                .map_err(|error| AppError::Download(error.to_string()))
                .and_then(|result| result)
        }
        Err(error) => Err(error),
    };
    // Downloaded or not, the zip is not kept: the library has the verses.
    let _ = std::fs::remove_file(&zip);
    result
}

/// Imports the downloaded `zip` of `entry`, with the catalogue's name and
/// copyright, and remembers where it came from.
fn install(app: &AppHandle, zip: &std::path::Path, entry: &CatalogBible) -> AppResult<BibleInfo> {
    let mut bible = bibles::read(zip)?;
    entry.name.clone_into(&mut bible.name);
    entry.abbreviation.clone_into(&mut bible.abbreviation);
    entry.copyright.clone_into(&mut bible.copyright);
    if bible.language.is_empty() {
        entry.language_code.clone_into(&mut bible.language);
    }
    let database = Database::open(&app.state::<AppState>().database_path)?;
    let info = database.bibles().import(&bible, None)?;
    database
        .bibles()
        .set_source(info.id, &format!("{SOURCE_PREFIX}{}", entry.id))?;
    bibles::announce(app);
    log::info!("bibles: installed {} from eBible.org", entry.id);
    Ok(info)
}

/// Downloads `entry`'s USFM zip to `path`, reporting progress.
async fn download(app: &AppHandle, entry: &CatalogBible, path: &std::path::Path) -> AppResult<()> {
    use std::io::Write;

    let failed = |error: reqwest::Error| AppError::Download(error.to_string());

    let url = format!("https://ebible.org/Scriptures/{}_usfm.zip", entry.id);
    let mut response = crate::web::client()
        .map_err(failed)?
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(failed)?;
    let total = response.content_length().unwrap_or(0);

    let mut file = std::fs::File::create(path)?;
    let mut received: u64 = 0;
    let mut reported = std::time::Instant::now();
    let report = |progress: f32| {
        let _ = app.emit(
            DOWNLOAD_EVENT,
            BibleDownload {
                id: entry.id.clone(),
                progress,
            },
        );
    };
    report(0.0);
    while let Some(chunk) = response.chunk().await.map_err(failed)? {
        file.write_all(&chunk)?;
        received += chunk.len() as u64;
        if total > 0 && reported.elapsed() >= Duration::from_millis(200) {
            reported = std::time::Instant::now();
            report((received as f32 / total as f32).min(0.99));
        }
    }
    file.flush()?;
    report(1.0);
    Ok(())
}

async fn fetch_catalog() -> AppResult<String> {
    let failed = |error: reqwest::Error| AppError::Download(error.to_string());
    crate::web::client()
        .map_err(failed)?
        .get(CATALOG_URL)
        .timeout(CATALOG_TIMEOUT)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(failed)?
        .text()
        .await
        .map_err(failed)
}

fn catalog_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app.path().app_data_dir()?.join(CATALOG_FILE))
}

/// Whether the kept list at `path` exists and is less than a week old.
fn is_recent(path: &std::path::Path) -> bool {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age < CATALOG_MAX_AGE)
}

/// The eBible.org ids of the Bibles already downloaded into the library.
fn installed_ids(app: &AppHandle) -> AppResult<Vec<String>> {
    let sources = app.state::<AppState>().database().bibles().sources()?;
    Ok(sources
        .into_iter()
        .filter_map(|source| source.strip_prefix(SOURCE_PREFIX).map(str::to_owned))
        .collect())
}

/// The downloadable, freely shared translations in eBible.org's
/// `translations.csv`, by language and then name.
fn parse_catalog(text: &str) -> Vec<CatalogBible> {
    let mut rows = csv_rows(text.trim_start_matches('\u{feff}')).into_iter();
    let Some(header) = rows.next() else {
        return Vec::new();
    };
    let column = |name: &str| header.iter().position(|title| title == name);
    let columns = (
        column("translationId"),
        column("languageCode"),
        column("languageName"),
        column("languageNameInEnglish"),
        column("title"),
        column("shortTitle"),
        column("Copyright"),
        column("Redistributable"),
        column("downloadable"),
        column("OTbooks"),
        column("NTbooks"),
        column("textDirection"),
    );
    let (
        Some(id),
        Some(code),
        Some(language),
        Some(english),
        Some(title),
        Some(short),
        Some(copyright),
        Some(shared),
        Some(downloadable),
        Some(old),
        Some(new),
        Some(direction),
    ) = columns
    else {
        log::warn!("bibles: eBible.org's catalogue has unexpected columns");
        return Vec::new();
    };

    let mut bibles: Vec<CatalogBible> = rows
        .filter_map(|row| {
            let field = |at: usize| row.get(at).map(|value| value.trim()).unwrap_or("");
            let books = |at: usize| field(at).parse::<u32>().unwrap_or(0);
            let id = field(id);
            let usable = field(shared) == "True"
                && field(downloadable) == "True"
                && is_safe_id(id)
                && books(old) + books(new) > 0;
            if !usable {
                return None;
            }
            let title = field(title);
            let name = if field(short).is_empty() {
                title
            } else {
                field(short)
            };
            Some(CatalogBible {
                id: id.to_owned(),
                language_code: field(code).to_owned(),
                language: field(language).to_owned(),
                language_english: field(english).to_owned(),
                name: name.to_owned(),
                title: title.to_owned(),
                abbreviation: abbreviation(id, field(code)),
                copyright: field(copyright).to_owned(),
                coverage: coverage(books(old), books(new)),
                right_to_left: field(direction) == "rtl",
                installed: false,
            })
        })
        .collect();
    bibles.sort_by(|a, b| {
        (a.language_english.to_lowercase(), a.name.to_lowercase())
            .cmp(&(b.language_english.to_lowercase(), b.name.to_lowercase()))
    });
    bibles
}

/// How much of the Bible `old` Old Testament and `new` New Testament books
/// make.
fn coverage(old: u32, new: u32) -> Coverage {
    match (old >= 39, new >= 27) {
        (true, true) => Coverage::Bible,
        (false, true) => Coverage::NewTestament,
        (true, false) => Coverage::OldTestament,
        (false, false) => Coverage::Portions,
    }
}

/// A short name from eBible.org's id: the id without its language code in
/// front, e.g. `engbsb` → "BSB", `eng-kjv2006` → "KJV2006", `twiasante` →
/// "ASANTE", and the language code for a language's main one (`twi` →
/// "TWI"). At most 8 letters; the operator can change it.
fn abbreviation(id: &str, language_code: &str) -> String {
    let rest = id
        .strip_prefix(language_code)
        .map(|rest| rest.trim_start_matches(['-', '_']))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(id);
    rest.chars().take(8).collect::<String>().to_uppercase()
}

/// Whether `id` is made of the letters, digits, dashes and underscores
/// eBible.org's ids use (so it is safe in a URL and a file name).
fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The rows of a CSV text: fields separated by commas, any field may be in
/// double quotes (with `""` for a quote, and commas and line breaks
/// inside), rows end with a line break.
fn csv_rows(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            ('"', true) => quoted = false,
            ('"', false) if field.is_empty() => quoted = true,
            (',', false) => row.push(std::mem::take(&mut field)),
            ('\r', false) => {}
            ('\n', false) => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "\u{feff}\"languageCode\",\"translationId\",\"languageName\",\"languageNameInEnglish\",\"title\",\"shortTitle\",\"Copyright\",\"Redistributable\",\"downloadable\",\"OTbooks\",\"NTbooks\",\"textDirection\"\n";

    #[test]
    fn reads_quoted_fields_with_commas_quotes_and_line_breaks() {
        let rows = csv_rows("a,\"b, c\",\"say \"\"hi\"\"\"\r\n\"two\nlines\",x\n");
        assert_eq!(
            rows,
            [vec!["a", "b, c", "say \"hi\""], vec!["two\nlines", "x"]]
        );
    }

    #[test]
    fn offers_only_downloadable_freely_shared_translations() {
        let text = format!(
            "{HEADER}\
             \"twi\",\"twi\",\"Akuapem Twi\",\"Twi\",\"Akuapem Twi Nkwa Asɛm\",\"Akuapem Twi Bible\",\"Copyright © 2020 Biblica, Inc.\",\"True\",\"True\",\"39\",\"27\",\"ltr\"\n\
             \"eng\",\"engnet\",\"English\",\"English\",\"NET Bible\",\"\",\"Copyright © Biblical Studies Press\",\"True\",\"True\",\"39\",\"27\",\"ltr\"\n\
             \"eng\",\"engniv\",\"English\",\"English\",\"Not free\",\"\",\"©\",\"False\",\"True\",\"39\",\"27\",\"ltr\"\n\
             \"aai\",\"aai\",\"Miniafia\",\"Miniafia\",\"Tur Gewasin\",\"\",\"©\",\"True\",\"True\",\"0\",\"27\",\"ltr\"\n\
             \"xyz\",\"../evil\",\"X\",\"X\",\"X\",\"\",\"©\",\"True\",\"True\",\"39\",\"27\",\"ltr\"\n"
        );
        let bibles = parse_catalog(&text);
        let ids: Vec<_> = bibles.iter().map(|bible| bible.id.as_str()).collect();
        assert_eq!(
            ids,
            ["engnet", "aai", "twi"],
            "sorted by language in English"
        );

        let twi = &bibles[2];
        assert_eq!(twi.name, "Akuapem Twi Bible");
        assert_eq!(twi.language, "Akuapem Twi");
        assert_eq!(twi.abbreviation, "TWI");
        assert_eq!(twi.coverage, Coverage::Bible);
        assert_eq!(
            bibles[0].name, "NET Bible",
            "the title when there is no short one"
        );
        assert_eq!(bibles[1].coverage, Coverage::NewTestament);
    }

    #[test]
    fn short_names_come_from_the_id() {
        assert_eq!(abbreviation("engbsb", "eng"), "BSB");
        assert_eq!(abbreviation("eng-kjv2006", "eng"), "KJV2006");
        assert_eq!(abbreviation("twiasante", "twi"), "ASANTE");
        assert_eq!(abbreviation("twi", "twi"), "TWI");
        assert_eq!(abbreviation("frapdb", "fra"), "PDB");
    }

    #[test]
    fn ids_must_be_safe_in_urls_and_file_names() {
        assert!(is_safe_id("eng-kjv2006"));
        assert!(!is_safe_id("../evil"));
        assert!(!is_safe_id("a/b"));
        assert!(!is_safe_id(""));
    }

    /// Reads a real copy of eBible.org's list. Run with
    /// `SW_BIBLE_CATALOG=translations.csv cargo test -p simpleworship-desktop
    /// -- --ignored real_catalog`.
    #[test]
    #[ignore = "needs a copy of eBible.org's translations.csv"]
    fn reads_the_real_catalogue() {
        let path = std::env::var("SW_BIBLE_CATALOG").expect("set SW_BIBLE_CATALOG");
        let bibles = parse_catalog(&std::fs::read_to_string(path).unwrap());
        let whole = bibles
            .iter()
            .filter(|bible| bible.coverage == Coverage::Bible);
        println!(
            "{} translations, {} whole Bibles",
            bibles.len(),
            whole.count()
        );
        assert!(bibles.len() > 1000);
        let twi = bibles.iter().find(|bible| bible.id == "twiasante").unwrap();
        assert_eq!(twi.name, "Asante Twi Bible");
        assert_eq!(twi.coverage, Coverage::Bible);
    }
}
