use crate::endpoint::Endpoint;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct FromTo<T> {
    pub from: T,
    pub to: T,
}

// One change at once, or multiple in one POST?
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Changes {
    pub create: Vec<Endpoint>,
    #[serde(flatten, with = "serde_fromto")]
    pub update: Vec<FromTo<Endpoint>>,
    pub delete: Vec<Endpoint>,
}

mod serde_fromto {
    use super::FromTo;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct FromTos<T> {
        #[serde(rename = "UpdateOld")]
        old: Vec<T>,
        #[serde(rename = "UpdateNew")]
        new: Vec<T>,
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
        let json = serde_json::to_string(&Changes {
            update: vec![],
            delete: vec![],
            create: vec![],
        });
        assert_eq!(
            json.unwrap(),
            r##"{"Create":[],"UpdateOld":[],"UpdateNew":[],"Delete":[]}"##
        );
    }
}