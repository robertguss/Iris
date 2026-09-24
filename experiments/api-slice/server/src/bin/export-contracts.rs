use iris_api_spike::{aide_router, utoipa_router};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .ok_or("usage: export-contracts OUTPUT_DIRECTORY")?;
    std::fs::create_dir_all(&out)?;
    for (name, api) in [
        ("utoipa", serde_json::to_value(utoipa_router().1)?),
        ("aide", serde_json::to_value(aide_router().1)?),
    ] {
        std::fs::write(
            format!("{out}/{name}.json"),
            serde_json::to_string_pretty(&api)? + "\n",
        )?;
    }
    Ok(())
}
