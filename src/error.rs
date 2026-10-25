use std::fmt;
use std::path::PathBuf;

/// Every failure names the file it happened to, so a batch run is diagnosable
/// from its output alone.
#[derive(Debug)]
pub enum Error {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Image {
        path: PathBuf,
        source: image::ImageError,
    },
    Usage(String),
    Exists(PathBuf),
    Animated {
        path: PathBuf,
        kind: &'static str,
    },
    TooLarge {
        path: PathBuf,
        pixels: u64,
        limit: u64,
    },
    Unsupported {
        path: PathBuf,
        what: String,
    },
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    pub fn image(path: impl Into<PathBuf>, source: image::ImageError) -> Self {
        Error::Image {
            path: path.into(),
            source,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "{}: {}", path.display(), source),
            Error::Image { path, source } => write!(f, "{}: {}", path.display(), source),
            Error::Usage(msg) => write!(f, "{msg}"),
            Error::Exists(path) => write!(
                f,
                "{}: output exists (pass --force to overwrite)",
                path.display()
            ),
            Error::Animated { path, kind } => write!(
                f,
                "{}: {kind} input is not supported, this tool handles still images only",
                path.display()
            ),
            Error::TooLarge {
                path,
                pixels,
                limit,
            } => write!(
                f,
                "{}: {} megapixels exceeds the {} megapixel limit (raise it with --max-pixels)",
                path.display(),
                pixels / 1_000_000,
                limit / 1_000_000
            ),
            Error::Unsupported { path, what } => write!(f, "{}: {}", path.display(), what),
        }
    }
}

impl std::error::Error for Error {}

impl From<Error> for String {
    fn from(e: Error) -> String {
        e.to_string()
    }
}
