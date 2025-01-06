pub(crate) const CONFIG_DIR: &str = "CONFIG_DIR";

pub mod base64 {
    pub const BASE64_ENGINE: base64::engine::GeneralPurpose =
        base64::engine::general_purpose::STANDARD;
}

pub const X_REQUEST_ID: &str = "x-request-id";
pub const TENANT_HEADER: &str = "x-tenant-id";
