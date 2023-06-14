use anyhow::Result;
use std::fs::File;
use std::io::BufReader;

pub fn read(filepath: &str) -> Result<serde_json::Value> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let hw_json: serde_json::Value = serde_json::from_reader(reader).unwrap();
    Ok(hw_json)
}
