//! Public launch references carry identity-free IDs. Authorization remains server-side.
use crate::CloudResult;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudLaunch {
    pub space: u64,
    pub file: u64,
}
impl CloudLaunch {
    pub fn parse(query: &str) -> CloudResult<Option<Self>> {
        let mut values = BTreeMap::new();
        for (key, value) in url::form_urlencoded::parse(query.trim_start_matches('?').as_bytes()) {
            if key.starts_with("cloud_") && values.insert(key.to_string(), value.to_string()).is_some() {
                return Err("Duplicate cloud launch parameter".into());
            }
        }
        if values.is_empty() {
            return Ok(None);
        }
        if values.len() != 3 || values.get("cloud_service").map(String::as_str) != Some("cloud") {
            return Err("Unsupported cloud file link".into());
        }
        let id = |name: &str| -> CloudResult<u64> {
            let text = values.get(name).ok_or("Incomplete cloud file link")?;
            if !text.bytes().all(|c| c.is_ascii_digit()) {
                return Err("Invalid file ID".into());
            }
            text.parse::<u64>().ok().filter(|id| *id > 0).ok_or("Invalid file ID".into())
        };
        Ok(Some(Self { space: id("cloud_space")?, file: id("cloud_file")? }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_require_one_unambiguous_supported_reference() {
        assert_eq!(CloudLaunch::parse("?webgl").unwrap(), None);
        assert_eq!(CloudLaunch::parse("?cloud_service=cloud&cloud_space=1&cloud_file=2").unwrap(), Some(CloudLaunch { space: 1, file: 2 }));
        for value in [
            "cloud_file=2",
            "cloud_service=other&cloud_space=1&cloud_file=2",
            "cloud_service=cloud&cloud_space=0&cloud_file=2",
            "cloud_service=cloud&cloud_space=1&cloud_file=2&cloud_file=3",
            "cloud_service=cloud&cloud_space=1&cloud_file=18446744073709551616",
            "cloud_service=cloud&cloud_space=1&cloud_file=2&cloud_token=secret",
        ] {
            assert!(CloudLaunch::parse(value).is_err(), "{value}");
        }
    }
}
