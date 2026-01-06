use std::{
    convert::Infallible,
    fmt::{self, Display, Formatter},
    fs::OpenOptions,
    path::PathBuf,
    str::FromStr,
};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, registry::LookupSpan, util::SubscriberInitExt,
    Layer,
};

// SAFETY: Configuration file needs many bools.
#[allow(clippy::struct_excessive_bools)]
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogConfig {
    #[serde(default = "LogConfig::default_file_path")]
    pub file_path: Option<PathBuf>,

    #[serde(default = "LogConfig::default_emit_journald")]
    pub emit_journald: bool,

    #[serde(default = "LogConfig::default_emit_stdout")]
    pub emit_stdout: bool,

    #[serde(default = "LogConfig::default_emit_stderr")]
    pub emit_stderr: bool,

    #[serde(default = "LogConfig::default_log_filters")]
    pub log_filters: String,

    #[serde(default = "LogConfig::default_log_formatter")]
    #[serde_as(as = "DisplayFromStr")]
    pub formatter: LogFormatter,

    // Display function latency in logs
    #[serde(default = "LogConfig::default_show_fn_latency")]
    pub show_fn_latency: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            file_path: Self::default_file_path(),
            emit_journald: Self::default_emit_journald(),
            emit_stdout: Self::default_emit_stdout(),
            emit_stderr: Self::default_emit_stderr(),
            log_filters: Self::default_log_filters(),
            formatter: Self::default_log_formatter(),
            show_fn_latency: Self::default_show_fn_latency(),
        }
    }
}

impl LogConfig {
    #[inline]
    #[must_use]
    pub fn default_log_filters() -> String { "info,zpl-client=info".to_string() }

    #[inline]
    #[must_use]
    pub const fn default_file_path() -> Option<PathBuf> { None }

    #[inline]
    #[must_use]
    pub const fn default_emit_journald() -> bool { false }

    #[inline]
    #[must_use]
    pub const fn default_emit_stdout() -> bool { true }

    #[inline]
    #[must_use]
    pub const fn default_emit_stderr() -> bool { false }

    #[inline]
    #[must_use]
    pub const fn default_log_formatter() -> LogFormatter { LogFormatter::Pretty }

    #[inline]
    #[must_use]
    pub const fn default_show_fn_latency() -> bool { false }

    pub fn registry(&self) {
        let Self {
            emit_journald,
            file_path,
            emit_stdout,
            emit_stderr,
            log_filters,
            formatter,
            show_fn_latency,
        } = self;

        let filter_layer = tracing_subscriber::filter::EnvFilter::new(log_filters.as_str());

        // Display function latency in logs, for example:
        // `XXX_FUNCTION close, time.busy: 37.5µs, time.idle: 2.01s`.
        let fmt_span = if *show_fn_latency { FmtSpan::CLOSE } else { FmtSpan::NONE };

        tracing_subscriber::registry()
            .with(filter_layer)
            .with(emit_journald.then(|| LogDriver::Journald.layer(fmt_span.clone())))
            .with(
                file_path
                    .clone()
                    .map(|path| LogDriver::File(path, formatter.clone()).layer(fmt_span.clone())),
            )
            .with(emit_stdout.then(|| LogDriver::Stdout(formatter.clone()).layer(fmt_span.clone())))
            .with(emit_stderr.then(|| LogDriver::Stderr(formatter.clone()).layer(fmt_span)))
            .init();
    }
}

#[derive(Clone, Debug)]
enum LogDriver {
    Stdout(LogFormatter),
    Stderr(LogFormatter),
    Journald,
    File(PathBuf, LogFormatter),
}

impl LogDriver {
    #[allow(clippy::type_repetition_in_bounds)]
    fn layer<S>(self, span_events: FmtSpan) -> Option<Box<dyn Layer<S> + Send + Sync + 'static>>
    where
        S: tracing::Subscriber,
        for<'a> S: LookupSpan<'a>,
    {
        // Shared configuration regardless of where logs are output to.
        let fmt = tracing_subscriber::fmt::layer()
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_target(true)
            .with_span_events(span_events);

        match self {
            Self::Stdout(formatter) => match formatter {
                LogFormatter::Pretty => Some(fmt.with_writer(std::io::stdout).pretty().boxed()),
                LogFormatter::Json => {
                    Some(fmt.with_writer(std::io::stdout).json().flatten_event(true).boxed())
                }
            },
            Self::Stderr(formatter) => match formatter {
                LogFormatter::Pretty => Some(fmt.with_writer(std::io::stderr).pretty().boxed()),
                LogFormatter::Json => {
                    Some(fmt.with_writer(std::io::stderr).json().flatten_event(true).boxed())
                }
            },
            Self::File(path, formatter) => {
                let file = OpenOptions::new().create(true).append(true).open(path).ok()?;
                match formatter {
                    LogFormatter::Pretty => Some(fmt.with_writer(file).pretty().boxed()),
                    LogFormatter::Json => {
                        Some(fmt.with_writer(file).json().flatten_event(true).boxed())
                    }
                }
            }
            Self::Journald => Some(tracing_journald::layer().ok()?.boxed()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum LogFormatter {
    Pretty,
    Json,
}

impl FromStr for LogFormatter {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            _ => Ok(Self::Pretty),
        }
    }
}

impl Display for LogFormatter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pretty => write!(f, "pretty"),
            Self::Json => write!(f, "json"),
        }
    }
}
