//! Public launch references carry identity-free IDs. Authorization remains server-side.
use crate::{CloudResult, ObjectId};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloudLaunch {
    File { space: ObjectId, file: ObjectId },
    New { space: ObjectId, parent: ObjectId },
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
        let id = |name: &str| -> CloudResult<ObjectId> {
            let text = values.get(name).ok_or("Incomplete cloud file link")?;
            text.parse::<ObjectId>().map_err(str::to_string)
        };
        let space = id("cloud_space")?;
        Ok(Some(if values.contains_key("cloud_parent") {
            Self::New { space, parent: id("cloud_parent")? }
        } else {
            Self::File { space, file: id("cloud_file")? }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_require_one_unambiguous_supported_reference() {
        let a = ObjectId::from_random_bytes([1; 16]);
        let b = ObjectId::from_random_bytes([2; 16]);
        assert_eq!(CloudLaunch::parse("?webgl").unwrap(), None);
        assert_eq!(CloudLaunch::parse(&format!("?cloud_service=cloud&cloud_space={a}&cloud_file={b}")).unwrap(), Some(CloudLaunch::File { space: a, file: b }));
        assert_eq!(
            CloudLaunch::parse(&format!("?cloud_service=cloud&cloud_space={a}&cloud_parent={b}")).unwrap(),
            Some(CloudLaunch::New { space: a, parent: b })
        );
        // Shell navigation hints are deliberately outside the cloud protocol.
        assert_eq!(CloudLaunch::parse(&format!("?cloud_service=cloud&cloud_space={a}&cloud_parent={b}&workspace_return=%2Fpilveruum%2F%23failid")).unwrap(), Some(CloudLaunch::New { space: a, parent: b }));
        for value in [
            "cloud_service=cloud&cloud_space=1&cloud_parent=2&cloud_file=3",
            "cloud_service=cloud&cloud_space=1&cloud_parent=0",
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
