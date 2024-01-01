#[cfg(test)]
pub mod tests {
    use std::fmt::Debug;
    use serde::{Deserialize, Serialize};

    ///
    /// Helper function for serialization/deserialization tests.
    /// First serializes ``data`` and compares the result with ``json``.
    /// Then deserializes ``json`` into an object of type ``Data`` and compares the result with ``data``.
    ///
    pub fn serde_and_verify<'de, Data>(data: &Data, json: &'de str)
        where Data: Debug + Deserialize<'de> + Serialize + PartialEq {

        // 1. Serialize data and string-compare it to json
        let json_result = serde_json::to_string(&data);
        assert!(json_result.is_ok());
        assert_eq!(json_result.unwrap(), String::from(json));

        // 2. Deserialize json and compare it with data
        let data_result: Result<Data, serde_json::Error> = serde_json::from_str(json);
        assert!(data_result.is_ok());
        assert_eq!(data_result.unwrap(), *data);
    }
}