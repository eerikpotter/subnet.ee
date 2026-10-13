//! Resolve a service separately from the logical file reference. Trust comes from site config.
use crate::CloudResult;
use candid::Principal;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceEndpoint {
    pub gateway: String,
    pub canister: Principal,
    pub root_key: Vec<u8>,
    pub derivation_origin: Option<String>,
}

/// Same-origin paths share a principal; a configured alternative origin is explicitly opt-in.
pub fn origin(value: &str) -> CloudResult<String> {
    let parsed = url::Url::parse(value).map_err(|_| "Invalid cloud origin")?;
    let loopback = matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")) || parsed.host_str().is_some_and(|h| h.ends_with(".localhost"));
    if (parsed.scheme() != "https" && !(parsed.scheme() == "http" && loopback))
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err("Cloud origin must be HTTPS (or local HTTP), without credentials, a path or query".into());
    }
    Ok(parsed.origin().ascii_serialization())
}

// Internet Identity treats the official gateway aliases as one application.
fn identity_origin(value: &str) -> String {
    for suffix in [".icp.net", ".icp0.io", ".ic0.app"] {
        if let Some(id) = value.strip_prefix("https://").and_then(|host| host.strip_suffix(suffix))
            && let Ok(canister) = Principal::from_text(id)
        {
            return format!("https://{canister}.icp.net");
        }
    }
    value.to_string()
}

pub fn environment(cookies: &str) -> CloudResult<BTreeMap<String, String>> {
    let mut found = None;
    for cookie in cookies.split(';').map(str::trim).filter_map(|c| c.strip_prefix("ic_env=")) {
        let decoded = percent_encoding::percent_decode_str(cookie).decode_utf8().map_err(|_| "Invalid cloud environment cookie")?;
        let mut env = BTreeMap::new();
        for item in decoded.split('&').filter(|s| !s.is_empty()) {
            let (key, value) = item.split_once('=').ok_or("Invalid cloud environment entry")?;
            if let Some(old) = env.insert(key.to_string(), value.to_string())
                && old != value
            {
                return Err("Conflicting cloud environment keys".into());
            }
        }
        if found.as_ref().is_some_and(|old| old != &env) {
            return Err("Conflicting cloud environment cookies; clear site cookies and reload".into());
        }
        found = Some(env);
    }
    found.ok_or("Cloud environment unavailable. Use the local ICP gateway or a configured frontend canister.".into())
}

impl ServiceEndpoint {
    pub fn from_environment(cookies: &str, application_origin: &str) -> CloudResult<Self> {
        let env = environment(cookies)?;
        let application_origin = origin(application_origin)?;
        let canister = Principal::from_text(env.get("PUBLIC_CANISTER_ID:cloud").ok_or("Cloud canister is not configured")?).map_err(|e| e.to_string())?;
        if canister == Principal::anonymous() || canister == Principal::management_canister() {
            return Err("Invalid cloud canister".into());
        }
        let root_key = hex::decode(env.get("ic_root_key").ok_or("Missing network trust key")?).map_err(|_| "Invalid network trust key")?;
        if root_key.len() != 133 {
            return Err("Invalid network trust key length".into());
        }
        // Certified-assets exposes only PUBLIC_* settings in its certified cookie.
        // Keep the old names readable for existing independently hosted clients.
        let gateway = env.get("PUBLIC_SOVEREIGN_CLOUD_GATEWAY").or_else(|| env.get("SOVEREIGN_CLOUD_GATEWAY"));
        let derivation_origin = env
            .get("PUBLIC_SOVEREIGN_CLOUD_DERIVATION_ORIGIN")
            .or_else(|| env.get("SOVEREIGN_CLOUD_DERIVATION_ORIGIN"))
            .map(|s| origin(s))
            .transpose()?
            .filter(|value| identity_origin(value) != identity_origin(&application_origin));
        Ok(Self { gateway: origin(gateway.map_or(application_origin.as_str(), String::as_str))?, canister, root_key, derivation_origin })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_cookie_conflicts_fail_closed() {
        assert!(environment("ic_env=a%3D1; ic_env=a%3D2").is_err());
        assert!(environment("ic_env=a%3D1%26a%3D2").is_err());
        assert_eq!(environment("ic_env=a%3D1; ic_env=a%3D1").unwrap().get("a").unwrap(), "1");
    }
    #[test]
    fn origins_do_not_accept_remote_http_or_arbitrary_paths() {
        for value in ["http://evil.example", "https://subnet.ee/apps/photo", "https://user@subnet.ee", "https://subnet.ee/?x=1", "data:text/plain,test"] {
            assert!(origin(value).is_err(), "{value}");
        }
        assert_eq!(origin("https://subnet.ee/").unwrap(), "https://subnet.ee");
        assert!(origin("http://frontend.local.localhost:8000").is_ok());
    }
    #[test]
    fn endpoint_requires_explicit_trust_and_preserves_origin_by_default() {
        let key = "11".repeat(133);
        let cookies = format!("ic_env=PUBLIC_CANISTER_ID:cloud=rn333-2qaaa-aaaas-amyfa-cai&ic_root_key={key}");
        let endpoint = ServiceEndpoint::from_environment(&cookies, "https://subnet.ee").unwrap();
        assert_eq!(endpoint.derivation_origin, None);
        assert_eq!(endpoint.gateway, "https://subnet.ee");
        assert!(ServiceEndpoint::from_environment("ic_env=PUBLIC_CANISTER_ID:cloud=rn333-2qaaa-aaaas-amyfa-cai", "https://subnet.ee").is_err());
    }

    #[test]
    fn custom_domain_uses_public_api_and_existing_canister_identity() {
        let key = "11".repeat(133);
        let primary = "https://vigkv-fqaaa-aaaac-bgeha-cai.icp.net";
        let cookies = format!(
            "ic_env=PUBLIC_CANISTER_ID:cloud=vbfbj-tyaaa-aaaac-bgegq-cai&ic_root_key={key}&PUBLIC_SOVEREIGN_CLOUD_GATEWAY=https://icp-api.io&PUBLIC_SOVEREIGN_CLOUD_DERIVATION_ORIGIN={primary}"
        );
        let endpoint = ServiceEndpoint::from_environment(&cookies, "https://www.subnet.ee").unwrap();
        assert_eq!(endpoint.gateway, "https://icp-api.io");
        assert_eq!(endpoint.derivation_origin.as_deref(), Some(primary));
        for suffix in ["icp.net", "icp0.io", "ic0.app"] {
            let origin = format!("https://vigkv-fqaaa-aaaac-bgeha-cai.{suffix}");
            let endpoint = ServiceEndpoint::from_environment(&cookies, &origin).unwrap();
            assert_eq!(endpoint.gateway, "https://icp-api.io");
            assert_eq!(endpoint.derivation_origin, None);
        }
        let other = ServiceEndpoint::from_environment(&cookies, "https://vbfbj-tyaaa-aaaac-bgegq-cai.icp.net").unwrap();
        assert_eq!(other.derivation_origin.as_deref(), Some(primary));
    }

    #[test]
    fn public_endpoint_settings_are_validated_and_local_stays_local() {
        let base = format!("ic_env=PUBLIC_CANISTER_ID:cloud=vbfbj-tyaaa-aaaac-bgegq-cai&ic_root_key={}", "11".repeat(133));
        for setting in ["PUBLIC_SOVEREIGN_CLOUD_GATEWAY=http://remote.example", "PUBLIC_SOVEREIGN_CLOUD_DERIVATION_ORIGIN=https://example.com/path"] {
            assert!(ServiceEndpoint::from_environment(&format!("{base}&{setting}"), "https://www.subnet.ee").is_err());
        }
        let local = "http://frontend.local.localhost:8000";
        let endpoint = ServiceEndpoint::from_environment(&base, local).unwrap();
        assert_eq!(endpoint.gateway, local);
        assert_eq!(endpoint.derivation_origin, None);
    }
}
