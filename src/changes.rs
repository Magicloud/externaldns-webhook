use crate::endpoint::Endpoint;
use serde::{Deserialize, Serialize};
use serde_with::{DefaultOnNull, serde_as};

/// Pair with direction
#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct FromTo<T> {
    pub from: T,
    pub to: T,
}

/// Data structure posted from ExternalDNS
/// The data represent the changes that ExternalDNS wants to make
/// It is not certain that all fields would be filled in one request.
/// Could be an Enum.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Changes {
    // Funny enough, when removing records, this field is `null`,
    // instead of `[]` as used in other fields.
    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub create: Vec<Endpoint>,
    #[serde(flatten, with = "serde_fromto")]
    pub update: Vec<FromTo<Endpoint>>,
    pub delete: Vec<Endpoint>,
}
impl Default for Changes {
    fn default() -> Self {
        Self {
            create: Default::default(),
            update: Default::default(),
            delete: Default::default(),
        }
    }
}

mod serde_fromto {
    use super::FromTo;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(default)]
    struct FromTos<T> {
        #[serde(rename = "UpdateOld")]
        old: Vec<T>,
        #[serde(rename = "UpdateNew")]
        new: Vec<T>,
    }
    impl<T> Default for FromTos<T> {
        fn default() -> Self {
            Self {
                old: Default::default(),
                new: Default::default(),
            }
        }
    }

    pub fn serialize<S, T>(fts: &Vec<FromTo<T>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize + Clone,
    {
        let mut out = FromTos {
            old: vec![],
            new: vec![],
        };
        for ft in fts {
            out.old.push(ft.from.clone());
            out.new.push(ft.to.clone());
        }

        out.serialize(serializer)
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Vec<FromTo<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        let the_in = FromTos::deserialize(deserializer)?;
        if the_in.old.len() == the_in.new.len() {
            let ret: Vec<FromTo<_>> = std::iter::zip(the_in.old, the_in.new)
                .map(|(from, to)| FromTo { from, to })
                .collect();
            Ok(ret)
        } else {
            Err(D::Error::custom(
                "The count of old and new data are not the same",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let json: Result<Changes, _> = serde_json::from_str(
            r##"{
    "create": [
        {
            "dnsName": "nextcloud.magicloud.lan",
            "targets": [
                "192.168.0.102"
            ],
            "recordType": "A",
            "labels": {
                "owner": "default",
                "resource": "ingress/nextcloud/nextcloud"
            }
        },
        {
            "dnsName": "a-nextcloud.magicloud.lan",
            "targets": [
                "\"heritage=external-dns,external-dns/owner=default,external-dns/resource=ingress/nextcloud/nextcloud\""
            ],
            "recordType": "TXT",
            "labels": {
                "ownedRecord": "nextcloud.magicloud.lan"
            }
        }
    ]
}"##,
        );
        eprintln!("{json:?}");
    }
}
