use super::{CensoredString, LogType, resolve_instance_path};
use crate::util::file_stamp::FileStamp;
use crate::{ErrorKind, State, prelude::Credentials, state::ElyCredentials};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::Instant;

const PAGE_BYTES: u64 = 256 * 1024;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoricalLogCursor {
    before: u64,
    stamp: String,
}

#[derive(Serialize)]
pub struct HistoricalLogPage {
    pub output: CensoredString,
    pub cursor: Option<HistoricalLogCursor>,
    pub reset: bool,
}

struct ExpandedLog {
    stamp: String,
    file: Arc<tempfile::NamedTempFile>,
    used: Instant,
}

static EXPANDED_LOGS: LazyLock<Mutex<HashMap<PathBuf, ExpandedLog>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn open_log(
    path: &Path,
    stamp: &str,
    source: File,
) -> crate::Result<(File, Option<Arc<tempfile::NamedTempFile>>)> {
    if path.extension().is_none_or(|ext| ext != "gz") {
        return Ok((source, None));
    }
    {
        let mut cache = EXPANDED_LOGS.lock();
        if let Some(entry) = cache.get_mut(path)
            && entry.stamp == stamp
        {
            entry.used = Instant::now();
            return Ok((entry.file.reopen()?, Some(entry.file.clone())));
        }
    }
    let mut source = flate2::read::GzDecoder::new(source);
    let mut expanded = tempfile::NamedTempFile::new()?;
    std::io::copy(&mut source, &mut expanded)?;
    expanded.flush()?;
    let file = Arc::new(expanded);
    let mut cache = EXPANDED_LOGS.lock();
    if cache.len() >= 8
        && let Some(oldest) = cache
            .iter()
            .min_by_key(|(_, entry)| entry.used)
            .map(|(path, _)| path.clone())
    {
        cache.remove(&oldest);
    }
    cache.insert(
        path.to_owned(),
        ExpandedLog {
            stamp: stamp.to_owned(),
            file: file.clone(),
            used: Instant::now(),
        },
    );
    Ok((file.reopen()?, Some(file)))
}

fn read_page(
    path: &Path,
    cursor: Option<HistoricalLogCursor>,
) -> crate::Result<(String, Option<HistoricalLogCursor>, bool)> {
    let source = File::open(path)?;
    let stamp = format!(
        "{:?}",
        FileStamp::read(&source).ok_or_else(|| ErrorKind::InputError(
            "cannot identify log file".into()
        ))?
    );
    let (mut file, _temporary) = open_log(path, &stamp, source.try_clone()?)?;
    let size = file.metadata()?.len();
    let mut reset = cursor
        .as_ref()
        .is_some_and(|cursor| cursor.stamp != stamp || cursor.before > size);
    if let Some(cursor) = &cursor
        && !reset
        && cursor.before > 0
        && cursor.before < size
    {
        file.seek(SeekFrom::Start(cursor.before - 1))?;
        let mut previous = [0];
        file.read_exact(&mut previous)?;
        reset = previous[0] != b'\n';
    }
    let end = cursor
        .filter(|_| !reset)
        .map_or(size, |cursor| cursor.before);
    let start = end.saturating_sub(PAGE_BYTES);
    file.seek(SeekFrom::Start(start.saturating_sub(1)))?;
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(end - start.saturating_sub(1))
        .read_to_end(&mut bytes)?;
    let skip = if start == 0 {
        0
    } else {
        bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |index| index + 1)
    };
    let before = start.saturating_sub(1) + skip as u64;
    // shortcut: single lines larger than a page are omitted; keep IPC bounded even for corrupt logs.
    let (output, before) = if skip == bytes.len() && start > 0 {
        let mut before = start.saturating_sub(1);
        while before > 0 {
            let from = before.saturating_sub(PAGE_BYTES);
            file.seek(SeekFrom::Start(from))?;
            let mut block = vec![0; (before - from) as usize];
            file.read_exact(&mut block)?;
            if let Some(newline) = block.iter().rposition(|byte| *byte == b'\n')
            {
                before = from + newline as u64 + 1;
                break;
            }
            before = from;
        }
        ("[Log line exceeds display limit]\n".to_owned(), before)
    } else {
        (String::from_utf8_lossy(&bytes[skip..]).into_owned(), before)
    };
    let after = FileStamp::read(&source);
    let current = File::open(path)
        .ok()
        .and_then(|file| FileStamp::read(&file));
    if after.is_none_or(|after| format!("{after:?}") != stamp)
        || current.is_none_or(|current| format!("{current:?}") != stamp)
    {
        return Err(ErrorKind::InputError(
            "log changed while reading; select it again".into(),
        )
        .into());
    }
    Ok((
        output,
        (before > 0).then_some(HistoricalLogCursor { before, stamp }),
        reset,
    ))
}

pub async fn get_historical_log_page(
    instance_id: &str,
    log_type: LogType,
    filename: &str,
    cursor: Option<HistoricalLogCursor>,
) -> crate::Result<HistoricalLogPage> {
    let state = State::get().await?;
    let instance_path = resolve_instance_path(instance_id, &state).await?;
    if Path::new(filename)
        .file_name()
        .and_then(|name| name.to_str())
        != Some(filename)
        || !matches!(
            Path::new(filename).extension().and_then(|ext| ext.to_str()),
            Some("log" | "txt" | "gz")
        )
    {
        return Err(ErrorKind::InputError("invalid log filename".into()).into());
    }
    let folder = match log_type {
        LogType::InfoLog => state.directories.instance_logs_dir(&instance_path),
        LogType::CrashReport => {
            state.directories.crash_reports_dir(&instance_path)
        }
    };
    let path = folder.join(filename);
    let (output, cursor, reset) =
        tokio::task::spawn_blocking(move || read_page(&path, cursor)).await??;
    let compacted = super::read_compacted_log(&mut std::io::Cursor::new(
        output.as_bytes(),
    ))?;
    super::maybe_emit_log_compaction_warning(filename, compacted.stats).await;
    let output = compacted.output;
    let credentials = Credentials::get_all(&state.pool)
        .await?
        .into_iter()
        .map(|(_, credentials)| credentials)
        .collect::<Vec<_>>();
    let ely = ElyCredentials::get_all(&state.pool)
        .await
        .unwrap_or_default();
    Ok(HistoricalLogPage {
        output: CensoredString::censor_all(output, &credentials, &ely),
        cursor,
        reset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forged_cursors_cannot_split_a_secret_across_pages() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cursor.log");
        let expected = "safe\nSession ID is token:sensitive:uuid\n";
        std::fs::write(&path, expected).unwrap();
        let stamp = format!(
            "{:?}",
            FileStamp::read(&File::open(&path).unwrap()).unwrap()
        );
        let (output, _, reset) =
            read_page(&path, Some(HistoricalLogCursor { before: 30, stamp }))
                .unwrap();
        assert!(reset);
        let censored = CensoredString::censor_all(output, &[], &[]);
        assert!(!censored.as_ref().contains("sensitive"));
    }

    #[test]
    fn oversized_lines_never_return_partial_credentials_or_loop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.log");
        let giant = format!(
            "prefix\nSession ID is {}\nafter\n",
            "sensitive".repeat(100_000)
        );
        std::fs::write(&path, &giant).unwrap();
        let (mut output, mut cursor, _) = read_page(&path, None).unwrap();
        let mut pages = 1;
        while cursor.is_some() {
            let (previous, next, _) = read_page(&path, cursor).unwrap();
            assert!(previous.len() <= PAGE_BYTES as usize);
            output.insert_str(0, &previous);
            cursor = next;
            pages += 1;
            assert!(pages < 10);
        }
        assert!(!output.contains("sensitive"));
        assert!(output.starts_with("prefix\n"));
        assert!(output.ends_with("after\n"));
    }

    #[test]
    fn pages_restore_utf8_lines_and_reset_after_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("latest.log");
        let expected = (0..30_000)
            .map(|index| format!("{index}: привет 🦉\n"))
            .collect::<String>();
        std::fs::write(&path, &expected).unwrap();
        let (tail, mut cursor, _) = read_page(&path, None).unwrap();
        assert!(tail.len() <= PAGE_BYTES as usize);
        assert!(expected.ends_with(&tail));
        let saved_cursor = cursor.clone();
        let mut actual = tail;
        while cursor.is_some() {
            let (earlier, next, reset) = read_page(&path, cursor).unwrap();
            assert!(!reset);
            actual.insert_str(0, &earlier);
            cursor = next;
        }
        assert_eq!(actual, expected);
        std::fs::write(&path, "replacement\n").unwrap();
        let (output, cursor, reset) = read_page(&path, saved_cursor).unwrap();
        assert!(reset);
        assert!(cursor.is_none());
        assert_eq!(output, "replacement\n");
    }

    #[test]
    fn gzip_pages_restore_the_entire_log() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.log.gz");
        let expected = "a log entry\n".repeat(40_000);
        let mut encoder = flate2::write::GzEncoder::new(
            File::create(&path).unwrap(),
            flate2::Compression::fast(),
        );
        encoder.write_all(expected.as_bytes()).unwrap();
        encoder.finish().unwrap();
        let (mut output, mut cursor, _) = read_page(&path, None).unwrap();
        while cursor.is_some() {
            let (earlier, next, _) = read_page(&path, cursor).unwrap();
            output.insert_str(0, &earlier);
            cursor = next;
        }
        assert_eq!(output, expected);
    }
}
