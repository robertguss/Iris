fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: export-s16 OUTPUT_FILE")?;
    std::fs::write(
        path,
        iris_api_spike::s16::routes().1.to_pretty_json()? + "\n",
    )?;
    Ok(())
}
