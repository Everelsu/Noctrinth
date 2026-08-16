use tracing_error::ExtractSpanTrace;

/// Whether an error is the network being the network.
///
/// Losing DNS or having a connection refused says nothing about the launcher —
/// it says the machine briefly had no route out, which happens on flaky Wi-Fi,
/// VPNs and captive portals. These arrive through background polling, so a
/// twenty-second outage can log well over a hundred lines and bury whatever
/// actually went wrong that session.
fn is_transient_network_error(err: &theseus::Error) -> bool {
    use std::io::ErrorKind;

    match err.raw.as_ref() {
        theseus::ErrorKind::StdIOError(io) => matches!(
            io.kind(),
            ErrorKind::NotFound
                | ErrorKind::ConnectionRefused
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::BrokenPipe
                | ErrorKind::TimedOut
                | ErrorKind::UnexpectedEof
                | ErrorKind::HostUnreachable
                | ErrorKind::NetworkUnreachable
                | ErrorKind::NetworkDown
        ),
        theseus::ErrorKind::FetchError(fetch) => {
            fetch.is_connect() || fetch.is_timeout()
        }
        _ => false,
    }
}

/// Logs an error on its way back to the frontend.
///
/// This runs from `Serialize`, so it fires for every error the UI is handed,
/// including ones it goes on to handle quietly. Transient network failures are
/// therefore logged as warnings: they are worth keeping, but they are not
/// faults, and at `ERROR` they drown the log.
pub fn display_tracing_error(err: &theseus::Error) {
    let span_trace = get_span_trace(err);

    if is_transient_network_error(err) {
        match span_trace {
            Some(span_trace) => {
                tracing::warn!(error = %err, span_trace = %span_trace, "Network request failed")
            }
            None => tracing::warn!(error = %err, "Network request failed"),
        }
        return;
    }

    match span_trace {
        Some(span_trace) => {
            tracing::error!(error = %err, span_trace = %span_trace);
        }
        None => {
            tracing::error!(error = %err);
        }
    }
}

pub fn get_span_trace<'a>(
    error: &'a (dyn std::error::Error + 'static),
) -> Option<&'a tracing_error::SpanTrace> {
    error.source().and_then(|e| e.span_trace())
}
