use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExtensionRepo {
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExtensionListing {
    pub pkg_name: String,
    pub name: String,
    pub version_name: String,
    pub version_code: i64,
    pub lang: String,
    pub is_nsfw: bool,
    pub apk_url: String,
    pub icon_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JsSourceMeta {
    pub id: i64,
    pub name: String,
    pub lang: String,
    pub base_url: String,
}
