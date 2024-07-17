use std::collections::HashMap;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Blueprint {
    pub version: String,
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub template: Template,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Template {
    pub variables: Option<HashMap<String, complate::config::VariableDefinition>>,
    pub helpers: std::option::Option<HashMap<String, String>>,
}
