//! Account-domain regular endpoints (captcha / earphone models / verify info).
//!
//! Ported from `domains/account.ts`.

use serde_json::{Value, json};

use crate::catalog::def::{Built, EndpointSpec, simple_endpoint, spec};
use crate::error::Result;
use crate::js::{self, FlexI64, FlexString};
use crate::query::QueryRecord;

use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `captcha.sent`.
    pub CaptchaSentParams {
        required {
            /// Mobile number.
            mobile: FlexString => "mobile",
        }
        optional {}
    }
}

kugou_params! {
    /// Parameters for `get.mode.info`.
    pub GetModeInfoParams {
        required {
            /// Earphone model id.
            model_id: FlexI64 => "model_id",
        }
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
        }
    }
}

kugou_params! {
    /// Parameters for `get.model`.
    pub GetModelParams {
        required {}
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size.
            page_size: FlexI64 => "pagesize",
            /// Sort (default 2).
            sort: FlexI64 => "sort",
        }
    }
}

kugou_params! {
    /// Parameters for `get.verify.info`.
    pub GetVerifyInfoParams {
        required {
            /// Verification event id.
            eventid: FlexI64 => "eventid",
        }
        optional {
            /// Platform id (default 2).
            platid: FlexI64 => "platid",
        }
    }
}

const EP_CAPTCHA_SENT: EndpointSpec = EndpointSpec {
    origin: Some("http://login.user.kugou.com"),
    ..spec(
        "captcha.sent",
        "captcha_sent",
        crate::http::Method::Post,
        "/v7/send_mobile_code",
    )
};
const EP_GET_MODE_INFO: EndpointSpec = EndpointSpec {
    origin: Some("http://mobileservice.kugou.com"),
    ..spec(
        "get.mode.info",
        "get_mode_info",
        crate::http::Method::Get,
        "/api/v5/earphone/get_model_info",
    )
};
const EP_GET_MODEL: EndpointSpec = spec(
    "get.model",
    "get_model",
    crate::http::Method::Get,
    "/ocean/v6/sound/list",
);
const EP_GET_VERIFY_INFO: EndpointSpec = spec(
    "get.verify.info",
    "get_verify_info",
    crate::http::Method::Post,
    "/verifyservice/v3/get_verify_info",
);

/// Endpoints for this domain.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        simple_endpoint(&EP_CAPTCHA_SENT, |p: &CaptchaSentParams, _| {
            let body = json!({
                "businessid": 5,
                "mobile": p.mobile.value().unwrap_or_default(),
                "plat": 3,
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_GET_MODE_INFO, |p: &GetModeInfoParams, ctx| {
            let mut query = QueryRecord::new();
            query.set("model_id", p.model_id.value().unwrap_or(0));
            query.set("req_src", "collection");
            query.set("earphone_vip", 1);
            query.set("sound_ver", 2);
            query.set("key", ctx.sign_params_key(crate::crypto::now_ms()));
            query.set("page", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_GET_MODEL, |p: &GetModelParams, _| {
            let mut query = QueryRecord::new();
            query.set("super_vip", 1);
            query.set("sound_ver", 2);
            query.set("page", js::or_flex(&p.page, 1));
            query.set("pagesize", js::or_flex(&p.page_size, 30));
            query.set("apiver", 3);
            query.set("classify", "2,3");
            query.set("plat", 2);
            query.set("privilege", 1);
            query.set("sort", js::or_flex(&p.sort, 2));
            Built {
                query,
                ..Built::default()
            }
        }),
        simple_endpoint(&EP_GET_VERIFY_INFO, |p: &GetVerifyInfoParams, ctx| {
            let body = json!({
                "eventid": p.eventid.value(),
                "userid": ctx.userid,
                "platid": js::or_flex(&p.platid, 2),
                "rtype": 1,
                "wasm": 1,
                "i": "",
                "sid": "",
                "edt": "",
            });
            Built {
                json: Some(body),
                ..Built::default()
            }
        }),
    ]
}

impl crate::api::AccountApi<'_> {
    /// Send an SMS verification code (`captcha.sent`).
    pub async fn captcha_sent(&self, params: CaptchaSentParams) -> Result<Value> {
        self.client.call_value("captcha.sent", params).await
    }

    /// Earphone model info (`get.mode.info`).
    pub async fn get_mode_info(&self, params: GetModeInfoParams) -> Result<Value> {
        self.client.call_value("get.mode.info", params).await
    }

    /// Sound-effect models (`get.model`).
    pub async fn get_model(&self, params: GetModelParams) -> Result<Value> {
        self.client.call_value("get.model", params).await
    }

    /// Verification info (`get.verify.info`).
    pub async fn get_verify_info(&self, params: GetVerifyInfoParams) -> Result<Value> {
        self.client.call_value("get.verify.info", params).await
    }
}
