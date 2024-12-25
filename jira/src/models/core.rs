use std::{
    cmp::Ordering,
    fmt::{self, Formatter},
};

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize,
};

/// Represents the author (user) of a worklog item
#[derive(Debug, Deserialize, Serialize, PartialOrd, PartialEq, Eq, Hash, Clone)]
#[allow(non_snake_case)]
pub struct Author {
    pub accountId: String,
    pub emailAddress: Option<String>,
    pub displayName: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, PartialOrd, PartialEq, Eq, Hash, Ord)]
pub struct Fields {
    pub summary: String,
    pub components: Vec<Component>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, PartialOrd, PartialEq, Eq, Hash, Ord)]
pub struct Component {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Asset {
    #[serde(alias = "self")]
    pub url: String,
    pub id: String,
    pub value: String,
}

/// Represents a Jira issue key like for instance `TIME-148`
/// This struct is typically known as a "value object" in other programming languages.
#[derive(Debug, Serialize, Default, Eq, PartialEq, Clone)]
pub struct IssueKey {
    #[serde(rename = "key")]
    pub value: String,
}

impl IssueKey {
    ///
    /// # Panics
    /// If the supplied value is empty
    #[must_use]
    pub fn new(input: &str) -> Self {
        assert!(
            !(input.is_empty() || input.trim().is_empty()),
            "JiraKey may not be empty!"
        );
        IssueKey {
            value: input.to_uppercase(),
        }
    }
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.value.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.value.trim().len() == 0
    }
}

impl From<String> for IssueKey {
    fn from(s: String) -> Self {
        IssueKey::new(&s)
    }
}

impl From<&str> for IssueKey {
    fn from(value: &str) -> Self {
        IssueKey::new(value)
    }
}

impl fmt::Display for IssueKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
impl Ord for IssueKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}
impl PartialOrd for IssueKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'de> Deserialize<'de> for IssueKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct JiraKeyVisitor;

        impl<'de> Visitor<'de> for JiraKeyVisitor {
            type Value = IssueKey;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or a map with a value field")
            }

            fn visit_str<E>(self, value: &str) -> Result<IssueKey, E>
            where
                E: de::Error,
            {
                Ok(IssueKey {
                    value: value.to_string(),
                })
            }

            fn visit_map<M>(self, mut map: M) -> Result<IssueKey, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let mut value = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        "key" => {
                            if value.is_some() {
                                return Err(de::Error::duplicate_field("key"));
                            }
                            value = Some(map.next_value()?);
                        }
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                let value = value.ok_or_else(|| de::Error::missing_field("key"))?;
                Ok(IssueKey { value })
            }
        }

        deserializer.deserialize_any(JiraKeyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jira_key() {
        let k1 = IssueKey::from("TIME-40");
        let k2 = IssueKey::from("TIME-40");
        assert_eq!(&k1, &k2, "Seems JiraKey does not compare by value");
    }

    #[test]
    fn test_jira_key_uppercase() {
        let k1 = IssueKey::from("time-147");
        assert_eq!(k1.to_string(), "TIME-147".to_string());
    }
}
