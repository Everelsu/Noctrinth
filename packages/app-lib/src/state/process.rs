use crate::event::emit::{emit_instance, emit_process};
use crate::event::{InstancePayloadType, ProcessPayloadType};
#[cfg(feature = "tauri")]
use crate::event::{LogEvent, LogPayload};
use crate::util::io::IOError;
use crate::util::rpc::RpcServer;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use dashmap::DashMap;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::Deserialize;
use serde::Serialize;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::LazyLock;
use std::time::Instant;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use uuid::Uuid;

const LAUNCHER_LOG_PATH: &str = "launcher_log.txt";
const LOG_BUFFER_CAPACITY: usize = 50_000;

struct LogRingBuffer {
    lines: VecDeque<String>,
}

impl LogRingBuffer {
    fn new() -> Self {
        Self {
            lines: VecDeque::new(),
        }
    }

    fn push(&mut self, line: String) {
        if self.lines.len() >= LOG_BUFFER_CAPACITY {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    fn get_all(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }
}

/// Live output, kept for each copy of an instance separately.
///
/// One instance can be running twice, and two games writing into one buffer is
/// a log nobody can read. Keyed by the process, therefore, with the newest one
/// of each instance remembered below so that a page which knows only the
/// instance still has something to show — a log outlives the game that wrote
/// it, and is worth reading after it has closed.
static LOG_BUFFERS: LazyLock<DashMap<String, LogRingBuffer>> =
    LazyLock::new(DashMap::new);

static INSTANCE_PROCESSES: LazyLock<DashMap<String, Vec<String>>> =
    LazyLock::new(DashMap::new);

pub fn push_log_line(process_uuid: &str, line: String) {
    LOG_BUFFERS
        .entry(process_uuid.to_string())
        .or_insert_with(LogRingBuffer::new)
        .push(crate::api::logs::censor_session_ids(line));
}

pub fn get_log_buffer(process_uuid: &str) -> Vec<String> {
    LOG_BUFFERS
        .get(process_uuid)
        .map(|buf| buf.get_all())
        .unwrap_or_default()
}

/// The live output of the copy of this instance that started last.
pub fn get_instance_log_buffer(instance_id: &str) -> Vec<String> {
    INSTANCE_PROCESSES
        .get(instance_id)
        .and_then(|uuids| uuids.last().map(|uuid| get_log_buffer(uuid)))
        .unwrap_or_default()
}

pub fn remove_log_buffer(process_uuid: &str) {
    LOG_BUFFERS.remove(process_uuid);
}

/// Forgets the live output of every copy of an instance.
///
/// What each copy said goes; which copies there are stays, or a running game
/// would have nowhere to be read from and the live log would stay empty until
/// it was started again.
pub fn remove_instance_log_buffers(instance_id: &str) {
    if let Some(uuids) = INSTANCE_PROCESSES.get(instance_id) {
        for uuid in uuids.value() {
            remove_log_buffer(uuid);
        }
    }
}

/// Takes note of a copy that has just started.
///
/// A launch that is not a second copy is a fresh start, and what the last one
/// said goes with it — which is what starting the game has always done to the
/// live log. A second copy joins whatever is already there instead: the game
/// that wrote it is still running and still being read.
pub fn note_new_process(
    instance_id: &str,
    process_uuid: &str,
    additional: bool,
) {
    if additional {
        INSTANCE_PROCESSES
            .entry(instance_id.to_string())
            .or_default()
            .push(process_uuid.to_string());
        return;
    }

    if let Some(previous) = INSTANCE_PROCESSES
        .insert(instance_id.to_string(), vec![process_uuid.to_string()])
    {
        for uuid in previous {
            remove_log_buffer(&uuid);
        }
    }
}

async fn clear_persisted_process(
    state: &crate::State,
    process: Option<(i64, i64)>,
) {
    let Some((pid, start_time)) = process else {
        return;
    };
    if let Err(error) = sqlx::query!(
        "DELETE FROM processes WHERE pid = ? AND start_time = ?",
        pid,
        start_time,
    )
    .execute(&state.pool)
    .await
    {
        tracing::warn!("Failed to clear persisted process {pid}: {error}");
    }
}

pub(crate) async fn instance_has_running_process(
    instance_id: &str,
    state: &crate::State,
) -> crate::Result<bool> {
    if state
        .process_manager
        .get_all()
        .iter()
        .any(|process| process.instance_id == instance_id)
    {
        return Ok(true);
    }

    let processes = sqlx::query!(
        "
		SELECT pid, start_time
		FROM processes
		WHERE instance_id = ?
		",
        instance_id,
    )
    .fetch_all(&state.pool)
    .await?;
    if processes.is_empty() {
        return Ok(false);
    }
    let system = sysinfo::System::new_all();
    let mut running = false;
    for process in processes {
        let process_is_running = u32::try_from(process.pid)
            .ok()
            .and_then(|pid| system.process(sysinfo::Pid::from_u32(pid)))
            .is_some_and(|system_process| {
                let started_at = system_process.start_time() as i64;
                started_at.abs_diff(process.start_time) <= 2
            });
        if process_is_running {
            running = true;
        } else {
            clear_persisted_process(
                state,
                Some((process.pid, process.start_time)),
            )
            .await;
        }
    }
    Ok(running)
}

pub struct ProcessManager {
    processes: DashMap<Uuid, Process>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: DashMap::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_new_process(
        &self,
        instance_id: &str,
        instance_path: &str,
        instance_name: &str,
        account_name: &str,
        // Whether this instance is already running and this is a second copy of
        // it, whose log joins the one already being written.
        additional: bool,
        mut mc_command: Command,
        post_exit_command: Option<String>,
        post_exit_env_vars: Vec<(String, String)>,
        logs_folder: PathBuf,
        xml_logging: bool,
        main_class_keep_alive: TempDir,
        rpc_server: RpcServer,
        post_process_init: impl AsyncFnOnce(
            &ProcessMetadata,
            &RpcServer,
        ) -> crate::Result<()>,
    ) -> crate::Result<ProcessMetadata> {
        mc_command.stdout(std::process::Stdio::piped());
        mc_command.stderr(std::process::Stdio::piped());
        mc_command.stdin(std::process::Stdio::piped());
        let executable = mc_command
            .as_std()
            .get_program()
            .to_string_lossy()
            .into_owned();

        // Minted before the log header so the note below can name this copy;
        // the process built after the spawn carries the same id.
        let process_uuid = Uuid::new_v4();

        if !logs_folder.exists() {
            tokio::fs::create_dir_all(&logs_folder)
                .await
                .map_err(|e| IOError::with_path(e, &logs_folder))?;
        }

        let log_path = logs_folder.join(LAUNCHER_LOG_PATH);

        // An instance has one log file, so a second copy of it joins the one
        // already being written rather than starting it over: the copy running
        // now is still writing there, and that log is not this one's to throw
        // away. Its live output, on the other hand, is its own.
        note_new_process(instance_id, &process_uuid.to_string(), additional);

        {
            let mut log_file = OpenOptions::new()
                .write(true)
                .create(true)
                .append(additional)
                .truncate(!additional)
                .open(&log_path)
                .map_err(|e| IOError::with_path(e, &log_path))?;

            let now = chrono::Local::now();
            writeln!(
                log_file,
                "# Minecraft launcher log started at {}",
                now.format("%Y-%m-%d %H:%M:%S")
            )
            .map_err(|e| IOError::with_path(e, &log_path))?;
            writeln!(log_file, "# Instance: {instance_path} \n")
                .map_err(|e| IOError::with_path(e, &log_path))?;
            if additional {
                // Two games write here from now on, and whose line is whose is
                // otherwise anybody's guess.
                writeln!(
                    log_file,
                    "# A second copy, signed in as {account_name}"
                )
                .map_err(|e| IOError::with_path(e, &log_path))?;
            }
            writeln!(log_file).map_err(|e| IOError::with_path(e, &log_path))?;
        }

        let mut mc_proc = mc_command.spawn().map_err(IOError::from)?;
        let child_pid = mc_proc.id();

        let stdout = mc_proc.stdout.take();
        let stderr = mc_proc.stderr.take();

        let mut process = Process {
            metadata: ProcessMetadata {
                uuid: process_uuid,
                start_time: Utc::now(),
                instance_id: instance_id.to_string(),
                instance_path: instance_path.to_string(),
                instance_name: instance_name.to_string(),
                account_name: account_name.to_string(),
            },
            child: mc_proc,
            rpc_server,
            _main_class_keep_alive: main_class_keep_alive,
        };

        let state = match crate::State::get().await {
            Ok(state) => state,
            Err(error) => {
                let _ = process.child.kill().await;
                return Err(error);
            }
        };
        let persisted_process = child_pid.map(|pid| {
            (i64::from(pid), process.metadata.start_time.timestamp())
        });
        if let Some((pid, start_time)) = persisted_process {
            let post_exit_command = post_exit_command.as_deref();
            if let Err(error) = sqlx::query!(
                "
				INSERT INTO processes
					(pid, start_time, name, executable, instance_id,
					 post_exit_command)
				VALUES (?, ?, ?, ?, ?, ?)
				ON CONFLICT(pid) DO UPDATE SET
					start_time = excluded.start_time,
					name = excluded.name,
					executable = excluded.executable,
					instance_id = excluded.instance_id,
					post_exit_command = excluded.post_exit_command
				",
                pid,
                start_time,
                instance_name,
                executable,
                instance_id,
                post_exit_command,
            )
            .execute(&state.pool)
            .await
            {
                let _ = process.child.kill().await;
                return Err(error.into());
            }
        }

        if let Err(e) =
            post_process_init(&process.metadata, &process.rpc_server).await
        {
            tracing::error!("Failed to run post-process init: {e}");
            clear_persisted_process(&state, persisted_process).await;
            let _ = process.child.kill().await;
            return Err(e);
        }

        let metadata = process.metadata.clone();

        if let Some(stdout) = stdout {
            let log_path_clone = log_path.clone();

            let instance_id = metadata.instance_id.clone();
            let instance_path = metadata.instance_path.clone();
            let process_uuid = metadata.uuid.to_string();
            tokio::spawn(async move {
                Process::process_output(
                    &instance_id,
                    &instance_path,
                    &process_uuid,
                    stdout,
                    log_path_clone,
                    xml_logging,
                )
                .await;
            });
        }

        if let Some(stderr) = stderr {
            let log_path_clone = log_path.clone();

            let instance_id = metadata.instance_id.clone();
            let instance_path = metadata.instance_path.clone();
            let process_uuid = metadata.uuid.to_string();
            tokio::spawn(async move {
                Process::process_output(
                    &instance_id,
                    &instance_path,
                    &process_uuid,
                    stderr,
                    log_path_clone,
                    xml_logging,
                )
                .await;
            });
        }

        self.processes.insert(process.metadata.uuid, process);

        tokio::spawn(Process::sequential_process_manager(
            instance_id.to_string(),
            instance_path.to_string(),
            post_exit_command,
            post_exit_env_vars,
            metadata.uuid,
            persisted_process,
        ));

        emit_process(
            instance_id,
            metadata.uuid,
            ProcessPayloadType::Launched,
            "Launched Minecraft",
        )
        .await?;

        Ok(metadata)
    }

    pub fn get(&self, id: Uuid) -> Option<ProcessMetadata> {
        self.processes.get(&id).map(|x| x.metadata.clone())
    }

    pub fn get_rpc(&self, id: Uuid) -> Option<RpcServer> {
        self.processes.get(&id).map(|x| x.rpc_server.clone())
    }

    pub fn get_all(&self) -> Vec<ProcessMetadata> {
        self.processes
            .iter()
            .map(|x| x.value().metadata.clone())
            .collect()
    }

    pub fn try_wait(
        &self,
        id: Uuid,
    ) -> crate::Result<Option<Option<ExitStatus>>> {
        if let Some(mut process) = self.processes.get_mut(&id) {
            Ok(Some(process.child.try_wait()?))
        } else {
            Ok(None)
        }
    }

    pub async fn wait_for(&self, id: Uuid) -> crate::Result<()> {
        if let Some(mut process) = self.processes.get_mut(&id) {
            process.child.wait().await?;
        }
        Ok(())
    }

    pub async fn kill(&self, id: Uuid) -> crate::Result<()> {
        if let Some(mut process) = self.processes.get_mut(&id) {
            process.child.kill().await?;
        }

        Ok(())
    }

    fn remove(&self, id: Uuid) {
        self.processes.remove(&id);
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProcessMetadata {
    pub uuid: Uuid,
    pub instance_id: String,
    pub instance_path: String,
    pub instance_name: String,
    /// Who the game was started as.
    ///
    /// One instance can be running more than once, on a different account each
    /// time, and then the name is the only thing telling the two apart.
    pub account_name: String,
    pub start_time: DateTime<Utc>,
}

#[derive(Debug)]
struct Process {
    metadata: ProcessMetadata,
    child: Child,
    _main_class_keep_alive: TempDir,
    rpc_server: RpcServer,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[cfg_attr(
    feature = "export-ts",
    derive(ts_rs::TS, postcard_bindgen::PostcardBindings)
)]
pub struct Log4jEvent {
    pub timestamp_millis: Option<i64>,
    pub logger_name: Option<String>,
    pub level: Option<String>,
    pub thread_name: Option<String>,
    pub message: Option<String>,
    pub throwable: Option<String>,
}

impl Process {
    async fn process_output<R>(
        instance_id: &str,
        _instance_path: &str,
        process_uuid: &str,
        reader: R,
        log_path: impl AsRef<Path>,
        xml_logging: bool,
    ) where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut buf_reader = BufReader::new(reader);

        if xml_logging {
            let mut reader = Reader::from_reader(buf_reader);
            reader.config_mut().enable_all_checks(false);

            let mut buf = Vec::new();
            let mut current_event = Log4jEvent::default();
            let mut in_event = false;
            let mut in_message = false;
            let mut in_throwable = false;
            let mut current_content = String::new();

            loop {
                match reader.read_event_into_async(&mut buf).await {
                    Err(e) => {
                        tracing::error!(
                            "Error at position {}: {:?}",
                            reader.buffer_position(),
                            e
                        );
                        break;
                    }
                    // exits the loop when reaching end of file
                    Ok(Event::Eof) => break,

                    Ok(Event::Start(e)) => {
                        match e.name().as_ref() {
                            b"log4j:Event" => {
                                // Reset for new event
                                current_event = Log4jEvent::default();
                                in_event = true;

                                // Extract attributes
                                for attr in e.attributes().flatten() {
                                    let key = String::from_utf8_lossy(
                                        attr.key.into_inner(),
                                    )
                                    .to_string();
                                    let value =
                                        String::from_utf8_lossy(&attr.value)
                                            .to_string();

                                    match key.as_str() {
                                        "logger" => {
                                            current_event.logger_name =
                                                Some(value)
                                        }
                                        "level" => {
                                            current_event.level = Some(value)
                                        }
                                        "thread" => {
                                            current_event.thread_name =
                                                Some(value)
                                        }
                                        "timestamp" => {
                                            current_event.timestamp_millis =
                                                value.parse::<i64>().ok()
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            b"log4j:Message" => {
                                in_message = true;
                                current_content = String::new();
                            }
                            b"log4j:Throwable" => {
                                in_throwable = true;
                                current_content = String::new();
                            }
                            _ => {}
                        }
                    }
                    Ok(Event::End(e)) => {
                        match e.name().as_ref() {
                            b"log4j:Message" => {
                                in_message = false;
                                current_event.message =
                                    Some(current_content.clone());
                            }
                            b"log4j:Throwable" => {
                                in_throwable = false;
                                current_event.throwable =
                                    if current_content.is_empty() {
                                        None
                                    } else {
                                        Some(current_content.clone())
                                    };

                                // Write log entry + throwable to file
                                if let Some(formatted_log) =
                                    Self::format_log4j_entry(&current_event)
                                {
                                    if let Err(e) = Process::append_to_log_file(
                                        &log_path,
                                        &formatted_log,
                                    ) {
                                        tracing::error!(
                                            "Failed to write to log file: {}",
                                            e
                                        );
                                    }

                                    if let Some(ref throwable) =
                                        current_event.throwable
                                        && let Err(e) =
                                            Process::append_to_log_file(
                                                &log_path, throwable,
                                            )
                                    {
                                        tracing::error!(
                                            "Failed to write throwable to log file: {}",
                                            e
                                        );
                                    }
                                }

                                Self::emit_log4j_event(
                                    instance_id,
                                    process_uuid,
                                    &current_event,
                                );
                            }
                            b"log4j:Event" => {
                                in_event = false;
                                // If no throwable was present, write the log entry at the end of the event
                                if current_event.message.is_some()
                                    && current_event.throwable.is_none()
                                {
                                    if let Some(formatted_log) =
                                        Self::format_log4j_entry(&current_event)
                                        && let Err(e) =
                                            Process::append_to_log_file(
                                                &log_path,
                                                &formatted_log,
                                            )
                                    {
                                        tracing::error!(
                                            "Failed to write to log file: {}",
                                            e
                                        );
                                    }

                                    if let Some(timestamp_millis) =
                                        current_event.timestamp_millis
                                    {
                                        let timestamp =
                                            timestamp_millis.to_string();
                                        let message = current_event
                                            .message
                                            .as_deref()
                                            .unwrap_or("")
                                            .trim();
                                        if let Err(e) = Self::maybe_handle_server_join_logging(
											instance_id,
											&timestamp,
											message,
                                        ).await {
                                            tracing::error!("Failed to handle server join logging: {e}");
                                        }
                                    }

                                    Self::emit_log4j_event(
                                        instance_id,
                                        process_uuid,
                                        &current_event,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(Event::Text(mut e)) => {
                        if in_message || in_throwable {
                            if let Ok(text) = e.xml_content() {
                                current_content.push_str(&text);
                            }
                        } else if !in_event
                            && !e.inplace_trim_end()
                            && !e.inplace_trim_start()
                            && let Ok(text) = e.xml_content()
                        {
                            if let Err(e) = Process::append_to_log_file(
                                &log_path,
                                &format!("{text}\n"),
                            ) {
                                tracing::error!(
                                    "Failed to write to log file: {}",
                                    e
                                );
                            }
                            Self::emit_legacy_log(
                                instance_id,
                                process_uuid,
                                &text,
                            );
                        }
                    }
                    Ok(Event::CData(e)) => {
                        if (in_message || in_throwable)
                            && let Ok(text) = e.xml_content()
                        {
                            current_content.push_str(&text);
                        }
                    }
                    _ => (),
                }

                buf.clear();
            }
        } else {
            let mut line = String::new();

            while let Ok(bytes_read) = buf_reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break; // End of stream
                }

                if !line.is_empty() {
                    if let Err(e) = Self::append_to_log_file(&log_path, &line) {
                        tracing::warn!("Failed to write to log file: {}", e);
                    }
                    Self::emit_legacy_log(
                        instance_id,
                        process_uuid,
                        line.trim_ascii_end(),
                    );
                    if let Err(e) = Self::maybe_handle_old_server_join_logging(
                        instance_id,
                        line.trim_ascii_end(),
                    )
                    .await
                    {
                        tracing::error!(
                            "Failed to handle old server join logging: {e}"
                        );
                    }
                }

                line.clear();
            }
        }
    }

    fn format_timestamp(timestamp_millis: Option<i64>) -> String {
        if let Some(timestamp_val) = timestamp_millis {
            let datetime_utc = if timestamp_val > i32::MAX as i64 {
                let secs = timestamp_val / 1000;
                let nsecs = ((timestamp_val % 1000) * 1_000_000) as u32;

                chrono::DateTime::<Utc>::from_timestamp(secs, nsecs)
                    .unwrap_or_default()
            } else {
                chrono::DateTime::<Utc>::from_timestamp_secs(timestamp_val)
                    .unwrap_or_default()
            };

            let datetime_local = datetime_utc.with_timezone(&chrono::Local);
            format!("[{}]", datetime_local.format("%H:%M:%S"))
        } else {
            "[??:??:??]".to_string()
        }
    }

    /// The same event with the session id taken out of what it says.
    fn without_session_id(event: &Log4jEvent) -> Log4jEvent {
        let censor = |text: &Option<String>| {
            text.as_ref()
                .map(|text| crate::api::logs::censor_session_ids(text.clone()))
        };

        Log4jEvent {
            message: censor(&event.message),
            throwable: censor(&event.throwable),
            ..event.clone()
        }
    }

    fn format_log4j_entry(event: &Log4jEvent) -> Option<String> {
        let message = event.message.as_ref()?;
        let thread = event.thread_name.as_deref().unwrap_or("");
        let level = event.level.as_deref().unwrap_or("");
        let logger = event.logger_name.as_deref().unwrap_or("");
        let formatted_time = Self::format_timestamp(event.timestamp_millis);

        Some(format!(
            "{} [{}] [{}{}]: {}\n",
            formatted_time,
            thread,
            if !logger.is_empty() {
                format!("{logger}/")
            } else {
                String::new()
            },
            level,
            message.trim()
        ))
    }

    fn emit_log4j_event(
        instance_id: &str,
        process_uuid: &str,
        event: &Log4jEvent,
    ) {
        // Before anything is kept or shown: what the game says about its own
        // session is a working credential, and it belongs in no buffer, no file
        // and on no screen somebody is about to screenshot.
        let event = &Self::without_session_id(event);

        if let Some(formatted) = Self::format_log4j_entry(event) {
            push_log_line(process_uuid, formatted.trim_end().to_string());
        }
        if let Some(ref throwable) = event.throwable {
            for line in throwable.lines().filter(|l| !l.is_empty()) {
                push_log_line(process_uuid, line.to_string());
            }
        }

        #[cfg(feature = "tauri")]
        {
            let event_state = crate::EventState::get();
            let _ = event_state.send(crate::event::AppEvent::Log(LogPayload {
                instance_id: instance_id.to_string(),
                process_uuid: process_uuid.to_string(),
                event: LogEvent::Log4j(event.clone()),
            }));
        }
        #[cfg(not(feature = "tauri"))]
        {
            let _ = (instance_id, process_uuid, event);
        }
    }

    fn emit_legacy_log(instance_id: &str, process_uuid: &str, message: &str) {
        let message = crate::api::logs::censor_session_ids(message.to_string());

        push_log_line(process_uuid, message.clone());

        #[cfg(feature = "tauri")]
        {
            let event_state = crate::EventState::get();
            let _ = event_state.send(crate::event::AppEvent::Log(LogPayload {
                instance_id: instance_id.to_string(),
                process_uuid: process_uuid.to_string(),
                event: LogEvent::Legacy { message },
            }));
        }
        #[cfg(not(feature = "tauri"))]
        {
            let _ = (instance_id, process_uuid, message);
        }
    }

    fn append_to_log_file(
        path: impl AsRef<Path>,
        line: &str,
    ) -> std::io::Result<()> {
        let mut file =
            OpenOptions::new().append(true).create(true).open(path)?;

        // The game writes its session id into its own output on every launch,
        // and that is a working credential. It is taken out on the way past
        // rather than on the way out to a screen, so that the copy kept on disk
        // does not hold one either — whoever opens that file, or attaches it
        // somewhere, gets a log and not an account.
        file.write_all(
            crate::api::logs::censor_session_ids(line.to_string()).as_bytes(),
        )?;
        Ok(())
    }

    async fn maybe_handle_server_join_logging(
        instance_id: &str,
        timestamp: &str,
        message: &str,
    ) -> crate::Result<()> {
        let timestamp = timestamp
            .parse::<i64>()
            .map(|x| x / 1000)
            .map_err(|x| {
                crate::ErrorKind::OtherError(format!(
                    "Failed to parse timestamp: {x}"
                ))
            })
            .and_then(|x| {
                Utc.timestamp_opt(x, 0).single().ok_or_else(|| {
                    crate::ErrorKind::OtherError(
                        "Failed to convert timestamp to DateTime".to_string(),
                    )
                })
            })?;
        Self::parse_and_insert_server_join(instance_id, message, timestamp)
            .await
    }

    async fn maybe_handle_old_server_join_logging(
        instance_id: &str,
        line: &str,
    ) -> crate::Result<()> {
        if let Some((timestamp, message)) = line.split_once(" [CLIENT] [INFO] ")
        {
            let timestamp =
                NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S")?
                    .and_local_timezone(chrono::Local)
                    .map(|x| x.to_utc())
                    .single()
                    .unwrap_or_else(Utc::now);
            Self::parse_and_insert_server_join(instance_id, message, timestamp)
                .await
        } else {
            Self::parse_and_insert_server_join(instance_id, line, Utc::now())
                .await
        }
    }

    async fn parse_and_insert_server_join(
        instance_id: &str,
        message: &str,
        timestamp: DateTime<Utc>,
    ) -> crate::Result<()> {
        let Some(host_port_string) = message.strip_prefix("Connecting to ")
        else {
            return Ok(());
        };
        let Some((host, port_string)) = host_port_string.rsplit_once(", ")
        else {
            return Ok(());
        };
        let Some(port) = port_string.parse::<u16>().ok() else {
            return Ok(());
        };

        let state = crate::State::get().await?;
        crate::state::server_join_log::JoinLogEntry {
            instance_id: instance_id.to_owned(),
            host: host.to_string(),
            port,
            join_time: timestamp,
        }
        .upsert(&state.pool)
        .await?;
        {
            let instance_id = instance_id.to_owned();
            let host = host.to_owned();
            tokio::spawn(async move {
                let _ = emit_instance(
                    &instance_id,
                    InstancePayloadType::ServerJoined {
                        host,
                        port,
                        timestamp: timestamp.to_rfc3339(),
                    },
                )
                .await;
            });
        }

        Ok(())
    }

    // Spawns a new child process and inserts it into the hashmap
    // Also, as the process ends, it spawns the follow-up process if it exists
    // By convention, ExitStatus is last command's exit status, and we exit on the first non-zero exit status
    async fn sequential_process_manager(
        instance_id: String,
        instance_path: String,
        post_exit_command: Option<String>,
        post_exit_env_vars: Vec<(String, String)>,
        uuid: Uuid,
        persisted_process: Option<(i64, i64)>,
    ) -> crate::Result<()> {
        async fn update_playtime(
            last_updated_playtime: &mut Instant,
            instance_id: &str,
            force_update: bool,
        ) {
            let elapsed = last_updated_playtime.elapsed().as_secs();
            if elapsed == 0 || (!force_update && elapsed < 60) {
                return;
            }

            let state = match crate::State::get().await {
                Ok(state) => state,
                Err(e) => {
                    tracing::warn!(
                        "Failed to get state for playtime update on instance {}: {}",
                        instance_id,
                        e
                    );
                    return;
                }
            };
            if let Err(e) =
                crate::state::instances::commands::add_instance_recent_playtime(
                    instance_id,
                    elapsed,
                    &state.pool,
                )
                .await
            {
                tracing::warn!(
                    "Failed to update playtime for instance {}: {}",
                    instance_id,
                    e
                );
            }
            *last_updated_playtime = Instant::now();
        }

        // Wait on current Minecraft Child
        let mc_exit_status;
        let mut last_updated_playtime = Instant::now();

        let state = crate::State::get().await?;
        loop {
            if let Some(process) = state.process_manager.try_wait(uuid)? {
                if let Some(t) = process {
                    mc_exit_status = t;
                    break;
                }
            } else {
                mc_exit_status = ExitStatus::default();
                break;
            }

            // sleep for 10ms
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // Auto-update playtime every minute
            update_playtime(&mut last_updated_playtime, &instance_id, false)
                .await;
        }

        state.process_manager.remove(uuid);
        clear_persisted_process(&state, persisted_process).await;
        emit_process(
            &instance_id,
            uuid,
            ProcessPayloadType::Finished,
            "Exited process",
        )
        .await?;

        // Now fully complete- update playtime one last time
        update_playtime(&mut last_updated_playtime, &instance_id, true).await;

        let reconcile_instance_id = instance_id.clone();
        tokio::spawn(async move {
            if let Err(error) =
                crate::api::instance::reconcile_instance_synced_options(
                    &reconcile_instance_id,
                )
                .await
            {
                tracing::warn!(
                    "Failed to reconcile synced options after Minecraft exited for {reconcile_instance_id}: {error}"
                );
            }
        });

        // Publish play time update
        // Allow failure, it will be stored locally and sent next time
        // Sent in another thread as first call may take a couple seconds and hold up process ending
        let playtime_instance_id = instance_id.clone();
        tokio::spawn(async move {
            if let Err(e) =
                crate::api::instance::try_update_playtime_by_instance_id(
                    &playtime_instance_id,
                )
                .await
            {
                tracing::warn!(
                    "Failed to update playtime for instance {}: {}",
                    playtime_instance_id,
                    e
                );
            }
        });

        let logs_folder = state.directories.instance_logs_dir(&instance_path);
        let log_path = logs_folder.join(LAUNCHER_LOG_PATH);

        if log_path.exists()
            && let Err(e) = Process::append_to_log_file(
                &log_path,
                &format!("\n# Process exited with status: {mc_exit_status}\n"),
            )
        {
            tracing::warn!("Failed to write exit status to log file: {}", e);
        }

        let _ = state.discord_rpc.clear_to_default(true).await;

        let _ = state.friends_socket.update_status(None).await;

        // If in tauri, window should show itself again after process exists if it was hidden
        #[cfg(feature = "tauri")]
        {
            let window = crate::EventState::get_main_window().await?;
            if let Some(window) = window {
                window.unminimize()?;
                window.set_focus()?;
            }
        }

        if mc_exit_status.success() {
            // We do not wait on the post exist command to finish running! We let it spawn + run on its own.
            // This behaviour may be changed in the future
            if let Some(hook) = post_exit_command {
                let mut cmd = shlex::split(&hook)
                    .ok_or_else(|| {
                        crate::ErrorKind::LauncherError(format!(
                            "Invalid post-exit command: {hook}",
                        ))
                    })?
                    .into_iter();

                if let Some(command) = cmd.next() {
                    let mut command = Command::new(command);
                    command.args(cmd).envs(post_exit_env_vars).current_dir(
                        state.directories.instances_dir().join(&instance_path),
                    );
                    command.spawn().map_err(IOError::from)?;
                }
            }
        }

        Ok(())
    }
}
