use std::path::PathBuf;

/// Custom error types for TidyMac operations.
/// We use `anyhow` at the top level for CLI error handling,
/// but these typed errors allow modules to be precise about failures.

#[derive(Debug)]
pub enum TidyError {
    /// File system operation failed
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Permission denied accessing a path
    PermissionDenied {
        path: PathBuf,
        hint: String,
    },

    /// SIP-protected path cannot be accessed
    SipProtected {
        path: PathBuf,
    },

    /// Configuration file is invalid
    ConfigError {
        path: PathBuf,
        message: String,
    },

    /// Profile not found
    ProfileNotFound {
        name: String,
    },

    /// Staging area operation failed
    StagingError {
        message: String,
    },

    /// Hash computation failed
    HashError {
        path: PathBuf,
        message: String,
    },

    /// App bundle is invalid or unreadable
    AppError {
        name: String,
        message: String,
    },

    /// Generic error with context
    Other {
        message: String,
    },
}

impl std::fmt::Display for TidyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TidyError::Io { path, source } => {
                write!(f, "I/O error at '{}': {}", path.display(), source)
            }
            TidyError::PermissionDenied { path, hint } => {
                write!(
                    f,
                    "Permission denied: '{}'. {}",
                    path.display(),
                    hint
                )
            }
            TidyError::SipProtected { path } => {
                write!(
                    f,
                    "SIP-protected path (cannot modify): '{}'",
                    path.display()
                )
            }
            TidyError::ConfigError { path, message } => {
                write!(f, "Config error in '{}': {}", path.display(), message)
            }
            TidyError::ProfileNotFound { name } => {
                write!(f, "Profile '{}' not found", name)
            }
            TidyError::StagingError { message } => {
                write!(f, "Staging error: {}", message)
            }
            TidyError::HashError { path, message } => {
                write!(f, "Hash error for '{}': {}", path.display(), message)
            }
            TidyError::AppError { name, message } => {
                write!(f, "App error for '{}': {}", name, message)
            }
            TidyError::Other { message } => {
                write!(f, "{}", message)
            }
        }
    }
}

impl std::error::Error for TidyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TidyError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TidyError {
    fn from(e: std::io::Error) -> Self {
        TidyError::Io {
            path: PathBuf::from("<unknown>"),
            source: e,
        }
    }
}
