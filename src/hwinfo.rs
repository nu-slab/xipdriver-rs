use anyhow::Result;
use std::fs::File;
use std::io::BufReader;

#[macro_export]
macro_rules! json_as_map {
    ($json_value: expr) => {
        $json_value.as_object().context(format!("{} is not an object type", stringify!($json_value)))?
    };
}

#[macro_export]
macro_rules! json_as_vec {
    ($json_value: expr) => {
        $json_value.as_array().context(format!("{} is not an array", stringify!($json_value)))?
    };
}

#[macro_export]
macro_rules! json_as_u32 {
    ($json_value: expr) => {
        $json_value.as_i64().context(format!("{} is not numeric", stringify!($json_value)))? as u32
    };
}

#[macro_export]
macro_rules! json_as_i32 {
    ($json_value: expr) => {
        $json_value.as_i64().context(format!("{} is not numeric", stringify!($json_value)))? as i32
    };
}

#[macro_export]
macro_rules! json_as_str {
    ($json_value: expr) => {
        $json_value.as_str().context(format!("{} is not string", stringify!($json_value)))?
    };
}

#[macro_export]
macro_rules! json_as_f32 {
    ($json_value: expr) => {
        $json_value.as_str().context(format!("{} is not string", stringify!($json_value)))?.parse::<f32>()?
    };
}

pub fn read(filepath: &str) -> Result<serde_json::Value> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let hw_json: serde_json::Value = serde_json::from_reader(reader).unwrap();
    Ok(hw_json)
}
