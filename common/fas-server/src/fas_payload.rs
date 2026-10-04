use validator::Validate;

use crate::fas_payload_validators::{
    validate_authdir, validate_gateway_address, validate_hid, validate_ip, validate_mac,
};

#[derive(Debug, Clone, Validate)]
pub struct FasPayload {
    #[validate(custom(function = "validate_mac"))]
    pub clientmac: String,
    #[validate(custom(function = "validate_ip"))]
    pub clientip: String,
    #[validate(custom(function = "validate_hid"))]
    pub hid: String,
    #[validate(custom(function = "validate_gateway_address"))]
    pub gatewayaddress: String,
    #[validate(custom(function = "validate_authdir"))]
    pub authdir: String,
    #[validate(length(min = 1, max = 128))]
    pub gatewayname: String,
    pub originurl: Option<String>,
    pub clientif: Option<String>,
}
