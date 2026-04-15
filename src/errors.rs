use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum MxpmError {
    #[error("package not found: {name}")]
    PackageNotFound { name: String },

    #[error("failed to fetch index from {url}")]
    IndexFetch {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("failed to parse index")]
    IndexParse(#[from] serde_json::Error),

    #[error("failed to download package from {url}")]
    Download {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("download failed for {url}: HTTP {status}")]
    DownloadStatus { url: String, status: u16 },

    #[error("git clone failed for {url}: {message}")]
    GitClone { url: String, message: String },

    #[error("failed to extract archive")]
    Extraction(#[source] std::io::Error),

    #[error("failed to read config at {path}")]
    Config {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("could not determine Maxima user directory; set $MAXIMA_USERDIR")]
    MaximaUserDirNotFound,

    #[error("package '{name}' is already installed")]
    AlreadyInstalled { name: String },

    #[error("package '{name}' is not installed")]
    NotInstalled { name: String },

    #[error("unsupported index version: {version} (this CLI supports version 1)")]
    UnsupportedIndexVersion { version: u32 },

    #[error("hash mismatch for {url}: expected {expected}, got {actual}")]
    HashMismatch {
        url: String,
        expected: String,
        actual: String,
    },

    #[error("unsafe path in archive: {path}")]
    UnsafePath { path: String },

    #[error("invalid package name: {0}")]
    InvalidPackageName(String),

    #[error("manifest.toml not found in {path}")]
    ManifestNotFound { path: String },

    #[error("info file not found: {path}")]
    InfoFileNotFound { path: String },

    #[error("invalid info file format: {message}")]
    InvalidInfoFormat { message: String },

    #[error("makeinfo not found; install GNU Texinfo to build documentation from .texi files")]
    MakeinfoNotFound,

    #[error("makeinfo failed: {message}")]
    MakeinfoFailed { message: String },

    #[error("pandoc not found; install Pandoc to build documentation from .md files")]
    PandocNotFound,

    #[error("pandoc failed: {message}")]
    PandocFailed { message: String },

    #[error("{0}")]
    Io(#[from] std::io::Error),
}
