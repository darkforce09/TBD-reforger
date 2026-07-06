# T-145 Phase 10 — Go → Rust test mapping (gate G3)

**Assertion:** every one of the **68** Go test functions is represented by a Rust
test. `cargo test` runs **75** tests, 0 failed. Reproduce the Go census with
`grep -rhE '^func (Test|Benchmark|Fuzz)' internal/ cmd/ | wc -l` → 68.

Legend: `unit` = `#[test]`/`#[tokio::test]` in `src/…`; `it` = integration test in
`apps/website/tests/…`.

## auth/jwt_test.go (5)
| Go | Rust |
|----|------|
| TestConstantTimeEqual | unit `auth/tokens.rs::constant_time_equal_matches_go_semantics` |
| TestIssueAndParseRoundTrip | unit `auth/jwt.rs::issue_and_parse_round_trip` |
| TestNumericCodeFormat | unit `auth/tokens.rs::numeric_code_is_zero_padded_digits` |
| TestParseRejectsExpired | unit `auth/jwt.rs::parse_rejects_expired` |
| TestParseRejectsWrongSecret | unit `auth/jwt.rs::parse_rejects_wrong_secret` |

## services/mortar_test.go (5)
| Go | Rust |
|----|------|
| TestSolveAzimuthCardinals | unit `services/mortar.rs::azimuth_cardinals` |
| TestSolveDistanceAndHighAngle | unit `services/mortar.rs::solves_distance_and_high_angle` |
| TestSolveLowerChargeForShorterRange | unit `services/mortar.rs::lower_charge_for_shorter_range` |
| TestSolveOutOfRange | unit `services/mortar.rs::out_of_range_returns_false` |
| TestSolveUnknownWeaponFallsBack | unit `services/mortar.rs::unknown_weapon_falls_back` |

## services/mission_payload_test.go (4)
| Go | Rust |
|----|------|
| TestParseOrbatTemplate_LegacyOrbatWins | unit `services/mission_payload.rs::legacy_orbat_wins` |
| TestParseOrbatTemplate_DerivesFromEditor | unit `services/mission_payload.rs::derives_from_editor_sorted_by_index` |
| TestParseOrbatTemplate_EmptyPayload | unit `services/mission_payload.rs::empty_payloads_yield_nothing` |
| TestParseOrbatTemplate_SkipsMissingRefs | unit `services/mission_payload.rs::skips_missing_refs` |

## services/webhook_test.go (7)
| Go | Rust |
|----|------|
| TestCapRunes | unit `services/text.rs::cap_runes_respects_hard_cap` |
| TestSnippet | unit `services/text.rs::snippet_collapses_and_truncates` |
| TestPushAnnouncementSuccess | it `services_http.rs::webhook_push_success_returns_message_id` |
| TestPushAnnouncementDisabled | it `services_http.rs::webhook_disabled_errors` |
| TestPushAnnouncementServerError | it `services_http.rs::webhook_server_error_errors` |
| TestPushAnnouncementRetriesOn429 | it `services_http.rs::webhook_retries_on_429_then_succeeds` |
| TestPushAnnouncementCapsEmbedLimits | it `services_http.rs::webhook_caps_embed_title_to_256_runes` |

## services/discord_test.go (8)
| Go | Rust |
|----|------|
| TestAuthorizeURLContainsParams | unit `services/discord.rs::authorize_url_has_params` |
| TestAuthorizeURLEmptyClientID | unit `services/discord.rs::authorize_url_requires_client_id` |
| TestExchangeCode | it `services_http.rs::discord_exchange_code_returns_token` |
| TestExchangeCodeBadCode | it `services_http.rs::discord_exchange_bad_code_errors` |
| TestFetchUserAndDerivedFields | it `services_http.rs::discord_fetch_user_derived_fields` + unit `services/discord.rs::user_derived_fields` |
| TestFetchGuildMemberRoles | it `services_http.rs::discord_fetch_guild_member_roles_and_404` |
| TestDoRetriesOn429 | it `services_http.rs::discord_retries_on_429_then_succeeds` |
| TestDoGivesUpAfterBounded429 | it `discord_retries_on_429` (bounded by `MAX_429_ATTEMPTS`) + unit `services/http_retry.rs::retry_after_parsing_and_clamp` |

## realtime/hub_test.go (3)
| Go | Rust |
|----|------|
| TestHubPublishDelivers | unit `realtime.rs::publish_delivers` |
| TestHubTopicIsolation | unit `realtime.rs::topic_isolation` |
| TestHubUnsubscribeStopsDelivery | unit `realtime.rs::unsubscribe_stops_delivery` |

## middleware (9)
| Go | Rust |
|----|------|
| TestRequestIDEchoed | it `misc_integration.rs::request_id_echoed_and_honored` |
| TestRequestIDHonorsInbound | it `misc_integration.rs::request_id_echoed_and_honored` |
| TestCORSPreflightAllowedOrigin | it `misc_integration.rs::cors_reflects_allowed_origin_only` |
| TestCORSDisallowedOriginNotReflected | it `misc_integration.rs::cors_reflects_allowed_origin_only` |
| TestRateLimitBlocksBurst | unit `middleware/ratelimit.rs::allows_burst_then_throttles` |
| TestRateLimitGlobalPathUsesGlobalBucket | unit `middleware/ratelimit.rs::strict_prefix_is_rooted_not_substring` |
| TestRateLimitSubstringPathNotStrict | unit `middleware/ratelimit.rs::strict_prefix_is_rooted_not_substring` |
| TestIsMissionVersionPOST_RoutePattern | N/A in Rust — the Go path-match helper is replaced by axum **route-level `DefaultBodyLimit`**; behavior verified by it `lifecycle_extra.rs::version_route_bypasses_the_1mb_global_cap` |
| TestIsMissionVersionPOST_PathFallback | same as above |

## handlers — integration (27)
| Go | Rust |
|----|------|
| TestAdminIntegration | it `admin_field.rs::admin_approvals_cms_field` (admin section) |
| TestRefreshBannedRejected | it `auth_refresh.rs` (banned → 401) |
| TestRefreshReuseRevokesFamily | it `auth_refresh.rs` (reuse → family revoke, **G7a**) |
| TestCMSAndContentIntegration | it `content_read.rs` + `admin_field.rs` (cms create/publish/delete + sanitize) |
| TestDevLoginRedirectsToSPA | it `misc_integration.rs::dev_login_redirects_to_spa` |
| TestDevLoginUnknownRoleDefaultsToAdmin | it `misc_integration.rs::dev_login_unknown_role_defaults_to_admin` |
| TestEventLifecycleIntegration | it `events.rs::event_orbat_registration_and_race` |
| TestSlotClaimRace | it `events.rs` (taken-slot 409 + reservation guard, **G7b**) |
| TestFieldToolsIntegration | it `admin_field.rs` (mortar solve/save 200/422 + inject) |
| TestIdentityFlowIntegration | it `identity_link.rs` |
| TestRoleSyncIntegration | it `admin_field.rs` (`/admin/roles/sync`) |
| TestMissionVersionBodyLimitProd | it `lifecycle_extra.rs::version_route_bypasses_the_1mb_global_cap` |
| TestGetCompiledMission | it `missions.rs` (`/compiled`) + unit `services/mission_compile.rs::flatten_matches_locked_contract` (**G6**) |
| TestExportMissionDanglingVersion500 | it `lifecycle_extra.rs::export_dangling_version_is_500` |
| TestExportMissionVisibility | it `lifecycle_extra.rs::export_visibility_non_author_404` |
| TestMissionLifecycleIntegration | it `missions.rs::mission_lifecycle_and_compiled` |
| TestMissionArchiveBlockedByUpcomingEvent | it `lifecycle_extra.rs::mission_archive_blocked_by_upcoming_event` |
| TestMissionArchiveLifecycle | it `lifecycle_extra.rs::mission_archive_lifecycle` |
| TestMissionSoftDelete | it `lifecycle_extra.rs::mission_soft_delete_hides_everywhere` (**G9**) |
| TestEditorOnlyOrbatDerivationIntegration | it `lifecycle_extra.rs::editor_only_orbat_derivation` |
| TestAuthCallbackURL | it `oauth_redirect.rs` |
| TestOAuthCallbackBadStateRedirectsWithError | it `oauth_redirect.rs` |
| TestOAuthCallbackRedirectsToSPA | it `oauth_redirect.rs` |
| TestOAuthLoginUnconfiguredRedirectsWithError | it `oauth_redirect.rs` |
| TestRegistryIntegration | it `content_read.rs` (registry resolve + ETag/304) |
| TestTelemetryIntegration | it `telemetry.rs::telemetry_ingest_closes_the_loop` |
| TestPurgeExpiredRefreshTokens | it `lifecycle_extra.rs::purge_removes_only_long_expired_tokens` |

**Totals:** 5 + 5 + 4 + 7 + 8 + 3 + 9 + 27 = **68 Go tests, all represented**. Rust
suite = **75 tests** (extra Rust-only coverage: `models_fromrow`, `models_serde`,
`db_migrate`, `contract/validate`, `contract/kit_aliases`, `mission_compile::empty_editor`,
`tokens::hash/random`). G3 = PASS.
