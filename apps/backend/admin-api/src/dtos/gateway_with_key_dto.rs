use serde::Serialize;

use crate::dtos::GatewayDto;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayWithKeyDto {
    pub gateway: GatewayDto,
    pub api_key: String,
}
