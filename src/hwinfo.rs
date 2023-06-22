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


pub fn match_hw(hw_json: &serde_json::Value, hier_name: &str, hw_name: &str) -> Result<String> {
    let hw_object = json_as_map!(hw_json);
    for k in hw_object.keys() {
        if let Some(_) = k.find(hier_name) {
            if json_as_str!(hw_object[k]["name"])== hw_name {
                return Ok(k.clone());
            }
        }
    }
    Err(anyhow!("hw object not found: {}, {}", hier_name, hw_name))
}
