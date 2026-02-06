

pub fn escape_sql_string_literal(s: &str) -> String {
    // Escape backslashes first, then single quotes
    s.replace('\\', "\\\\").replace('\'', "''")
}