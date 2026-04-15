use serde::Serialize;

/// Output format for CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}

/// Print a serializable value as JSON to stdout.
pub fn print_json(value: &impl Serialize) -> Result<(), crate::errors::MxpmError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| crate::errors::MxpmError::Io(std::io::Error::other(e)))?;
    println!("{json}");
    Ok(())
}
