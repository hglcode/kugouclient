//! Team endpoints (the `team_*` family).
//!
//! The team family shares the "h5 token envelope + web signature" protocol
//! with `login.device.kick`.
//!
//! Ported from `special/team.ts`.

use serde_json::Value;

use crate::catalog::ctx::{GatewayRequest, SignControl};
use crate::catalog::def::{EndpointSpec, Salt, handler_endpoint, spec};
use crate::error::{Error, Result};
use crate::js::{self, FlexI64};
use crate::query::QueryRecord;

use super::super::ctx::CallCtx;
use super::super::def::RegisteredEndpoint;

kugou_params! {
    /// Parameters for `team.history`.
    pub TeamHistoryParams {
        required {}
        optional {
            /// Page number.
            page: FlexI64 => "page",
            /// Page size (default 10).
            page_size: FlexI64 => "page_size",
        }
    }
}

kugou_params! {
    /// Parameters for `team.my.info` / `team.my.status`.
    pub TeamPeriodParams {
        required {}
        optional {
            /// Period id.
            period_id: FlexI64 => "period_id",
        }
    }
}

/// The h5 token envelope: `h5` + raw no-padding RSA (128-byte zero-padded)
/// uppercase hex, following the original reference semantics (lite key).
fn h5_token(_ctx: &CallCtx<'_>, raw_token: &str) -> Result<String> {
    let input = format!("moc.uoguk.59::{raw_token}").into_bytes();
    let mut padded = vec![0u8; 128];
    if input.len() > 128 {
        return Err(Error::Crypto(
            "token exceeds the 128-byte RSA slot".to_owned(),
        ));
    }
    padded[..input.len()].copy_from_slice(&input);
    use rsa::pkcs8::DecodePublicKey;
    let key = rsa::PublicKey::from_public_key_pem(crate::crypto::LITE_RSA_PUBLIC_KEY)
        .map_err(|e| Error::Crypto(e.to_string()))?;
    Ok(format!(
        "h5{}",
        hex::encode(crate::crypto::raw_modpow_public(&key, &padded)?).to_uppercase()
    ))
}

/// The team-family shared request (web signature + h5 token).
async fn team_request(ctx: &CallCtx<'_>, url: &str, extra: QueryRecord) -> Result<Value> {
    let token = h5_token(ctx, &ctx.token)?;
    let mut query = QueryRecord::new();
    query.set("srcappid", ctx.srcappid);
    query.set("clientver", ctx.clientver);
    query.set("clienttime", crate::crypto::now_ms());
    query.set("mid", ctx.mid.clone());
    query.set("uuid", ctx.uuid.clone());
    query.set("dfid", ctx.dfid.clone());
    query.set("appid", ctx.appid);
    query.set("userid", ctx.userid);
    query.set("token", token);
    query.merge(&extra);

    ctx.request(
        url,
        GatewayRequest {
            method: Some(crate::http::Method::Get),
            sign: SignControl::Web,
            query: Some(query),
            ..GatewayRequest::default()
        },
    )
    .await
}

const EP_TEAM_HISTORY: EndpointSpec = EndpointSpec {
    salt: Salt::Web,
    auth: true,
    ..spec(
        "team.history",
        "team_history",
        crate::http::Method::Get,
        "/youth/v1/ut/get_team_record",
    )
};
const EP_TEAM_MY_INFO: EndpointSpec = EndpointSpec {
    salt: Salt::Web,
    auth: true,
    ..spec(
        "team.my.info",
        "team_my_info",
        crate::http::Method::Get,
        "/youth/v1/ut/get_my_team_info",
    )
};
const EP_TEAM_MY_STATUS: EndpointSpec = EndpointSpec {
    salt: Salt::Web,
    auth: true,
    ..spec(
        "team.my.status",
        "team_my_status",
        crate::http::Method::Get,
        "/youth/v1/ut/get_user_status",
    )
};
const EP_TEAM_PERIOD_INFO: EndpointSpec = EndpointSpec {
    salt: Salt::Web,
    auth: true,
    ..spec(
        "team.period.info",
        "team_period_info",
        crate::http::Method::Get,
        "/youth/v1/ut/get_period_info",
    )
};

/// Endpoints for this family.
pub fn endpoints() -> Vec<RegisteredEndpoint> {
    vec![
        handler_endpoint(&EP_TEAM_HISTORY, team_history_handler),
        handler_endpoint(&EP_TEAM_MY_INFO, team_my_info_handler),
        handler_endpoint(&EP_TEAM_MY_STATUS, team_my_status_handler),
        handler_endpoint(&EP_TEAM_PERIOD_INFO, team_period_info_handler),
    ]
}

fn team_history_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a TeamHistoryParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let mut extra = QueryRecord::new();
        extra.set("page", js::or_flex(&p.page, 1));
        extra.set("page_size", js::or_flex(&p.page_size, 10));
        team_request(ctx, "/youth/v1/ut/get_team_record", extra).await
    })
}

fn team_my_info_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a TeamPeriodParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let mut extra = QueryRecord::new();
        extra.set_opt("period_id", p.period_id.as_ref().and_then(FlexI64::value));
        team_request(ctx, "/youth/v1/ut/get_my_team_info", extra).await
    })
}

fn team_my_status_handler<'a>(
    ctx: &'a CallCtx<'_>,
    p: &'a TeamPeriodParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let mut extra = QueryRecord::new();
        extra.set_opt("period_id", p.period_id.as_ref().and_then(FlexI64::value));
        team_request(ctx, "/youth/v1/ut/get_user_status", extra).await
    })
}

fn team_period_info_handler<'a>(
    ctx: &'a CallCtx<'_>,
    _p: &'a crate::catalog::def::NoParams,
) -> futures::future::BoxFuture<'a, Result<Value>> {
    Box::pin(
        async move { team_request(ctx, "/youth/v1/ut/get_period_info", QueryRecord::new()).await },
    )
}

impl crate::api::TeamApi<'_> {
    /// Team records (`team.history`).
    pub async fn history(&self, params: TeamHistoryParams) -> Result<Value> {
        self.client.call_value("team.history", params).await
    }

    /// My team info (`team.my.info`).
    pub async fn my_info(&self, params: TeamPeriodParams) -> Result<Value> {
        self.client.call_value("team.my.info", params).await
    }

    /// Team user status (`team.my.status`).
    pub async fn my_status(&self, params: TeamPeriodParams) -> Result<Value> {
        self.client.call_value("team.my.status", params).await
    }

    /// Period info (`team.period.info`).
    pub async fn period_info(&self) -> Result<Value> {
        self.client
            .call_value("team.period.info", crate::catalog::def::NoParams::default())
            .await
    }
}
