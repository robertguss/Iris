fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: export-openapi OUTPUT_FILE")?;
    std::fs::write(
        path,
        iris_reference::app::openapi().to_pretty_json()? + "\n",
    )?;
    Ok(())
}
