pub(crate) const CONFIG_DIR: &str = "CONFIG_DIR";
#[cfg(feature = "postgres_ssl")]
pub(crate) const DB_ROOT_CA_PATH: &str = "DB_ROOT_CA_PATH";

pub mod base64 {
    pub(crate) const BASE64_ENGINE: base64::engine::GeneralPurpose =
        base64::engine::general_purpose::STANDARD;
}
