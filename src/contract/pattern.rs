use globset::Glob;

pub(crate) fn validate_pattern(pattern: &str) -> Result<(), String> {
    Glob::new(pattern)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub(crate) fn matches_pattern(pattern: &str, value: &str) -> Result<bool, String> {
    let glob = Glob::new(pattern).map_err(|error| error.to_string())?;
    Ok(glob.compile_matcher().is_match(value))
}
