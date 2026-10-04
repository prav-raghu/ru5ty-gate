use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use validator::Validate;

use crate::{FasError, FasPayload};

const MAX_ENCODED_LENGTH: usize = 8192;
const MAX_ORIGIN_URL_LENGTH: usize = 2048;

pub struct FasPayloadDecoder;

impl FasPayloadDecoder {
    pub fn decode(encoded: &str) -> Result<FasPayload, FasError> {
        if encoded.is_empty() {
            return Err(FasError::MissingPayload);
        }
        if encoded.len() > MAX_ENCODED_LENGTH {
            return Err(FasError::MalformedPayload);
        }
        let bytes = STANDARD
            .decode(encoded.trim_matches(['\r', '\n']).replace(' ', "+"))
            .map_err(|_| FasError::MalformedPayload)?;
        let text = String::from_utf8(bytes).map_err(|_| FasError::MalformedPayload)?;
        let fields = Self::split_fields(&text);

        let required = |name: &'static str| {
            fields
                .get(name)
                .cloned()
                .ok_or(FasError::InvalidField(name))
        };
        let payload = FasPayload {
            clientmac: required("clientmac")?.to_ascii_lowercase(),
            clientip: required("clientip")?,
            hid: required("hid")?,
            gatewayaddress: required("gatewayaddress")?,
            authdir: required("authdir")?,
            gatewayname: Self::decode_component(&required("gatewayname")?)
                .ok_or(FasError::InvalidField("gatewayname"))?,
            originurl: fields
                .get("originurl")
                .and_then(|value| Self::decode_component(value))
                .filter(|url| Self::is_http_url(url)),
            clientif: fields.get("clientif").cloned(),
        };
        payload.validate().map_err(|errors| {
            let field = errors
                .field_errors()
                .keys()
                .next()
                .map_or("payload", |name| Self::static_field_name(name));
            FasError::InvalidField(field)
        })?;
        Ok(payload)
    }

    fn split_fields(text: &str) -> HashMap<String, String> {
        let mut fields = HashMap::new();
        for pair in text.split(", ") {
            if let Some((name, value)) = pair.split_once('=') {
                fields
                    .entry(name.trim().to_owned())
                    .or_insert_with(|| value.trim().to_owned());
            }
        }
        fields
    }

    fn decode_component(value: &str) -> Option<String> {
        urlencoding::decode(value)
            .ok()
            .map(std::borrow::Cow::into_owned)
    }

    fn is_http_url(value: &str) -> bool {
        value.len() <= MAX_ORIGIN_URL_LENGTH
            && (value.starts_with("http://") || value.starts_with("https://"))
    }

    fn static_field_name(name: &str) -> &'static str {
        match name {
            "clientmac" => "clientmac",
            "clientip" => "clientip",
            "hid" => "hid",
            "gatewayaddress" => "gatewayaddress",
            "authdir" => "authdir",
            "gatewayname" => "gatewayname",
            _ => "payload",
        }
    }
}
