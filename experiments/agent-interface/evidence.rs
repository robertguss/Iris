// Test-only evidence sink: fixed event identifiers and integer observations.
pub fn record(event: &str, observed: i64) {
    use std::io::Write;
    if let Some(path) = std::env::var_os("IRIS_EVIDENCE_FILE") {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .unwrap();
        writeln!(file, "{event}\t{observed}").unwrap();
    }
}
