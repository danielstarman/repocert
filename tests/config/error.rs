use std::path::PathBuf;

use repocert::config::ParseError;

fn parse_error_from_content(content: &str) -> ParseError {
    let source = toml::from_str::<toml::Table>(content).unwrap_err();
    ParseError::from_toml(&PathBuf::from("/tmp/config.toml"), content, source)
}

fn assert_invalid_toml_location(error: ParseError, expected_line: usize, expected_column: usize) {
    match error {
        ParseError::InvalidToml { line, column, .. } => {
            assert_eq!(line, Some(expected_line));
            assert_eq!(column, Some(expected_column));
        }
        other => panic!("unexpected parse error: {other:?}"),
    }
}

#[test]
fn parse_error_from_toml_ascii_error_returns_expected_line_and_column() {
    // Arrange
    let content = "title = \"ok\"\nvalue = [";

    // Act
    let error = parse_error_from_content(content);

    // Assert
    assert_invalid_toml_location(error, 2, 10);
}

#[test]
fn parse_error_from_toml_unicode_error_returns_character_based_column() {
    // Arrange
    let content = "title = \"ok\"\né = [";

    // Act
    let error = parse_error_from_content(content);

    // Assert
    assert_invalid_toml_location(error, 2, 6);
}

#[test]
fn parse_error_from_toml_crlf_error_returns_next_line_start_column() {
    // Arrange
    let content = "title = \"ok\"\r\nvalue = [";

    // Act
    let error = parse_error_from_content(content);

    // Assert
    assert_invalid_toml_location(error, 2, 10);
}

#[test]
fn parse_error_from_toml_tab_error_counts_tab_as_single_column() {
    // Arrange
    let content = "title = \"ok\"\n\tvalue = [";

    // Act
    let error = parse_error_from_content(content);

    // Assert
    assert_invalid_toml_location(error, 2, 11);
}
