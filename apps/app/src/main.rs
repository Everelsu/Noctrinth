#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#![recursion_limit = "256"]

use native_dialog::{DialogBuilder, MessageLevel};
use std::env;
use std::sync::atomic::Ordering;
use tauri::{Listener, Manager};
use tauri_plugin_fs::FsExt;
use theseus::prelude::*;

mod api;
mod error;

/// How long the exit path may spend landing a debounced skin change before it
/// gives up. `app.run`'s callback runs on the window event-loop thread, so this
/// is the longest the window can stop responding on the way out.
const EXIT_SKIN_FLUSH_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(5);

/// Routes panics into the session log before the process goes away.
///
/// Release builds are `panic = "abort"`, so a panic on any thread — including a
/// background task nobody is awaiting — takes the whole app down immediately.
/// The default hook prints to stderr, and the Windows release binary is built
/// with `windows_subsystem = "windows"`, so it has no stderr to print to. The
/// result is an app that vanishes leaving nothing in the log but whatever line
/// happened to be written just before, which says nothing about the cause.
///
/// The log layer writes straight to the file with no buffering in front of it,
/// so a line emitted here reaches disk before `abort`.
fn install_panic_logger() {
    let previous_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        let location = info.location().map_or_else(
            || "an unknown location".to_string(),
            |location| location.to_string(),
        );

        // `&str` for `panic!("literal")`, `String` for a formatted message.
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<non-string panic payload>");

        // `strip = true` leaves release backtraces symbol-less, and this only
        // captures at all when RUST_BACKTRACE is set, but the panic message and
        // its file:line come from the binary either way.
        tracing::error!(
            "Panicked at {location}: {message}\n{}",
            std::backtrace::Backtrace::capture()
        );

        previous_hook(info);
    }));
}

#[cfg(target_os = "macos")]
mod macos;

#[cfg(feature = "updater")]
mod updater_impl;
#[cfg(not(feature = "updater"))]
mod updater_impl_noop;

// Should be called in launcher initialization
#[tracing::instrument(skip_all)]
#[tauri::command]
async fn initialize_state(
    app: tauri::AppHandle,
    events: tauri::ipc::Channel<tauri::ipc::InvokeResponseBody>,
) -> api::Result<()> {
    tracing::info!("Initializing app event state...");
    theseus::EventState::init(app.clone(), events).await?;

    tracing::info!("Initializing app state...");
    State::init(app.config().identifier.clone()).await?;

    let state = State::get().await?;
    app.asset_protocol_scope()
        .allow_directory(state.directories.caches_dir(), true)?;
    app.asset_protocol_scope()
        .allow_directory(state.directories.caches_dir().join("icons"), true)?;
    app.fs_scope()
        .allow_directory(state.directories.instances_dir(), true)?;

    Ok(())
}

// Should be call once Vue has mounted the app
#[tracing::instrument(skip_all)]
#[tauri::command]
fn show_window(app: tauri::AppHandle) {
    // The frontend fires this from `onMounted` while its `initialize_state`
    // call is still in flight, so the two race. Panicking here aborts the whole
    // process (release builds are `panic = "abort"`) partway through init, and
    // the resulting log just stops mid-startup with no reason recorded — so
    // every failure below is reported rather than thrown.
    let Some(win) = app.get_window("main") else {
        tracing::error!(
            "Cannot display application window: there is no window labelled `main`"
        );
        return;
    };

    if let Err(error) = win.show() {
        tracing::error!("Cannot display application window: {error}");

        if let Err(dialog_error) = DialogBuilder::message()
            .set_level(MessageLevel::Error)
            .set_title("Initialization error")
            .set_text(format!(
                "Cannot display application window due to an error:\n{error}"
            ))
            .alert()
            .show()
        {
            tracing::error!(
                "Failed to show the initialization error dialog: {dialog_error}"
            );
        }

        // The window is what the app is, so there is nothing left to do — but
        // leave through the normal exit path instead of aborting.
        app.exit(1);
    } else {
        let _ = win.set_focus();
    }
}

#[tauri::command]
fn is_dev() -> bool {
    cfg!(debug_assertions)
}

#[tauri::command]
fn are_updates_enabled() -> bool {
    cfg!(feature = "updater")
        && env::var("MODRINTH_EXTERNAL_UPDATE_PROVIDER").is_err()
}

#[cfg(feature = "updater")]
pub use updater_impl::*;

#[cfg(not(feature = "updater"))]
pub use updater_impl_noop::*;

// Toggles decorations
#[tauri::command]
async fn toggle_decorations(b: bool, window: tauri::Window) -> api::Result<()> {
    window.set_decorations(b).map_err(|e| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Failed to toggle decorations: {e}"
        )))
    })?;
    Ok(())
}

#[tauri::command]
fn restart_app(app: tauri::AppHandle) {
    app.restart();
}

#[tauri::command]
async fn set_restart_after_pending_update(
    should_restart: bool,
) -> api::Result<()> {
    let state = State::get().await?;
    state
        .restart_after_pending_update
        .store(should_restart, Ordering::Relaxed);
    Ok(())
}

// if Tauri app is called with arguments, then those arguments will be treated as commands
// ie: deep links or filepaths for .mrpacks
/// Which build of this version this is, baked in at compile time.
///
/// It comes from the build number VERSION carries after a `+`, as semver build
/// metadata: zero for a version as it first went out, and one more for every
/// rebuild published under that same number.
#[cfg(feature = "updater")]
fn noctrinth_patch() -> u32 {
    env!("NOCTRINTH_PATCH").trim().parse().unwrap_or(0)
}

/// The same number, as the updater manifest gives it.
///
/// It rides in the manifest's notes, which the app itself never shows — the
/// changelog it displays is the one it shipped with. Anything else, including
/// a manifest from before this existed, counts as the version's first build.
#[cfg(feature = "updater")]
fn manifest_patch(notes: Option<&str>) -> u32 {
    const MARKER: &str = "noctrinth-patch:";

    notes
        .and_then(|notes| notes.split(MARKER).nth(1))
        .map(|rest| rest.trim_start())
        .and_then(|rest| {
            let digits: String =
                rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .unwrap_or(0)
}

/// Stack each async worker gets, in place of Rust's 2 MiB default for a thread
/// it spawns itself.
///
/// Tauri builds its own tokio runtime with `Runtime::new()`, which leaves the
/// worker threads on that default. The install path polls several large
/// futures together — `download_minecraft` joins the client, the log config,
/// the assets and the libraries in one `try_join!` — and an unoptimised build
/// lays every nested future out inline, so a big enough version runs that off
/// the end of the stack: a `cargo tauri dev` run dies with
/// STATUS_STACK_OVERFLOW on `tokio-rt-worker` where a release build survives.
///
/// The `/STACK` link flag in `.cargo/config.toml` does not cover this. That
/// sets the PE header's default, which Windows applies only to threads created
/// without a size of their own, and Rust always passes one explicitly — so it
/// raises the main thread and nothing else.
const ASYNC_WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;

/// Hands Tauri a runtime whose workers have room to poll the install futures.
///
/// Must run before anything touches `tauri::async_runtime`, which builds the
/// default runtime on first use and then refuses to be replaced.
fn install_async_runtime() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(ASYNC_WORKER_STACK_SIZE)
        .build()
        .expect("the app's async runtime should be buildable");

    // Tauri keeps the handle and not the runtime, so this one has to outlive
    // the call. It lives as long as the process either way.
    let runtime: &'static tokio::runtime::Runtime =
        Box::leak(Box::new(runtime));
    tauri::async_runtime::set(runtime.handle().clone());
}

fn main() {
    install_async_runtime();

    #[cfg(feature = "export-app-events")]
    theseus::export_app_event_bindings(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../app-frontend/src/generated/app-events"),
    )
    .expect("failed to export app event TypeScript bindings");

    /*
        tracing is set basd on the environment variable RUST_LOG=xxx, depending on the amount of logs to show
            ERROR > WARN > INFO > DEBUG > TRACE
        eg. RUST_LOG=info will show info, warn, and error logs
            RUST_LOG="theseus=trace" will show *all* messages but from theseus only (and not dependencies using similar crates)
            RUST_LOG="theseus=trace" will show *all* messages but from theseus only (and not dependencies using similar crates)

        Error messages returned to Tauri will display as traced error logs if they return an error.
        This will also include an attached span trace if the error is from a tracing error, and the level is set to info, debug, or trace

        on unix:
            RUST_LOG="theseus=trace" {run command}

    */

    let tauri_context = tauri::generate_context!();

    let _log_guard = theseus::start_logger(&tauri_context.config().identifier);

    // Directly after the logger, so the hook has somewhere to write.
    install_panic_logger();

    tracing::info!("Initialized tracing subscriber. Loading Modrinth App!");

    let mut builder = tauri::Builder::default();

    #[cfg(target_os = "macos")]
    {
        builder = builder
            .menu(|app| macos::menu::create(app))
            .on_menu_event(macos::menu::handle_event);
    }

    #[cfg(feature = "updater")]
    {
        use tauri_plugin_http::reqwest::header::{HeaderValue, USER_AGENT};
        use theseus::launcher_user_agent;
        builder = builder.plugin(
            tauri_plugin_updater::Builder::new()
                .header(
                    USER_AGENT,
                    HeaderValue::from_str(&launcher_user_agent()).unwrap(),
                )
                .unwrap()
                .default_version_comparator(|current, release| {
                    if release.version != current {
                        return release.version > current;
                    }

                    // The same version, rebuilt: a micropatch. Which of the two
                    // is newer is the number the release carries, not its date
                    // — a manifest is written minutes after the binaries it
                    // describes, and a build must never offer itself an update.
                    manifest_patch(release.notes.as_deref()) > noctrinth_patch()
                })
                .build(),
        );
    }

    builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(payload) = args.get(1) {
                tracing::info!("Handling command-line deep link");
                let payload = payload.clone();
                tauri::async_runtime::spawn(api::utils::handle_command(
                    payload,
                ));
            }

            if let Some(win) = app.get_window("main") {
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_filename("app-window-state.json")
                .with_denylist(&["signin"])
                // Use *only* POSITION and SIZE state flags, because saving VISIBLE causes the `visible: false` to not take effect
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED,
                )
                .build(),
        )
        .setup(|app| {
            // Claim the configured URL schemes on every launch. The installer
            // registers them once, which leaves two gaps: an existing install
            // never picks up a scheme added in a later version, and `modrinth://`
            // is shared with Modrinth App, where whoever registered last wins.
            // Doing it here means launching Noctrinth is enough to take the
            // schemes back. macOS registers them from the bundle's Info.plist
            // and rejects this call, so it is skipped there.
            #[cfg(not(target_os = "macos"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;

                if let Err(e) = app.deep_link().register_all() {
                    tracing::warn!("Failed to register deep link schemes: {e}");
                }
            }

            #[cfg(target_os = "macos")]
            {
                let payload = macos::deep_link::get_or_init_payload(app);

                let mtx_copy = payload.payload;
                app.listen("deep-link://new-url", move |url| {
                    let mtx_copy_copy = mtx_copy.clone();
                    let request = url.payload().to_owned();

                    let actual_request =
                        serde_json::from_str::<Vec<String>>(&request)
                            .ok()
                            .map(|mut x| x.remove(0))
                            .unwrap_or(request);

                    tauri::async_runtime::spawn(async move {
                        tracing::info!("Handling macOS deep link");

                        let mut payload = mtx_copy_copy.lock().await;
                        if payload.is_none() {
                            *payload = Some(actual_request.clone());
                        }

                        let _ =
                            api::utils::handle_command(actual_request).await;
                    });
                });
            };

            #[cfg(not(target_os = "macos"))]
            app.listen("deep-link://new-url", |url| {
                let payload = url.payload().to_owned();
                tracing::info!("Handling deep link");
                tauri::async_runtime::spawn(api::utils::handle_command(
                    payload,
                ));
            });

            #[cfg(not(target_os = "linux"))]
            if let Some(window) = app.get_window("main")
                && let Err(e) = window.set_shadow(true)
            {
                tracing::warn!("Failed to set window shadow: {e}");
            }

            Ok(())
        });

    builder = builder
        .plugin(api::auth::init())
        .plugin(api::ely_auth::init())
        .plugin(api::mr_auth::init())
        .plugin(api::onboarding_checklist::init())
        .plugin(api::import::init())
        .plugin(api::install::init())
        .plugin(api::instance::init())
        .plugin(api::logs::init())
        .plugin(api::jre::init())
        .plugin(api::metadata::init())
        .plugin(api::minecraft_skins::init())
        .plugin(api::process::init())
        .plugin(api::reports::init())
        .plugin(api::settings::init())
        .plugin(api::shortcuts::init())
        .plugin(api::tags::init())
        .plugin(api::users::init())
        .plugin(api::utils::init())
        .plugin(api::window_icon::init())
        .plugin(api::cache::init())
        .plugin(api::files::init())
        .plugin(api::ads::init())
        .plugin(api::friends::init())
        .plugin(api::worlds::init())
        .manage(PendingUpdateData::default())
        .invoke_handler(tauri::generate_handler![
            initialize_state,
            is_dev,
            are_updates_enabled,
            get_update_size,
            enqueue_update_for_installation,
            remove_enqueued_update,
            set_restart_after_pending_update,
            toggle_decorations,
            show_window,
            restart_app,
        ]);

    tracing::info!("Initializing app...");
    let app = builder.build(tauri_context);

    match app {
        Ok(app) => {
            app.run(|app, event| {
                #[cfg(not(any(feature = "updater", target_os = "macos")))]
                let _ = app;

                // Blocking here blocks the window event loop, so the flush is
                // given a deadline: an unreachable skin server used to be able
                // to park this thread for the rest of the process's life, which
                // Windows shows as a window that never repaints or closes.
                if matches!(&event, tauri::RunEvent::ExitRequested { .. }) {
                    // The timeout is built inside the block rather than passed
                    // into it: a Tokio timer can only be created from within a
                    // runtime, and this event arrives on the window event loop,
                    // which is not one. Built outside, closing the app panicked
                    // with "there is no reactor running".
                    match tauri::async_runtime::block_on(async {
                        tokio::time::timeout(
                            EXIT_SKIN_FLUSH_TIMEOUT,
                            theseus::minecraft_skins::flush_pending_skin_change(),
                        )
                        .await
                    }) {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => tracing::warn!(
                            "Failed to flush pending Minecraft skin change before exit: {error}"
                        ),
                        Err(_) => tracing::warn!(
                            "Timed out after {EXIT_SKIN_FLUSH_TIMEOUT:?} flushing the pending Minecraft skin change before exit"
                        ),
                    }
                }

                #[cfg(feature = "updater")]
                if matches!(&event, tauri::RunEvent::Exit) {
                    let update_data = app.state::<PendingUpdateData>().inner();
                    let should_restart = State::get_if_initialized()
                        .map(|s| {
                            s.restart_after_pending_update.load(Ordering::Relaxed)
                        })
                        .unwrap_or(false);
                    if let Some((update, data)) = &*update_data.0.lock().unwrap()
                    {
                        fn set_changelog_toast(version: Option<String>) {
                            let toast_result: theseus::Result<()> = tauri::async_runtime::block_on(async move {
                                let mut settings = settings::get().await?;
                                settings.pending_update_toast_for_version = version;
                                settings::set(settings).await?;
                                Ok(())
                            });
                            if let Err(e) = toast_result {
                                tracing::warn!(
                                    "Failed to set pending_update_toast: {e}"
                                )
                            }
                        }

                        set_changelog_toast(Some(update.version.clone()));
                        let update = if should_restart {
                            (**update).clone()
                        } else {
                            (**update).clone().restart_after_install(false)
                        };
                        match update.install(data) {
                            Ok(()) => {
                                if should_restart {
                                    tracing::info!(
                                        "Pending update installed successfully (version {}); restarting because user requested reload",
                                        update.version
                                    );
                                    app.restart();
                                } else {
                                    tracing::info!(
                                        "Pending update installed successfully (version {}); exiting without relaunch (user did not request reload)",
                                        update.version
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Pending update install failed (version {}): {e}",
                                    update.version
                                );
                                set_changelog_toast(None);

                                // A bare "Permission denied (os error 13)" tells
                                // nobody what to do about it — the install
                                // location is what actually matters here.
                                let text = match self_update_blocker() {
                                    Some(reason) => format!("Failed to install update.\n\n{reason}"),
                                    None => format!("Failed to install update due to an error:\n{e}"),
                                };

                                DialogBuilder::message()
                                    .set_level(MessageLevel::Error)
                                    .set_title("Update error")
                                    .set_text(text)
                                    .alert()
                                    .show()
                                    .unwrap();
                            }
                        }
                    }
                }
                #[cfg(target_os = "macos")]
                if let tauri::RunEvent::Opened { urls } = event {
                    tracing::info!("Handling webview open {urls:?}");

                    let file = urls
                        .into_iter()
                        .find_map(|url| url.to_file_path().ok());

                    if let Some(file) = file {
                        let payload =
                            macos::deep_link::get_or_init_payload(app);

                        let mtx_copy = payload.payload;
                        let request = file.to_string_lossy().to_string();
                        tauri::async_runtime::spawn(async move {
                            let mut payload = mtx_copy.lock().await;
                            if payload.is_none() {
                                *payload = Some(request.clone());
                            }

                            let _ = api::utils::handle_command(request).await;
                        });
                    }
                }
            });
        }
        Err(e) => {
            tracing::error!("Error while running tauri application: {:?}", e);

            #[cfg(target_os = "windows")]
            {
                // tauri doesn't expose runtime errors, so matching a string representation seems like the only solution
                if format!("{e:?}").contains(
                    "Runtime(CreateWebview(WebView2Error(WindowsError",
                ) {
                    DialogBuilder::message()
                        .set_level(MessageLevel::Error)
                        .set_title("Initialization error")
                        .set_text("Your Microsoft Edge WebView2 installation is corrupt.\n\nMicrosoft Edge WebView2 is required to run Modrinth App.\n\nLearn how to repair it at https://support.modrinth.com/en/articles/8797765-corrupted-microsoft-edge-webview2-installation")
                        .alert()
                        .show()
                        .unwrap();

                    panic!("webview2 initialization failed")
                }
            }

            DialogBuilder::message()
                .set_level(MessageLevel::Error)
                .set_title("Initialization error")
                .set_text(format!(
                    "Cannot initialize application due to an error:\n{e:?}"
                ))
                .alert()
                .show()
                .unwrap();

            panic!("{1}: {:?}", e, "error while running tauri application")
        }
    }
}
