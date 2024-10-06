use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::hash::Hash;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub dns_name: Option<String>,
    pub targets: Option<Vec<String>>,
    pub record_type: Option<RecordType>,
    pub set_identifier: Option<String>,
    #[serde(rename = "recordTTL")]
    pub record_ttl: Option<u32>,
    pub labels: Option<DashMap<String, String>>,
    pub provider_specific: Option<DashMap<String, String>>,
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
        let json = serde_json::to_string(&Endpoint {
            dns_name: Some("test.example.org".to_string()),
            targets: Some(vec!["localhost".to_string()]),
            record_type: Some(RecordType::CNAME),
            set_identifier: None,
            record_ttl: Some(128),
            labels: Some(DashMap::from_iter([(
                "msg".to_string(),
                "test".to_string(),
            )])),
            provider_specific: None,
        });
        assert_eq!(
            json.unwrap(),
            r##"{"dnsName":"test.example.org","targets":["localhost"],"recordType":"CNAME","recordTTL":128,"labels":{"msg":"test"}}"##
        );
    }
}
