use std::{collections::HashMap, hash::Hash};

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

/// DNS record with extra infor used by External-DNS
/// From sample code, all fields are marked optional. I highly doubt that.
/// The `PartialEq`, `Eq` and `Hash` are implenmented on DNS record fields
/// (`dns_name`, `targets`, `record_type`, `record_ttl`).
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub dns_name: Option<String>,
    pub targets: Option<Vec<String>>,
    pub record_type: Option<RecordType>,
    pub set_identifier: Option<String>,
    #[serde(rename = "recordTTL")]
    pub record_ttl: Option<i64>,
    pub labels: Option<HashMap<String, String>>,
    pub provider_specific: Option<HashMap<String, String>>,
}
impl PartialEq for Endpoint {
    fn eq(&self, other: &Self) -> bool {
        self.dns_name == other.dns_name
            && self.targets == other.targets
            && self.record_type == other.record_type
            && self.record_ttl == other.record_ttl
    }
}
impl Eq for Endpoint {}
impl Hash for Endpoint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.dns_name.hash(state);
        self.targets.hash(state);
        self.record_type.hash(state);
        self.record_ttl.hash(state);
    }
}

/// DNS records types
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub enum RecordType {
    A,
    AAAA,
    CNAME,
    TXT,
    SRV,
    NS,
    PTR,
    MX,
    NAPTR,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let json: Result<Endpoint, _> = serde_json::from_str(
            r#"{
            "dnsName": "nextcloud.magicloud.lan",
            "targets": [
                "192.168.0.102"
            ],
            "recordType": "A",
            "labels": {
                "owner": "default",
                "resource": "ingress/nextcloud/nextcloud"
            }
}"#,
        );
        eprintln!("{json:?}");

        let json: Result<Endpoint, _> = serde_json::from_str(
            r#"{
            "dnsName": "a-nextcloud.magicloud.lan",
            "targets": [
                "\"heritage=external-dns,external-dns/owner=default,external-dns/resource=ingress/nextcloud/nextcloud\""
            ],
            "recordType": "TXT",
            "labels": {
                "ownedRecord": "nextcloud.magicloud.lan"
            }
}"#,
        );
        eprintln!("{json:?}");
    }
}
