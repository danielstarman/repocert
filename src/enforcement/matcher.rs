use globset::Glob;

pub(super) fn matches(pattern: &str, reference: &str) -> Result<bool, String> {
    let glob = Glob::new(pattern).map_err(|error| error.to_string())?;
    Ok(glob.compile_matcher().is_match(reference))
}
