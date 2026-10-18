//! JSON-to-Candid boundaries shared by portal workflows. Catalog IDs are UUIDs;
//! delivery jobs and revision counters remain numbers, never catalog references.
use serde_json::Value;
use sovereign_cloud::{ObjectId, args, drive::DriveCursor};

fn text<'a>(payload: &'a Value, key: &str) -> Result<&'a str, String> {
    payload[key].as_str().ok_or_else(|| format!("Missing or invalid {key}"))
}

pub fn list(payload: &Value) -> Result<Vec<u8>, String> {
    let parent: Option<ObjectId> = match &payload["parent"] {
        Value::Null => None,
        _ => Some(text(payload, "parent")?.parse().map_err(str::to_owned)?),
    };
    let cursor: Option<DriveCursor> = match &payload["cursor"] {
        Value::Null => None,
        _ => Some(serde_json::from_str(text(payload, "cursor")?).map_err(|e| e.to_string())?),
    };
    args((parent, cursor, 50u32)).map_err(|e| e.to_string())
}

pub fn email_action(operation: &str, payload: &Value) -> Result<(&'static str, Vec<u8>), String> {
    if operation == "emailRemove" {
        return Ok(("email_remove", args((text(payload, "email")?,)).map_err(|e| e.to_string())?));
    }
    let method = match operation {
        "emailRetry" => "email_retry",
        "emailRevoke" => "email_revoke_invitation",
        _ => return Err("Unknown email action".into()),
    };
    let id: u64 = text(payload, "id")?.parse().map_err(|_| "Invalid email operation number")?;
    Ok((method, args((id,)).map_err(|e| e.to_string())?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn upload_collision_checks_and_folder_pickers_send_uuid_parents() {
        let uuid = "01234567-89ab-4cde-8fab-0123456789ab";
        let encoded = list(&json!({"parent": uuid, "cursor": null})).unwrap();
        let (parent, cursor, limit): (Option<ObjectId>, Option<DriveCursor>, u32) =
            candid::decode_args(&encoded).unwrap();
        assert_eq!(parent.unwrap().to_string(), uuid);
        assert!(cursor.is_none());
        assert_eq!(limit, 50);
        let (parent, _, _): (Option<ObjectId>, Option<DriveCursor>, u32) =
            candid::decode_args(&list(&json!({"parent": null})).unwrap()).unwrap();
        assert!(parent.is_none());
        for parent in [json!("8"), json!(8), json!({})] {
            assert!(list(&json!({"parent":parent})).is_err());
        }
    }

    #[test]
    fn invitation_retry_and_revocation_send_job_numbers_not_file_ids() {
        for (operation, method) in [("emailRetry", "email_retry"), ("emailRevoke", "email_revoke_invitation")] {
            let (actual, encoded) = email_action(operation, &json!({"id":"42"})).unwrap();
            assert_eq!(actual, method);
            assert_eq!(candid::decode_args::<(u64,)>(&encoded).unwrap(), (42,));
            assert!(email_action(operation, &json!({"id":"01234567-89ab-4cde-8fab-0123456789ab"})).is_err());
        }
    }
}
