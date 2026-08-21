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

    /// Winsock codes Rust has no `ErrorKind` for, so they arrive as
    /// `Uncategorized` and would be logged as faults.
    ///
    /// 11001-11004 are the resolver having no answer — no such host, try again,
    /// no recovery, no data — which is what a machine with no DNS looks like.
    /// The 100xx codes are the network being down, unreachable or refusing,
    /// some of which Rust maps and some of which it does not, depending on the
    /// version it was built with.
    const TRANSIENT_OS_ERRORS: &[i32] = &[
        10050, 10051, 10060, 10061, 10064, 10065, 11001, 11002, 11003, 11004,
    ];

    match err.raw.as_ref() {
        theseus::ErrorKind::StdIOError(io) => {
            matches!(
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
            ) || io
                .raw_os_error()
                .is_some_and(|code| TRANSIENT_OS_ERRORS.contains(&code))
        }
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

#[cfg(test)]
mod tests {
    use super::is_transient_network_error;
    use std::io;

    fn io_error(code: i32) -> theseus::Error {
        theseus::ErrorKind::StdIOError(io::Error::from_raw_os_error(code))
            .as_error()
    }

    /// Windows says "no such host" as a Winsock code Rust does not map, so it
    /// arrives as `Uncategorized` — which used to be logged as a fault every
    /// time a poll ran without DNS.
    #[test]
    fn windows_dns_failures_are_transient() {
        for code in [11001, 11002, 11003, 11004] {
            assert!(
                is_transient_network_error(&io_error(code)),
                "os error {code} should be treated as the network, not a fault"
            );
        }
    }

    #[test]
    fn refused_and_unreachable_are_transient() {
        for code in [10050, 10051, 10060, 10061, 10065] {
            assert!(is_transient_network_error(&io_error(code)));
        }
    }

    /// A disk that is full is the launcher's problem, and has to keep shouting.
    #[test]
    fn other_io_errors_are_still_faults() {
        assert!(!is_transient_network_error(
            &theseus::ErrorKind::StdIOError(io::Error::other("disk full"))
                .as_error()
        ));
    }
}
