//! Offline integration tests: mock transport, request construction, session
//! flow and catalog dispatch — ported from the TS `client.test.ts` /
//! `call.test.ts` assertions.

use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use kugouapi::catalog::def::unwrap_envelope;
use kugouapi::crypto::CryptoProvider;
use kugouapi::http::{HttpRequest, HttpResponse, Transport};
use kugouapi::{
    KuGouClient, Quality, RegisterDeviceParams, SearchSongParams, SessionSnapshot, SongUrlParams,
};

/// A scripted mock transport: pops one scripted response per request.
struct MockTransport {
    responses: Mutex<Vec<MockResponse>>,
    requests: Mutex<Vec<RecordedRequest>>,
}

#[derive(Clone)]
struct MockResponse {
    status: u16,
    body: Vec<u8>,
    headers: Vec<(String, String)>,
}

#[derive(Clone)]
struct RecordedRequest {
    method: String,
    url: String,
    query: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

impl MockTransport {
    fn new(mut responses: Vec<MockResponse>) -> Arc<Self> {
        // pop() serves from the tail: store reversed so scripted responses
        // play back in their given order.
        responses.reverse();
        Arc::new(Self {
            responses: Mutex::new(responses),
            requests: Mutex::new(Vec::new()),
        })
    }

    fn ok_json(body: Value) -> MockResponse {
        MockResponse {
            status: 200,
            body: serde_json::to_vec(&body).unwrap(),
            headers: vec![("content-type".into(), "application/json".into())],
        }
    }

    fn envelope(data: Value) -> MockResponse {
        Self::ok_json(json!({ "status": 1, "error_code": 0, "data": data }))
    }

    fn last(&self) -> RecordedRequest {
        self.requests.lock().unwrap().last().cloned().unwrap()
    }

    fn query_of(request: &RecordedRequest, key: &str) -> Option<String> {
        request
            .query
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    fn header_of(request: &RecordedRequest, key: &str) -> Option<String> {
        request
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
    }
}

impl Transport for MockTransport {
    fn execute(
        &self,
        request: HttpRequest,
    ) -> futures::future::BoxFuture<'static, kugouapi::Result<HttpResponse>> {
        let response = self.responses.lock().unwrap().pop();
        self.requests.lock().unwrap().push(RecordedRequest {
            method: request.method.as_str().to_owned(),
            url: request.url.clone(),
            query: request
                .query
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_wire_string()))
                .collect(),
            headers: request.headers.clone(),
            body: request.body.as_ref().map(|b| b.as_bytes().to_vec()),
        });
        Box::pin(async move {
            let mock = response.unwrap_or_else(|| MockResponse {
                status: 200,
                body: b"{}".to_vec(),
                headers: Vec::new(),
            });
            Ok(HttpResponse {
                status: mock.status,
                headers: mock.headers,
                bytes: mock.body,
            })
        })
    }
}

/// Fixed-key stub crypto: the body is the plain JSON so it can be asserted.
struct StubCrypto;

impl CryptoProvider for StubCrypto {
    fn encrypt_request(
        &self,
        data: &kugouapi::crypto::CipherInput,
    ) -> kugouapi::crypto::EncryptedRequest {
        let plain = data.to_plain();
        kugouapi::crypto::EncryptedRequest {
            body: format!("BODY:{plain}"),
            key: "abcdef".to_owned(),
        }
    }

    fn decrypt_response(
        &self,
        payload: &str,
        _key: &str,
    ) -> kugouapi::Result<kugouapi::crypto::CipherOutput> {
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .unwrap();
        Ok(kugouapi::crypto::CipherOutput::parse(
            String::from_utf8_lossy(&bytes).into_owned(),
        ))
    }

    fn encrypt_key(&self, _key: &str, _userid: u64, _token: &str) -> kugouapi::Result<String> {
        Ok("P-PARAM".to_owned())
    }
}

fn dyn_transport(transport: &Arc<MockTransport>) -> Arc<dyn Transport> {
    Arc::clone(transport) as Arc<dyn Transport>
}

fn client_with(transport: Arc<MockTransport>) -> KuGouClient {
    KuGouClient::builder()
        .transport(dyn_transport(&transport))
        .crypto_provider(Box::new(StubCrypto))
        .build()
        .unwrap()
}

fn session_client(transport: Arc<MockTransport>) -> KuGouClient {
    KuGouClient::builder()
        .transport(dyn_transport(&transport))
        .crypto_provider(Box::new(StubCrypto))
        .session(SessionSnapshot {
            dfid: "DFID1".into(),
            mid: "MID1".into(),
            uuid: "UUID1".into(),
            token: "TOKEN1".into(),
            userid: 42,
            vip_type: None,
            vip_token: None,
            registered_at: None,
        })
        .build()
        .unwrap()
}

fn query_map(request: &RecordedRequest) -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    for (k, v) in &request.query {
        map.insert(k.clone(), json!(v));
    }
    map
}

/// Recompute the signature over the sent parameters (minus `signature`).
fn recompute_signature(request: &RecordedRequest, platform: kugouapi::Platform) -> String {
    recompute_signature_opt(request, platform, true)
}

/// `with_body = false` matches endpoints that sign without the body
/// (`register_dev`, per the reference implementation).
fn recompute_signature_opt(
    request: &RecordedRequest,
    platform: kugouapi::Platform,
    with_body: bool,
) -> String {
    let mut record = kugouapi::QueryRecord::new();
    for (k, v) in &request.query {
        if k == "signature" {
            continue;
        }
        record.set(k.clone(), v.clone());
    }
    let data = if with_body {
        request
            .body
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).into_owned())
    } else {
        None
    };
    kugouapi::sign::signature(
        &record,
        kugouapi::sign::SignOptions {
            platform,
            data: data.as_deref(),
            ..Default::default()
        },
    )
}

#[tokio::test]
async fn search_builds_url_query_headers_and_signature() {
    let transport = MockTransport::new(vec![MockTransport::envelope(json!({
        "lists": [], "total": 0
    }))]);
    let client = client_with(Arc::clone(&transport));

    let data = client
        .search()
        .song(SearchSongParams::new("周杰伦").page(2).page_size(10))
        .await
        .unwrap();
    assert_eq!(data.total, 0);

    let request = transport.last();
    assert_eq!(request.method, "GET");
    assert!(
        request
            .url
            .starts_with("https://gateway.kugou.com/v2/search/song")
    );
    let q = |key: &str| MockTransport::query_of(&request, key);

    assert_eq!(q("keyword").as_deref(), Some("周杰伦"));
    assert_eq!(q("page").as_deref(), Some("2"));
    assert_eq!(q("pagesize").as_deref(), Some("10"));
    assert_eq!(q("appid").as_deref(), Some("1005"));
    assert_eq!(q("clientver").as_deref(), Some("20489"));
    assert_eq!(q("dfid").as_deref(), Some("-"));
    // unregistered mid = derive_mid("-") = md5("-")
    assert_eq!(
        q("mid").as_deref(),
        Some("336d5ebc5436534e61d16e63ddfca327")
    );
    assert_eq!(q("platform").as_deref(), Some("AndroidFilter"));
    assert_eq!(q("iscorrection").as_deref(), Some("1"));
    assert!(q("clienttime").is_some_and(|v| v.chars().all(|c| c.is_ascii_digit())));

    // the signature covers exactly the sent parameters
    let sent = recompute_signature(&request, kugouapi::Platform::Standard);
    assert_eq!(q("signature").as_deref(), Some(sent.as_str()));

    // headers
    assert_eq!(
        MockTransport::header_of(&request, "x-router").as_deref(),
        Some("complexsearch.kugou.com")
    );
    assert_eq!(
        MockTransport::header_of(&request, "kg-rc").as_deref(),
        Some("1")
    );
    assert!(MockTransport::header_of(&request, "mid").is_some());
}

#[tokio::test]
async fn session_credentials_are_carried() {
    let transport = MockTransport::new(vec![MockTransport::envelope(json!({
        "lists": [], "total": 0
    }))]);
    let client = session_client(Arc::clone(&transport));
    client
        .search()
        .song(SearchSongParams::new("k"))
        .await
        .unwrap();

    let request = transport.last();
    assert_eq!(
        MockTransport::query_of(&request, "dfid").as_deref(),
        Some("DFID1")
    );
    assert_eq!(
        MockTransport::query_of(&request, "mid").as_deref(),
        Some("MID1")
    );
    assert_eq!(
        MockTransport::query_of(&request, "token").as_deref(),
        Some("TOKEN1")
    );
    assert_eq!(
        MockTransport::query_of(&request, "userid").as_deref(),
        Some("42")
    );
    assert_eq!(
        MockTransport::header_of(&request, "dfid").as_deref(),
        Some("DFID1")
    );
}

#[tokio::test]
async fn lite_platform_switches_salt() {
    let transport = MockTransport::new(vec![MockTransport::envelope(json!({
        "lists": [], "total": 0
    }))]);
    let client = KuGouClient::builder()
        .transport(dyn_transport(&transport))
        .platform(kugouapi::Platform::Lite)
        .build()
        .unwrap();
    client
        .search()
        .song(SearchSongParams::new("k"))
        .await
        .unwrap();

    let request = transport.last();
    assert_eq!(
        MockTransport::query_of(&request, "appid").as_deref(),
        Some("3116")
    );
    let sent = recompute_signature(&request, kugouapi::Platform::Lite);
    assert_eq!(
        MockTransport::query_of(&request, "signature").as_deref(),
        Some(sent.as_str())
    );
}

#[tokio::test]
async fn song_url_normalizes_hash_quality_and_computes_tracker_key() {
    let transport = MockTransport::new(vec![MockTransport::envelope(
        json!({ "url": ["http://a"] }),
    )]);
    let client = client_with(Arc::clone(&transport));

    let data = client
        .song()
        .url(SongUrlParams::new("ABC123").quality(Quality::Dj))
        .await
        .unwrap();
    assert_eq!(data.url.as_deref(), Some(&["http://a".to_string()][..]));

    let request = transport.last();
    assert_eq!(
        MockTransport::query_of(&request, "hash").as_deref(),
        Some("abc123")
    );
    assert_eq!(
        MockTransport::query_of(&request, "quality").as_deref(),
        Some("magic_dj")
    );
    assert_eq!(
        MockTransport::query_of(&request, "pid").as_deref(),
        Some("2")
    );
    assert_eq!(
        MockTransport::query_of(&request, "page_id").as_deref(),
        Some("151369488")
    );
    assert_eq!(
        MockTransport::header_of(&request, "x-router").as_deref(),
        Some("trackercdn.kugou.com")
    );

    // tracker key computed with the final query's identity values
    let expected_key = kugouapi::sign::tracker_key(kugouapi::sign::TrackerKeyInput {
        hash: "abc123",
        mid: "336d5ebc5436534e61d16e63ddfca327",
        userid: 0,
        appid: 1005,
        platform: kugouapi::Platform::Standard,
    });
    assert_eq!(
        MockTransport::query_of(&request, "key").as_deref(),
        Some(expected_key.as_str())
    );

    // signature covers the key too
    let sent = recompute_signature(&request, kugouapi::Platform::Standard);
    assert_eq!(
        MockTransport::query_of(&request, "signature").as_deref(),
        Some(sent.as_str())
    );
}

#[tokio::test]
async fn song_url_regular_quality_and_free_part() {
    let transport = MockTransport::new(vec![MockTransport::envelope(json!({}))]);
    let client = client_with(Arc::clone(&transport));
    client
        .song()
        .url(
            SongUrlParams::new("abc")
                .quality(Quality::Flac)
                .free_part(true),
        )
        .await
        .unwrap();
    let request = transport.last();
    assert_eq!(
        MockTransport::query_of(&request, "quality").as_deref(),
        Some("flac")
    );
    assert_eq!(
        MockTransport::query_of(&request, "IsFreePart").as_deref(),
        Some("1")
    );
}

#[tokio::test]
async fn register_device_encrypts_and_persists_session() {
    let transport = MockTransport::new(vec![MockTransport::envelope(json!({
        "dfid": "DFID2", "token": "TOKEN2", "userid": 9, "mid": "MID2"
    }))]);
    let client = client_with(Arc::clone(&transport));

    let data = client
        .register_device(RegisterDeviceParams::new().device_field("imei", "i".repeat(32)))
        .await
        .unwrap();
    assert_eq!(data.dfid.as_deref(), Some("DFID2"));

    let request = transport.last();
    assert_eq!(request.method, "POST");
    assert!(
        request
            .url
            .starts_with("https://userservice.kugou.com/risk/v3/r_register_dev")
    );
    let body = String::from_utf8(request.body.clone().unwrap()).unwrap();
    assert!(body.starts_with("BODY:{"));
    assert!(body.contains("\"imei\":\"iiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii\""));
    assert!(body.contains("\"availableRamSize\":4983533568"));

    assert_eq!(
        MockTransport::query_of(&request, "p").as_deref(),
        Some("P-PARAM")
    );
    assert_eq!(
        MockTransport::query_of(&request, "appid").as_deref(),
        Some("1005")
    );
    assert_eq!(
        MockTransport::query_of(&request, "part").as_deref(),
        Some("1")
    );
    assert_eq!(
        MockTransport::query_of(&request, "platid").as_deref(),
        Some("1")
    );
    assert_eq!(
        MockTransport::header_of(&request, "user-agent").as_deref(),
        Some("Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi")
    );
    // android signature covers the p parameter (register signs WITHOUT the body)
    let sent = recompute_signature_opt(&request, kugouapi::Platform::Standard, false);
    assert_eq!(
        MockTransport::query_of(&request, "signature").as_deref(),
        Some(sent.as_str())
    );

    // session persisted
    let session = client.session().unwrap();
    assert_eq!(session.dfid, "DFID2");
    assert_eq!(session.token, "TOKEN2");
    assert_eq!(session.userid, 9);
    assert_eq!(session.mid, "MID2");
    assert_eq!(session.uuid, kugouapi::DEFAULT_DEVICE.uuid);
}

#[tokio::test]
async fn register_then_search_carries_credentials() {
    let transport = MockTransport::new(vec![
        MockTransport::envelope(json!({ "dfid": "DFID3" })),
        MockTransport::envelope(json!({ "lists": [], "total": 0 })),
    ]);
    let client = client_with(Arc::clone(&transport));
    client
        .register_device(RegisterDeviceParams::new())
        .await
        .unwrap();
    client
        .search()
        .song(SearchSongParams::new("k"))
        .await
        .unwrap();
    let request = transport.last();
    assert_eq!(
        MockTransport::query_of(&request, "dfid").as_deref(),
        Some("DFID3")
    );
}

#[tokio::test]
async fn request_escape_hatch_merges_and_signs() {
    let transport = MockTransport::new(vec![
        MockTransport::envelope(json!(1)),
        MockTransport::envelope(json!(1)),
    ]);
    let client = client_with(Arc::clone(&transport));

    client
        .request(
            "/v9/anything",
            kugouapi::RequestOptions::new().query_one("keyword", "x"),
        )
        .await
        .unwrap();
    let request = transport.last();
    assert!(
        transport
            .requests
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .url
            .starts_with("https://gateway.kugou.com/v9/anything")
    );
    assert_eq!(
        MockTransport::query_of(&request, "keyword").as_deref(),
        Some("x")
    );
    assert_eq!(
        MockTransport::query_of(&request, "appid").as_deref(),
        Some("1005")
    );
    assert_eq!(
        MockTransport::query_of(&request, "signature").map(|s| s.len()),
        Some(32)
    );

    client
        .request(
            "/v9/raw",
            kugouapi::RequestOptions::new()
                .query_one("keyword", "x")
                .sign_none(),
        )
        .await
        .unwrap();
    let request = transport.last();
    assert!(MockTransport::query_of(&request, "signature").is_none());
}

#[tokio::test]
async fn non_envelope_json_passthrough() {
    let transport = MockTransport::new(vec![MockTransport::ok_json(json!({ "plain": true }))]);
    let client = client_with(Arc::clone(&transport));
    let data = client
        .request(
            "/x",
            kugouapi::RequestOptions::new()
                .query_one("keyword", "x")
                .sign_none(),
        )
        .await
        .unwrap();
    assert_eq!(data, json!({ "plain": true }));
}

#[tokio::test]
async fn business_error_envelope_raises_with_code_and_payload() {
    let transport = MockTransport::new(vec![MockTransport::ok_json(json!({
        "status": 0, "error_code": 30001, "error_text": "签名错误", "data": null
    }))]);
    let client = client_with(Arc::clone(&transport));
    let err = client
        .search()
        .song(SearchSongParams::new("k"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), 30001);
    assert_eq!(err.payload().unwrap()["error_text"], json!("签名错误"));
}

#[tokio::test]
async fn tracker_error_envelope_raises() {
    let transport = MockTransport::new(vec![MockTransport::ok_json(json!({
        "errcode": "20006", "status": 0, "errmsg": "err signature"
    }))]);
    let client = client_with(Arc::clone(&transport));
    let err = client
        .song()
        .url(SongUrlParams::new("abc"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), 20006);
    assert!(err.to_string().contains("err signature"));
}

#[tokio::test]
async fn http_error_raises_with_status() {
    let transport = MockTransport::new(vec![
        MockResponse {
            status: 503,
            body: b"oops".to_vec(),
            headers: Vec::new(),
        },
        MockResponse {
            status: 503,
            body: b"oops".to_vec(),
            headers: Vec::new(),
        },
    ]);
    let client = client_with(Arc::clone(&transport));
    let err = client
        .search()
        .song(SearchSongParams::new("k"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), 503);
}

#[tokio::test]
async fn non_json_response_raises() {
    let transport = MockTransport::new(vec![MockResponse {
        status: 200,
        body: b"<html>not json</html>".to_vec(),
        headers: Vec::new(),
    }]);
    let client = client_with(Arc::clone(&transport));
    assert!(
        client
            .search()
            .song(SearchSongParams::new("k"))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn catalog_dispatch_rank_list() {
    let transport = MockTransport::new(vec![MockTransport::envelope(json!([1, 2, 3]))]);
    let client = client_with(Arc::clone(&transport));
    let rank = client.call("rank.list", json!({})).await.unwrap();
    assert_eq!(rank, json!([1, 2, 3]));
    let request = transport.last();
    assert!(
        request
            .url
            .starts_with("https://gateway.kugou.com/ocean/v6/rank/list")
    );
    assert_eq!(
        MockTransport::query_of(&request, "plat").as_deref(),
        Some("2")
    );
    assert_eq!(
        MockTransport::query_of(&request, "withsong").as_deref(),
        Some("1")
    );
}

#[tokio::test]
async fn catalog_dispatch_unknown_endpoint() {
    let transport = MockTransport::new(vec![]);
    let client = client_with(Arc::clone(&transport));
    let err = client
        .call("no.such.endpoint", json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, kugouapi::Error::UnknownEndpoint(_)));
}

#[tokio::test]
async fn catalog_dispatch_dynamic_json_params() {
    let transport = MockTransport::new(vec![MockTransport::envelope(json!({}))]);
    let client = client_with(Arc::clone(&transport));
    client
        .call(
            "search",
            json!({ "keyword": "测试", "page": "2", "pagesize": 5 }),
        )
        .await
        .unwrap();
    let request = transport.last();
    assert_eq!(
        MockTransport::query_of(&request, "keyword").as_deref(),
        Some("测试")
    );
    assert_eq!(
        MockTransport::query_of(&request, "page").as_deref(),
        Some("2")
    );
    assert_eq!(
        MockTransport::query_of(&request, "pagesize").as_deref(),
        Some("5")
    );
}

#[tokio::test]
async fn set_session_restores_state() {
    let transport = MockTransport::new(vec![MockTransport::envelope(json!({
        "lists": [], "total": 0
    }))]);
    let client = client_with(Arc::clone(&transport));
    client.set_session(SessionSnapshot {
        dfid: "DFID1".into(),
        mid: "MID1".into(),
        uuid: "UUID1".into(),
        token: "TOKEN1".into(),
        userid: 42,
        vip_type: None,
        vip_token: None,
        registered_at: None,
    });
    client
        .search()
        .song(SearchSongParams::new("k"))
        .await
        .unwrap();
    let request = transport.last();
    assert_eq!(
        MockTransport::query_of(&request, "dfid").as_deref(),
        Some("DFID1")
    );
    assert_eq!(client.session().unwrap().token, "TOKEN1");
}

#[tokio::test]
async fn qq_login_flow_via_catalog() {
    // login.qq with a mocked openplat response
    let transport = MockTransport::new(vec![MockTransport::envelope(json!({
        "token": "T", "userid": 7, "secu_params": ""
    }))]);
    let client = client_with(Arc::clone(&transport));
    let data = client
        .call("login.qq", json!({ "openid": "o", "access_token": "at" }))
        .await
        .unwrap();
    assert_eq!(data["token"], "T");
    let session = client.session().unwrap();
    assert_eq!(session.token, "T");
    assert_eq!(session.userid, 7);
}

#[tokio::test]
async fn envelope_unwrap_edge_cases() {
    // 200-status envelope
    assert_eq!(
        unwrap_envelope(json!({"status": 200, "error_code": 0, "data": 5})).unwrap(),
        json!(5)
    );
    // ok without data returns the record
    assert_eq!(
        unwrap_envelope(json!({"status": 1, "error_code": 0})).unwrap(),
        json!({"status": 1, "error_code": 0})
    );
    // risk-control tracker envelope
    let err = unwrap_envelope(json!({
        "fail_process": ["pkg"], "errcode": 20028, "status": 2
    }))
    .unwrap_err();
    assert_eq!(err.code(), 20028);
}

#[tokio::test]
async fn cloudlist_playlist_sort_round_trip() {
    // playlist.sort goes through the cloudlist pipeline; the mock returns a
    // plain (non-encrypted) body which must pass through as JSON.
    let transport = MockTransport::new(vec![MockTransport::ok_json(json!({
        "status": 1, "error_code": 0
    }))]);
    let client = client_with(Arc::clone(&transport));
    let out = client
        .call(
            "playlist.sort",
            json!({ "data": "123|0|2", "total_ver": 3 }),
        )
        .await
        .unwrap();
    let _ = out;
    let request = transport.last();
    assert!(request.url.contains("/v1/modify_list_sort"));
    assert_eq!(request.method, "POST");
    // the query carries the cloudlist key + p (RSA)
    assert!(MockTransport::query_of(&request, "key").is_some());
    assert!(MockTransport::query_of(&request, "p").is_some());
    assert_eq!(
        MockTransport::header_of(&request, "x-router").as_deref(),
        Some("cloudlist.service.kugou.com")
    );
}

#[tokio::test]
async fn register_device_without_credentials_keeps_session_empty() {
    let transport = MockTransport::new(vec![MockTransport::envelope(json!({
        "nothing": 1
    }))]);
    let client = client_with(Arc::clone(&transport));
    client
        .register_device(RegisterDeviceParams::new())
        .await
        .unwrap();
    assert!(client.session().is_none());
}

#[tokio::test]
async fn query_map_helper() {
    let transport = MockTransport::new(vec![MockTransport::envelope(json!({}))]);
    let client = client_with(Arc::clone(&transport));
    client.call("rank.list", json!({})).await.unwrap();
    let requests = transport.requests.lock().unwrap().clone();
    let map = query_map(&requests[0]);
    assert_eq!(map["appid"], json!("1005"));
}
