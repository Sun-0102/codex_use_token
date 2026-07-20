use codex_reserve_lib::app_server_protocol::{
    AccountReadParams, AccountReadResponse, AccountUsageReadResponse, ClientRequest,
    InitializeParams, InitializeResponse, RateLimitsReadResponse, RequestId, ServerResponse,
    method,
};

fn fixture(name: &str) -> &'static str {
    match name {
        "initialize_request" => {
            include_str!("fixtures/app_server_protocol/0.144.5/requests/initialize.json")
        }
        "account_read_request" => {
            include_str!("fixtures/app_server_protocol/0.144.5/requests/account-read.json")
        }
        "rate_limits_request" => {
            include_str!("fixtures/app_server_protocol/0.144.5/requests/rate-limits-read.json")
        }
        "account_usage_request" => {
            include_str!("fixtures/app_server_protocol/0.144.5/requests/account-usage-read.json")
        }
        "initialize_response" => {
            include_str!("fixtures/app_server_protocol/0.144.5/responses/initialize.json")
        }
        "account_read_unknown_plan" => include_str!(
            "fixtures/app_server_protocol/0.144.5/responses/account-read-unknown-plan.json"
        ),
        "account_read_signed_out" => include_str!(
            "fixtures/app_server_protocol/0.144.5/responses/account-read-signed-out.json"
        ),
        "rate_limits_multiple" => {
            include_str!("fixtures/app_server_protocol/0.144.5/responses/rate-limits-multiple.json")
        }
        "rate_limits_sparse" => {
            include_str!("fixtures/app_server_protocol/0.144.5/responses/rate-limits-sparse.json")
        }
        "account_usage" => {
            include_str!("fixtures/app_server_protocol/0.144.5/responses/account-usage.json")
        }
        "account_usage_sparse" => {
            include_str!("fixtures/app_server_protocol/0.144.5/responses/account-usage-sparse.json")
        }
        _ => panic!("unknown fixture: {name}"),
    }
}

#[test]
fn checked_in_schemas_are_valid_generated_json() {
    let schemas = [
        (
            "InitializeParams",
            include_str!("fixtures/app_server_protocol/0.144.5/schemas/InitializeParams.json"),
        ),
        (
            "InitializeResponse",
            include_str!("fixtures/app_server_protocol/0.144.5/schemas/InitializeResponse.json"),
        ),
        (
            "GetAccountParams",
            include_str!("fixtures/app_server_protocol/0.144.5/schemas/GetAccountParams.json"),
        ),
        (
            "GetAccountResponse",
            include_str!("fixtures/app_server_protocol/0.144.5/schemas/GetAccountResponse.json"),
        ),
        (
            "GetAccountRateLimitsResponse",
            include_str!(
                "fixtures/app_server_protocol/0.144.5/schemas/GetAccountRateLimitsResponse.json"
            ),
        ),
        (
            "GetAccountTokenUsageResponse",
            include_str!(
                "fixtures/app_server_protocol/0.144.5/schemas/GetAccountTokenUsageResponse.json"
            ),
        ),
    ];

    for (expected_title, schema) in schemas {
        let value: serde_json::Value = serde_json::from_str(schema).expect("valid schema JSON");
        assert_eq!(value["title"], expected_title);
    }
}

#[test]
fn initialize_request_matches_generated_protocol_shape() {
    let request: ClientRequest<InitializeParams> =
        serde_json::from_str(fixture("initialize_request")).expect("initialize request fixture");

    assert_eq!(request.id, RequestId::Integer(1));
    assert_eq!(request.method, method::INITIALIZE);
    let params = request.params.expect("initialize params");
    assert_eq!(params.client_info.name, "codex-reserve");
    assert_eq!(params.client_info.version, "0.1.0");
    assert_eq!(
        params.capabilities.expect("capabilities").experimental_api,
        Some(true)
    );
}

#[test]
fn account_request_methods_match_the_generated_protocol() {
    let account: ClientRequest<AccountReadParams> =
        serde_json::from_str(fixture("account_read_request")).expect("account/read request");
    let rate_limits: ClientRequest<serde_json::Value> =
        serde_json::from_str(fixture("rate_limits_request"))
            .expect("account/rateLimits/read request");
    let usage: ClientRequest<serde_json::Value> =
        serde_json::from_str(fixture("account_usage_request")).expect("account/usage/read request");

    assert_eq!(account.method, method::ACCOUNT_READ);
    assert_eq!(rate_limits.method, method::ACCOUNT_RATE_LIMITS_READ);
    assert_eq!(usage.method, method::ACCOUNT_USAGE_READ);
    assert_eq!(
        account.params.expect("account/read params").refresh_token,
        None
    );
    assert!(rate_limits.params.is_none());
    assert!(usage.params.is_none());
}

#[test]
fn initialize_response_tolerates_future_fields() {
    let response: ServerResponse<InitializeResponse> =
        serde_json::from_str(fixture("initialize_response")).expect("initialize response fixture");

    assert_eq!(response.id, RequestId::Integer(1));
    assert_eq!(
        response.result.user_agent.as_deref(),
        Some("codex-cli/0.144.5")
    );
    assert_eq!(response.result.platform_os.as_deref(), Some("macos"));
}

#[test]
fn account_read_preserves_an_unknown_plan() {
    let response: ServerResponse<AccountReadResponse> =
        serde_json::from_str(fixture("account_read_unknown_plan"))
            .expect("account response with unknown plan");

    let account = response.result.account.expect("signed-in account");
    assert_eq!(account.account_type, "chatgpt");
    assert_eq!(
        account.plan_type.expect("plan type").as_str(),
        "future_super_plan"
    );
    assert_eq!(account.email.as_deref(), Some("fixture@example.invalid"));
}

#[test]
fn account_read_accepts_missing_optional_account() {
    let response: ServerResponse<AccountReadResponse> =
        serde_json::from_str(fixture("account_read_signed_out"))
            .expect("signed-out account response");

    assert!(response.result.account.is_none());
    assert!(response.result.requires_openai_auth);

    let params: AccountReadParams = serde_json::from_str("{}").expect("empty account params");
    assert_eq!(params.refresh_token, None);
}

#[test]
fn rate_limits_read_keeps_every_limit_id_and_nullable_credits() {
    let response: ServerResponse<RateLimitsReadResponse> =
        serde_json::from_str(fixture("rate_limits_multiple"))
            .expect("multi-bucket rate limits response");

    let buckets = response
        .result
        .rate_limits_by_limit_id
        .expect("rateLimitsByLimitId");
    assert_eq!(
        buckets.keys().cloned().collect::<Vec<_>>(),
        ["codex", "reviews"]
    );
    assert_eq!(
        buckets["codex"]
            .primary
            .as_ref()
            .expect("primary window")
            .used_percent,
        27
    );
    assert!(buckets["reviews"].credits.is_none());
    assert_eq!(
        buckets["reviews"]
            .plan_type
            .as_ref()
            .expect("unknown plan")
            .as_str(),
        "org_preview"
    );
}

#[test]
fn rate_limits_read_accepts_missing_optional_snapshot_fields() {
    let response: ServerResponse<RateLimitsReadResponse> =
        serde_json::from_str(fixture("rate_limits_sparse")).expect("sparse rate limits response");

    assert!(response.result.rate_limits.primary.is_none());
    assert!(response.result.rate_limits.credits.is_none());
    assert!(response.result.rate_limits_by_limit_id.is_none());
}

#[test]
fn account_usage_read_deserializes_summary_and_daily_buckets() {
    let response: ServerResponse<AccountUsageReadResponse> =
        serde_json::from_str(fixture("account_usage")).expect("account usage response");

    assert_eq!(response.result.summary.lifetime_tokens, Some(1_234_567));
    assert_eq!(response.result.summary.peak_daily_tokens, Some(98_765));
    assert_eq!(
        response.result.daily_usage_buckets.expect("daily buckets")[0].tokens,
        12_345
    );
}

#[test]
fn account_usage_read_accepts_missing_optional_fields() {
    let response: ServerResponse<AccountUsageReadResponse> =
        serde_json::from_str(fixture("account_usage_sparse")).expect("sparse usage response");

    assert_eq!(response.result.summary.lifetime_tokens, None);
    assert!(response.result.daily_usage_buckets.is_none());
}

#[test]
fn string_request_ids_are_supported() {
    let response = ServerResponse {
        id: RequestId::String("account-usage-1".to_owned()),
        result: AccountUsageReadResponse {
            summary: Default::default(),
            daily_usage_buckets: None,
        },
    };

    let value = serde_json::to_value(response).expect("serializable protocol response");
    assert_eq!(value["id"], "account-usage-1");
}
