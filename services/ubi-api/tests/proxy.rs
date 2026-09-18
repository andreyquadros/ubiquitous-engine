//! End-to-end tests of the proxy against an in-process mock vendor.

mod common;

use common::{Harness, VendorMode, ADMIN_TOKEN, VENDOR_KEY, WEBHOOK_SECRET};
use serde_json::{json, Value};
use ubiqx_core::license::Plan;

fn header(resp: &reqwest::Response, name: &str) -> Option<String> {
    resp.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

#[tokio::test]
async fn healthz_is_alive_without_naming_the_vendor() {
    let h = Harness::start().await;
    let r = h.http.get(h.url("/healthz")).send().await.unwrap();
    let body = r.text().await.unwrap();
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["service"], "ubi-api");
    assert!(v.get("version").is_some());
    // A public probe never says which models or vendor sit behind the aliases.
    assert!(
        v.get("models").is_none() && v.get("vendor").is_none(),
        "{body}"
    );
    assert!(
        !body.contains("mock-fast-model") && !body.contains("mock-smart-model"),
        "{body}"
    );
}

#[tokio::test]
async fn admin_models_names_them_for_the_operator_only() {
    let h = Harness::start().await;
    let r = h.http.get(h.url("/admin/models")).send().await.unwrap();
    assert_eq!(r.status(), 401, "no token, no models");
    let v: Value = h
        .http
        .get(h.url("/admin/models"))
        .bearer_auth(ADMIN_TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["models"]["ubi-fast"], "mock-fast-model");
    assert_eq!(v["models"]["ubi-smart"], "mock-smart-model");
}

#[tokio::test]
async fn passthrough_maps_aliases_records_usage_and_reports_cost() {
    let h = Harness::start().await;
    h.vendor.set_mode(VendorMode::Json {
        input_tokens: 1_000_000,
        output_tokens: 100_000,
        cache_read: 0,
    });
    let key = h.managed_key("sub_a");

    let resp = h
        .post_messages(&key, &Harness::messages_body("ubi-fast", false))
        .await;
    assert_eq!(resp.status(), 200);
    // Unknown model ids are billed at the most expensive configured price (sonnet 5: 2/10).
    assert_eq!(
        header(&resp, "x-ubiqx-cost-usd").as_deref(),
        Some("3.000000")
    );
    assert_eq!(
        header(&resp, "x-ubiqx-spent-usd").as_deref(),
        Some("3.000000")
    );
    assert_eq!(
        header(&resp, "x-ubiqx-budget-usd").as_deref(),
        Some("6.000000")
    );
    assert_eq!(header(&resp, "request-id").as_deref(), Some("req_mock_1"));
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["id"], "msg_mock");
    assert_eq!(body["usage"]["input_tokens"], 1_000_000);

    // The vendor saw the real model, the server key and the version header; never the license.
    let call = h.vendor.calls().pop().unwrap();
    assert_eq!(call.body["model"], "mock-fast-model");
    assert_eq!(call.body["max_tokens"], 256);
    let hdr = |n: &str| {
        call.headers
            .iter()
            .find(|(k, _)| k == n)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(hdr("x-api-key").as_deref(), Some(VENDOR_KEY));
    assert_eq!(hdr("anthropic-version").as_deref(), Some("2023-06-01"));
    assert!(hdr("x-ubiqx-plan").is_none());

    let resp = h
        .post_messages(&key, &Harness::messages_body("ubi-smart", false))
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(h.vendor.last_model(), "mock-smart-model");
    assert_eq!(
        header(&resp, "x-ubiqx-spent-usd").as_deref(),
        Some("6.000000")
    );

    let month = ubi_api::db::month_of(chrono::Utc::now());
    assert_eq!(h.state.db.spent_usd("sub_a", &month).unwrap(), 6.0);
    let rows = h.state.db.usage_by_sub(&month).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].requests, 2);
    assert_eq!(rows[0].input_tokens, 2_000_000);
    assert_eq!(rows[0].output_tokens, 200_000);
}

#[tokio::test]
async fn configured_prices_apply_to_the_mapped_model() {
    let h = Harness::start_with(|c| {
        c.model_fast = "claude-haiku-4-5-20251001".into();
    })
    .await;
    h.vendor.set_mode(VendorMode::Json {
        input_tokens: 1_000_000,
        output_tokens: 100_000,
        cache_read: 1_000_000,
    });
    let resp = h
        .post_messages(
            &h.managed_key("sub_p"),
            &Harness::messages_body("ubi-fast", false),
        )
        .await;
    assert_eq!(resp.status(), 200);
    // 1.0 (input) + 0.5 (output) + 0.1 (cache read at 10 %)
    assert_eq!(
        header(&resp, "x-ubiqx-cost-usd").as_deref(),
        Some("1.600000")
    );
}

#[tokio::test]
async fn unknown_model_ids_are_rejected_before_reaching_the_vendor() {
    let h = Harness::start().await;
    let key = h.managed_key("sub_m");
    for model in ["claude-sonnet-5", "gpt-5", "", "ubi-fastest"] {
        let resp = h
            .post_messages(&key, &Harness::messages_body(model, false))
            .await;
        assert_eq!(resp.status(), 400, "model {model:?}");
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "error");
        assert_eq!(body["error"]["type"], "invalid_request_error");
    }
    assert!(h.vendor.calls().is_empty());

    let resp = h
        .http
        .post(h.url("/v1/messages"))
        .header("x-api-key", &key)
        .body("not json")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn budget_cut_off_answers_402() {
    let h = Harness::start_with(|c| c.monthly_budget_usd = 5.0).await;
    // 1M in + 100k out on an unknown model = 2 + 1 = 3 USD per call.
    h.vendor.set_mode(VendorMode::Json {
        input_tokens: 1_000_000,
        output_tokens: 100_000,
        cache_read: 0,
    });
    let key = h.managed_key("sub_b");
    let body = Harness::messages_body("ubi-fast", false);
    assert_eq!(h.post_messages(&key, &body).await.status(), 200); // spent 3
    assert_eq!(h.post_messages(&key, &body).await.status(), 200); // spent 6 ≥ 5
    let resp = h.post_messages(&key, &body).await;
    assert_eq!(resp.status(), 402);
    let err: Value = resp.json().await.unwrap();
    assert_eq!(err["type"], "error");
    assert_eq!(err["error"]["type"], "budget_exhausted");
    assert_eq!(err["budget_usd"], 5.0);
    assert_eq!(err["spent_usd"], 6.0);
    assert_eq!(h.vendor.calls().len(), 2);

    // Another subscriber is unaffected.
    assert_eq!(
        h.post_messages(&h.managed_key("sub_c"), &body)
            .await
            .status(),
        200
    );
}

#[tokio::test]
async fn streaming_passes_sse_through_and_records_usage_afterwards() {
    let h = Harness::start_with(|c| c.model_smart = "claude-sonnet-5".into()).await;
    h.vendor.set_mode(VendorMode::Sse {
        input_tokens: 500_000,
        output_tokens: 100_000,
    });
    let key = h.managed_key("sub_s");
    let resp = h
        .post_messages(&key, &Harness::messages_body("ubi-smart", true))
        .await;
    assert_eq!(resp.status(), 200);
    assert!(header(&resp, "content-type")
        .unwrap()
        .starts_with("text/event-stream"));
    assert!(
        header(&resp, "x-ubiqx-cost-usd").is_none(),
        "cost is only known once the stream ends"
    );
    let text = resp.text().await.unwrap();
    assert!(text.contains("event: message_start"));
    assert!(text.contains("\"text\":\"Olá\""));
    assert!(text.ends_with("event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"));

    // The ledger is written when the stream completes.
    let month = ubi_api::db::month_of(chrono::Utc::now());
    let mut spent = 0.0;
    for _ in 0..50 {
        spent = h.state.db.spent_usd("sub_s", &month).unwrap();
        if spent > 0.0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    // sonnet 5: 0.5M × 2 + 0.1M × 10 = 1 + 1
    assert!((spent - 2.0).abs() < 1e-9, "{spent}");
}

#[tokio::test]
async fn vendor_errors_are_passed_through() {
    let h = Harness::start().await;
    h.vendor.set_mode(VendorMode::Error(529));
    let resp = h
        .post_messages(
            &h.managed_key("sub_e"),
            &Harness::messages_body("ubi-fast", false),
        )
        .await;
    assert_eq!(resp.status(), 529);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["type"], "overloaded_error");
    let month = ubi_api::db::month_of(chrono::Utc::now());
    assert_eq!(h.state.db.spent_usd("sub_e", &month).unwrap(), 0.0);
}

#[tokio::test]
async fn license_checks_on_messages() {
    let h = Harness::start().await;
    let body = Harness::messages_body("ubi-fast", false);
    let expect = |status: u16, kind: &'static str| {
        move |resp: reqwest::Response| async move {
            assert_eq!(resp.status(), status);
            let v: Value = resp.json().await.unwrap();
            assert_eq!(v["error"]["type"], kind, "{v}");
        }
    };

    // Missing / malformed / foreign-key signatures.
    let resp = h
        .http
        .post(h.url("/v1/messages"))
        .json(&body)
        .send()
        .await
        .unwrap();
    expect(401, "authentication_error")(resp).await;
    expect(401, "license_invalid")(h.post_messages("UBIQX-NOPE-NOPE", &body).await).await;
    let other = Harness::start().await; // different key pair
    expect(401, "license_invalid")(h.post_messages(&other.managed_key("sub_x"), &body).await).await;

    // Expired, wrong plan, revoked.
    expect(403, "license_expired")(
        h.post_messages(&h.key(Plan::MonthlyManaged, "sub_exp", -1), &body)
            .await,
    )
    .await;
    expect(403, "plan_not_managed")(
        h.post_messages(&h.key(Plan::AnnualOwnKey, "sub_ann", 300), &body)
            .await,
    )
    .await;
    h.state
        .db
        .revoke("sub_rev", None, chrono::Utc::now())
        .unwrap();
    expect(403, "license_revoked")(h.post_messages(&h.managed_key("sub_rev"), &body).await).await;

    // Wrong plan header.
    let resp = h
        .http
        .post(h.url("/v1/messages"))
        .header("x-api-key", h.managed_key("sub_ok"))
        .header("x-ubiqx-plan", "annual_own_key")
        .json(&body)
        .send()
        .await
        .unwrap();
    expect(400, "invalid_request_error")(resp).await;

    // The Authorization: Bearer form works too.
    let resp = h
        .http
        .post(h.url("/v1/messages"))
        .bearer_auth(h.managed_key("sub_ok"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(h.vendor.calls().len(), 1);
}

#[tokio::test]
async fn license_status_route() {
    let h = Harness::start_with(|c| c.monthly_budget_usd = 6.0).await;
    let month = ubi_api::db::month_of(chrono::Utc::now());

    let resp = h
        .http
        .get(h.url("/v1/license/status"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    let v: Value = resp.json().await.unwrap();
    assert_eq!(v["state"], "invalid");

    let resp = h
        .http
        .get(h.url("/v1/license/status"))
        .header("x-api-key", "UBIQX-BAD-KEY")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    assert_eq!(resp.json::<Value>().await.unwrap()["state"], "invalid");

    let key = h.managed_key("sub_st");
    h.state
        .db
        .record_usage(&ubi_api::db::UsageRow {
            sub: "sub_st".into(),
            model: "m".into(),
            usage: Default::default(),
            cost_usd: 1.25,
            at: chrono::Utc::now(),
        })
        .unwrap();
    let resp = h
        .http
        .get(h.url("/v1/license/status"))
        .header("x-api-key", &key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: Value = resp.json().await.unwrap();
    assert_eq!(v["state"], "valid");
    assert_eq!(v["plan"], "monthly_managed");
    assert_eq!(v["month"], month);
    assert_eq!(v["spent_usd"], 1.25);
    assert_eq!(v["budget_usd"], 6.0);
    let days_left = v["days_left"].as_i64().unwrap();
    assert!((29..=30).contains(&days_left), "{days_left}");
    assert_eq!(v["key_hint"], key[key.len() - 4..]);
    assert!(v["expires_at"].as_str().unwrap().ends_with("+00:00"));

    let expired = h.key(Plan::MonthlyManaged, "sub_old", -5);
    let v: Value = h
        .http
        .get(h.url("/v1/license/status"))
        .header("x-api-key", &expired)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["state"], "expired");
    assert_eq!(v["days_left"], 0);

    let annual = h.key(Plan::AnnualOwnKey, "sub_ann", 200);
    let v: Value = h
        .http
        .get(h.url("/v1/license/status"))
        .header("x-api-key", &annual)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["state"], "valid");
    assert_eq!(v["plan"], "annual_own_key");
    assert_eq!(v["budget_usd"], 0.0);

    h.state
        .db
        .revoke("sub_st", None, chrono::Utc::now())
        .unwrap();
    let v: Value = h
        .http
        .get(h.url("/v1/license/status"))
        .header("x-api-key", &key)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["state"], "revoked");
}

#[tokio::test]
async fn admin_routes_need_the_bearer() {
    let h = Harness::start().await;
    let resp = h.http.get(h.url("/admin/usage")).send().await.unwrap();
    assert_eq!(resp.status(), 401);
    let resp = h
        .http
        .get(h.url("/admin/usage"))
        .bearer_auth("wrong")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    let resp = h
        .http
        .post(h.url("/admin/licenses/revoke"))
        .json(&json!({"sub": "x"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // Usage listing.
    h.vendor.set_mode(VendorMode::Json {
        input_tokens: 10,
        output_tokens: 5,
        cache_read: 0,
    });
    h.post_messages(
        &h.managed_key("sub_u1"),
        &Harness::messages_body("ubi-fast", false),
    )
    .await;
    h.post_messages(
        &h.managed_key("sub_u1"),
        &Harness::messages_body("ubi-fast", false),
    )
    .await;
    h.post_messages(
        &h.managed_key("sub_u2"),
        &Harness::messages_body("ubi-smart", false),
    )
    .await;
    let month = ubi_api::db::month_of(chrono::Utc::now());
    let v: Value = h
        .http
        .get(h.url(&format!("/admin/usage?month={month}")))
        .bearer_auth(ADMIN_TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["month"], month);
    assert_eq!(v["subscribers"].as_array().unwrap().len(), 2);
    let u1 = v["subscribers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["sub"] == "sub_u1")
        .unwrap();
    assert_eq!(u1["requests"], 2);
    assert_eq!(u1["input_tokens"], 20);
    assert!(v["total_cost_usd"].as_f64().unwrap() > 0.0);
    let resp = h
        .http
        .get(h.url("/admin/usage?month=2026-13"))
        .bearer_auth(ADMIN_TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Revocation takes effect immediately.
    let resp = h
        .http
        .post(h.url("/admin/licenses/revoke"))
        .bearer_auth(ADMIN_TOKEN)
        .json(&json!({"sub": "sub_u1", "reason": "chargeback"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let resp = h
        .post_messages(
            &h.managed_key("sub_u1"),
            &Harness::messages_body("ubi-fast", false),
        )
        .await;
    assert_eq!(resp.status(), 403);

    // Admin disabled when no token is configured.
    let h2 = Harness::start_with(|c| c.admin_token = None).await;
    let resp = h2
        .http
        .get(h2.url("/admin/usage"))
        .bearer_auth(ADMIN_TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 503);
}

#[tokio::test]
async fn webhook_issues_renews_and_revokes() {
    let h = Harness::start().await;
    let created = json!({
        "event": "subscription.created",
        "plan": "monthly_managed",
        "email": "Cliente@Exemplo.com",
        "months": 1,
        "external_id": "mp_sub_123"
    });

    // Wrong or missing signature.
    let resp = h.webhook(&created, "not-the-secret").await;
    assert_eq!(resp.status(), 401);
    let resp = h
        .http
        .post(h.url("/admin/webhooks/generic"))
        .json(&created)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // Created: a key comes back, verifies with the app's logic and works on the proxy.
    let resp = h.webhook(&created, WEBHOOK_SECRET).await;
    assert_eq!(resp.status(), 200);
    let v: Value = resp.json().await.unwrap();
    let key = v["key"].as_str().unwrap().to_string();
    let sub = v["sub"].as_str().unwrap().to_string();
    assert_eq!(v["plan"], "monthly_managed");
    assert_eq!(
        v["email_hash"],
        ubiqx_core::license::email_hash("cliente@exemplo.com")
    );
    let claims = ubiqx_core::license::verify_key_with(&key, &h.pair.public_hex).unwrap();
    assert_eq!(claims.plan, Plan::MonthlyManaged);
    assert_eq!(claims.sub, sub);
    assert!(!claims.is_expired_at(chrono::Utc::now()));
    assert!(claims.days_left_at(chrono::Utc::now()) >= 27);
    assert_eq!(
        h.post_messages(&key, &Harness::messages_body("ubi-fast", false))
            .await
            .status(),
        200
    );

    // Cancelled: the same subscriber is revoked, even though the key itself is still valid.
    let cancelled = json!({"event": "subscription.cancelled", "external_id": "mp_sub_123"});
    let v: Value = h
        .webhook(&cancelled, WEBHOOK_SECRET)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(v["sub"], sub);
    assert_eq!(v["revoked"], true);
    assert_eq!(
        h.post_messages(&key, &Harness::messages_body("ubi-fast", false))
            .await
            .status(),
        403
    );

    // Renewed: same sub, new key, revocation lifted.
    let renewed = json!({
        "event": "subscription.renewed",
        "plan": "monthly_managed",
        "email": "cliente@exemplo.com",
        "months": 2,
        "external_id": "mp_sub_123"
    });
    let v: Value = h
        .webhook(&renewed, WEBHOOK_SECRET)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(v["sub"], sub);
    assert_eq!(v["months"], 2);
    let key2 = v["key"].as_str().unwrap().to_string();
    assert_ne!(key, key2);
    assert_eq!(
        h.post_messages(&key2, &Harness::messages_body("ubi-fast", false))
            .await
            .status(),
        200
    );
    assert_eq!(h.state.db.license_events(&sub).unwrap(), 2);

    // Annual plan defaults to 12 months and cannot use the proxy.
    let annual =
        json!({"event": "subscription.created", "plan": "annual_own_key", "email": "a@b.c"});
    let v: Value = h
        .webhook(&annual, WEBHOOK_SECRET)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(v["months"], 12);
    let akey = v["key"].as_str().unwrap();
    assert_eq!(
        h.post_messages(akey, &Harness::messages_body("ubi-fast", false))
            .await
            .status(),
        403
    );

    // Validation.
    let resp = h
        .webhook(
            &json!({"event": "subscription.created", "plan": "gold", "email": "a@b.c"}),
            WEBHOOK_SECRET,
        )
        .await;
    assert_eq!(resp.status(), 400);
    let resp = h
        .webhook(
            &json!({"event": "subscription.created", "plan": "monthly_managed"}),
            WEBHOOK_SECRET,
        )
        .await;
    assert_eq!(resp.status(), 400);
    let resp = h
        .webhook(&json!({"event": "subscription.cancelled"}), WEBHOOK_SECRET)
        .await;
    assert_eq!(resp.status(), 400);
    let resp = h
        .webhook(&json!({"event": "payment.failed"}), WEBHOOK_SECRET)
        .await;
    assert_eq!(resp.status(), 400);

    // Issuing disabled without the private key.
    let h2 = Harness::start_with(|c| c.license_privkey_hex = None).await;
    let resp = h2.webhook(&created, WEBHOOK_SECRET).await;
    assert_eq!(resp.status(), 503);
    let h3 = Harness::start_with(|c| c.webhook_secret = None).await;
    let resp = h3.webhook(&created, WEBHOOK_SECRET).await;
    assert_eq!(resp.status(), 503);
}
