use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as, skip_serializing_none};

/// The way to inform External-DNS what kind of domains the DNS service provider
/// could handle.
/// The domain names used here does not accept the trailing dot.
/// I did not see it mentioned in External-DNS doc, so does not trim here.
#[serde_as]
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all_fields = "camelCase", untagged)]
pub enum DomainFilter {
    Strings {
        include: Option<Vec<String>>,
        exclude: Option<Vec<String>>,
    },
    Regex {
        #[serde_as(as = "Option<DisplayFromStr>")]
        regex_include: Option<Regex>,
        #[serde_as(as = "Option<DisplayFromStr>")]
        regex_exclude: Option<Regex>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let json = serde_json::to_string(&DomainFilter::Strings {
            include: None,
            exclude: Some(vec!["example.org".to_string()]),
        });
        assert_eq!(json.unwrap(), r#"{"exclude":["example.org"]}"#);

        let json = serde_json::to_string(&DomainFilter::Regex {
            regex_include: Some(Regex::new("[0-9]a").unwrap()),
            regex_exclude: None,
        });
        assert_eq!(json.unwrap(), r#"{"regexInclude":"[0-9]a"}"#);
    }
}
