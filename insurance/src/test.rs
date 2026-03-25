#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as AddressTrait, Ledger, LedgerInfo},
    Address, Env, String,
};

fn setup_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn set_time(env: &Env, timestamp: u64) {
    let proto = env.ledger().protocol_version();
    env.ledger().set(LedgerInfo {
        protocol_version: proto,
        sequence_number: 1,
        timestamp,
        network_id: [0u8; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 100_000,
    });
}

fn new_client(env: &Env) -> (Address, InsuranceClient) {
    let contract_id = env.register_contract(None, Insurance);
    let client = InsuranceClient::new(env, &contract_id);
    (contract_id, client)
}

// ── create_policy ───────────────────────────────────────────────────────────

#[test]
fn test_create_policy_succeeds() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Health Policy"),
        &CoverageType::Health,
        &100,
        &10000,
        &None,
    );

    assert_eq!(policy_id, 1);

    let policy = client.get_policy(&policy_id).unwrap();
    assert_eq!(policy.owner, owner);
    assert_eq!(policy.monthly_premium, 100);
    assert_eq!(policy.coverage_amount, 10000);
    assert!(policy.active);
    assert!(policy.external_ref.is_none());
}

#[test]
fn test_create_policy_with_external_ref() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let ext_ref = soroban_sdk::String::from_str(&env, "REF-001");
    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Life Policy"),
        &CoverageType::Life,
        &200,
        &20000,
        &Some(ext_ref.clone()),
    );

    let policy = client.get_policy(&policy_id).unwrap();
    assert_eq!(policy.external_ref, Some(ext_ref));
}

#[test]
fn test_create_policy_increments_id() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let id1 = client.create_policy(
        &owner,
        &String::from_str(&env, "P1"),
        &CoverageType::Auto,
        &100,
        &5000,
        &None,
    );
    let id2 = client.create_policy(
        &owner,
        &String::from_str(&env, "P2"),
        &CoverageType::Auto,
        &100,
        &5000,
        &None,
    );

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn test_create_policy_invalid_premium_zero() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let result = client.try_create_policy(
        &owner,
        &String::from_str(&env, "Bad"),
        &CoverageType::Health,
        &0,
        &10000,
        &None,
    );

    assert!(result.is_err());
}

#[test]
fn test_create_policy_invalid_premium_negative() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let result = client.try_create_policy(
        &owner,
        &String::from_str(&env, "Bad"),
        &CoverageType::Health,
        &-1,
        &10000,
        &None,
    );

    assert!(result.is_err());
}

#[test]
fn test_create_policy_invalid_coverage_zero() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let result = client.try_create_policy(
        &owner,
        &String::from_str(&env, "Bad"),
        &CoverageType::Health,
        &100,
        &0,
        &None,
    );

    assert!(result.is_err());
}

#[test]
fn test_create_policy_invalid_coverage_negative() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let result = client.try_create_policy(
        &owner,
        &String::from_str(&env, "Bad"),
        &CoverageType::Health,
        &100,
        &-1,
        &None,
    );

    assert!(result.is_err());
}

#[test]
fn test_create_policy_requires_auth() {
    let env = Env::default();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    // Do NOT mock auth — should fail
    let result = client.try_create_policy(
        &owner,
        &String::from_str(&env, "Policy"),
        &CoverageType::Health,
        &100,
        &10000,
        &None,
    );

    assert!(result.is_err());
}

// ── pay_premium ─────────────────────────────────────────────────────────────

#[test]
fn test_pay_premium_updates_next_payment_date() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Policy"),
        &CoverageType::Health,
        &100,
        &10000,
        &None,
    );

    let initial_policy = client.get_policy(&policy_id).unwrap();
    let initial_due = initial_policy.next_payment_date;

    // Advance time by 1000 seconds
    set_time(&env, env.ledger().timestamp() + 1000);

    client.pay_premium(&owner, &policy_id);

    let updated_policy = client.get_policy(&policy_id).unwrap();
    assert!(updated_policy.next_payment_date > initial_due);
    assert_eq!(
        updated_policy.next_payment_date,
        env.ledger().timestamp() + 30 * 86400
    );
}

#[test]
fn test_pay_premium_policy_not_found() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let result = client.try_pay_premium(&owner, &999);
    assert_eq!(result, Err(Ok(InsuranceError::PolicyNotFound)));
}

#[test]
fn test_pay_premium_unauthorized() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Policy"),
        &CoverageType::Life,
        &100,
        &10000,
        &None,
    );

    let result = client.try_pay_premium(&other, &policy_id);
    assert_eq!(result, Err(Ok(InsuranceError::Unauthorized)));
}

#[test]
fn test_pay_premium_inactive_policy() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Policy"),
        &CoverageType::Health,
        &100,
        &10000,
        &None,
    );
    client.deactivate_policy(&owner, &policy_id);

    let result = client.try_pay_premium(&owner, &policy_id);
    assert_eq!(result, Err(Ok(InsuranceError::PolicyInactive)));
}

#[test]
fn test_pay_premium_multiple_times() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "LongTerm"),
        &CoverageType::Life,
        &100,
        &10000,
        &None,
    );

    let p1 = client.get_policy(&policy_id).unwrap();
    let first_due = p1.next_payment_date;

    client.pay_premium(&owner, &policy_id);
    set_time(&env, env.ledger().timestamp() + 5000);
    client.pay_premium(&owner, &policy_id);

    let p2 = client.get_policy(&policy_id).unwrap();
    assert!(p2.next_payment_date > first_due);
    assert_eq!(
        p2.next_payment_date,
        env.ledger().timestamp() + 30 * 86400
    );
}

// ── batch_pay_premiums ───────────────────────────────────────────────────────

#[test]
fn test_batch_pay_premiums_success() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let id1 = client.create_policy(
        &owner,
        &String::from_str(&env, "P1"),
        &CoverageType::Health,
        &100,
        &5000,
        &None,
    );
    let id2 = client.create_policy(
        &owner,
        &String::from_str(&env, "P2"),
        &CoverageType::Life,
        &200,
        &10000,
        &None,
    );

    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(id1);
    ids.push_back(id2);

    let count = client.batch_pay_premiums(&owner, &ids);
    assert_eq!(count, 2);
}

#[test]
fn test_batch_pay_premiums_too_large() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let mut ids_vec = soroban_sdk::Vec::new(&env);
    for _ in 0..51u32 {
        let id = client.create_policy(
            &owner,
            &String::from_str(&env, "P"),
            &CoverageType::Health,
            &100,
            &5000,
            &None,
        );
        ids_vec.push_back(id);
    }

    let result = client.try_batch_pay_premiums(&owner, &ids_vec);
    assert_eq!(result, Err(Ok(InsuranceError::BatchTooLarge)));
}

// ── deactivate_policy ────────────────────────────────────────────────────────

#[test]
fn test_deactivate_policy_success() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Policy"),
        &CoverageType::Property,
        &100,
        &10000,
        &None,
    );

    let success = client.deactivate_policy(&owner, &policy_id);
    assert!(success);

    let policy = client.get_policy(&policy_id).unwrap();
    assert!(!policy.active);
}

#[test]
fn test_deactivate_policy_unauthorized() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Policy"),
        &CoverageType::Auto,
        &100,
        &10000,
        &None,
    );

    let result = client.try_deactivate_policy(&other, &policy_id);
    assert_eq!(result, Err(Ok(InsuranceError::Unauthorized)));
}

#[test]
fn test_deactivate_policy_not_found() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let result = client.try_deactivate_policy(&owner, &999);
    assert_eq!(result, Err(Ok(InsuranceError::PolicyNotFound)));
}

// ── get_active_policies ──────────────────────────────────────────────────────

#[test]
fn test_get_active_policies_returns_active_only() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let id1 = client.create_policy(
        &owner,
        &String::from_str(&env, "P1"),
        &CoverageType::Health,
        &100,
        &1000,
        &None,
    );
    let id2 = client.create_policy(
        &owner,
        &String::from_str(&env, "P2"),
        &CoverageType::Life,
        &200,
        &2000,
        &None,
    );
    client.create_policy(
        &owner,
        &String::from_str(&env, "P3"),
        &CoverageType::Auto,
        &300,
        &3000,
        &None,
    );

    // Deactivate P2
    client.deactivate_policy(&owner, &id2);

    let active = client.get_active_policies(&owner);
    assert_eq!(active.len(), 2);

    // All returned policies must be active
    for p in active.iter() {
        assert!(p.active);
        assert_eq!(p.owner, owner);
    }
    // P1 and P3 should be there, P2 should not
    let mut found_id1 = false;
    let mut found_id2 = false;
    for p in active.iter() {
        if p.id == id1 { found_id1 = true; }
        if p.id == id2 { found_id2 = true; }
    }
    assert!(found_id1, "P1 must be in active list");
    assert!(!found_id2, "P2 (deactivated) must NOT be in active list");
}

#[test]
fn test_get_active_policies_filters_by_owner() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner_a = Address::generate(&env);
    let owner_b = Address::generate(&env);

    client.create_policy(
        &owner_a,
        &String::from_str(&env, "PolicyA"),
        &CoverageType::Health,
        &100,
        &10000,
        &None,
    );
    client.create_policy(
        &owner_b,
        &String::from_str(&env, "PolicyB"),
        &CoverageType::Life,
        &200,
        &20000,
        &None,
    );

    let active_a = client.get_active_policies(&owner_a);
    assert_eq!(active_a.len(), 1);
    assert_eq!(active_a.get(0).unwrap().owner, owner_a);

    let active_b = client.get_active_policies(&owner_b);
    assert_eq!(active_b.len(), 1);
    assert_eq!(active_b.get(0).unwrap().owner, owner_b);
}

#[test]
fn test_get_active_policies_empty_for_new_user() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let active = client.get_active_policies(&owner);
    assert_eq!(active.len(), 0);
}

// ── get_total_monthly_premium ────────────────────────────────────────────────

#[test]
fn test_get_total_monthly_premium_sums_active_policies() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    client.create_policy(
        &owner,
        &String::from_str(&env, "P1"),
        &CoverageType::Health,
        &100,
        &1000,
        &None,
    );
    client.create_policy(
        &owner,
        &String::from_str(&env, "P2"),
        &CoverageType::Life,
        &200,
        &2000,
        &None,
    );
    client.create_policy(
        &owner,
        &String::from_str(&env, "P3"),
        &CoverageType::Auto,
        &300,
        &3000,
        &None,
    );

    let total = client.get_total_monthly_premium(&owner);
    assert_eq!(total, 600); // 100 + 200 + 300
}

#[test]
fn test_get_total_monthly_premium_zero_for_no_policies() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let total = client.get_total_monthly_premium(&owner);
    assert_eq!(total, 0);
}

#[test]
fn test_get_total_monthly_premium_excludes_deactivated() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let id1 = client.create_policy(
        &owner,
        &String::from_str(&env, "P1"),
        &CoverageType::Health,
        &100,
        &1000,
        &None,
    );
    client.create_policy(
        &owner,
        &String::from_str(&env, "P2"),
        &CoverageType::Life,
        &200,
        &2000,
        &None,
    );

    assert_eq!(client.get_total_monthly_premium(&owner), 300);

    client.deactivate_policy(&owner, &id1);

    assert_eq!(client.get_total_monthly_premium(&owner), 200);
}

#[test]
fn test_get_total_monthly_premium_owner_isolation() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner_a = Address::generate(&env);
    let owner_b = Address::generate(&env);

    client.create_policy(
        &owner_a,
        &String::from_str(&env, "PA"),
        &CoverageType::Health,
        &100,
        &1000,
        &None,
    );
    client.create_policy(
        &owner_b,
        &String::from_str(&env, "PB"),
        &CoverageType::Life,
        &500,
        &5000,
        &None,
    );

    assert_eq!(client.get_total_monthly_premium(&owner_a), 100);
    assert_eq!(client.get_total_monthly_premium(&owner_b), 500);
}

// ── get_policy ───────────────────────────────────────────────────────────────

#[test]
fn test_get_policy_nonexistent_returns_none() {
    let env = setup_env();
    let (_, client) = new_client(&env);

    let policy = client.get_policy(&999);
    assert!(policy.is_none());
}

#[test]
fn test_get_policy_all_fields_correct() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    set_time(&env, 1_700_000_000);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Full Policy"),
        &CoverageType::Liability,
        &250,
        &25000,
        &None,
    );

    let policy = client.get_policy(&policy_id).unwrap();
    assert_eq!(policy.id, policy_id);
    assert_eq!(policy.owner, owner);
    assert_eq!(policy.monthly_premium, 250);
    assert_eq!(policy.coverage_amount, 25000);
    assert_eq!(policy.coverage_type, CoverageType::Liability);
    assert!(policy.active);
    assert_eq!(policy.next_payment_date, 1_700_000_000 + 30 * 86400);
}

// ── premium schedules ────────────────────────────────────────────────────────

#[test]
fn test_create_premium_schedule_succeeds() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    set_time(&env, 1000);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Health Insurance"),
        &CoverageType::Health,
        &500,
        &50000,
        &None,
    );

    let schedule_id = client.create_premium_schedule(&owner, &policy_id, &3000, &2592000);
    assert_eq!(schedule_id, 1);

    let schedule = client.get_premium_schedule(&schedule_id).unwrap();
    assert_eq!(schedule.next_due, 3000);
    assert_eq!(schedule.interval, 2592000);
    assert!(schedule.active);
}

#[test]
fn test_modify_premium_schedule() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    set_time(&env, 1000);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Health Insurance"),
        &CoverageType::Health,
        &500,
        &50000,
        &None,
    );

    let schedule_id = client.create_premium_schedule(&owner, &policy_id, &3000, &2592000);
    client.modify_premium_schedule(&owner, &schedule_id, &4000, &2678400);

    let schedule = client.get_premium_schedule(&schedule_id).unwrap();
    assert_eq!(schedule.next_due, 4000);
    assert_eq!(schedule.interval, 2678400);
}

#[test]
fn test_cancel_premium_schedule() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    set_time(&env, 1000);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Health Insurance"),
        &CoverageType::Health,
        &500,
        &50000,
        &None,
    );

    let schedule_id = client.create_premium_schedule(&owner, &policy_id, &3000, &2592000);
    client.cancel_premium_schedule(&owner, &schedule_id);

    let schedule = client.get_premium_schedule(&schedule_id).unwrap();
    assert!(!schedule.active);
}

#[test]
fn test_execute_due_premium_schedules_one_due() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    set_time(&env, 1000);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Health Insurance"),
        &CoverageType::Health,
        &500,
        &50000,
        &None,
    );

    let schedule_id = client.create_premium_schedule(&owner, &policy_id, &3000, &0);

    set_time(&env, 3500);
    let executed = client.execute_due_premium_schedules();

    assert_eq!(executed.len(), 1);
    assert_eq!(executed.get(0).unwrap(), schedule_id);

    let policy = client.get_policy(&policy_id).unwrap();
    assert_eq!(policy.next_payment_date, 3500 + 30 * 86400);
}

#[test]
fn test_execute_recurring_premium_schedule() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    set_time(&env, 1000);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Health Insurance"),
        &CoverageType::Health,
        &500,
        &50000,
        &None,
    );

    let schedule_id = client.create_premium_schedule(&owner, &policy_id, &3000, &2592000);

    set_time(&env, 3500);
    client.execute_due_premium_schedules();

    let schedule = client.get_premium_schedule(&schedule_id).unwrap();
    assert!(schedule.active);
    assert_eq!(schedule.next_due, 3000 + 2592000);
}

#[test]
fn test_get_premium_schedules_returns_all() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    set_time(&env, 1000);

    let policy_id1 = client.create_policy(
        &owner,
        &String::from_str(&env, "Health Insurance"),
        &CoverageType::Health,
        &500,
        &50000,
        &None,
    );
    let policy_id2 = client.create_policy(
        &owner,
        &String::from_str(&env, "Life Insurance"),
        &CoverageType::Life,
        &300,
        &100000,
        &None,
    );

    client.create_premium_schedule(&owner, &policy_id1, &3000, &2592000);
    client.create_premium_schedule(&owner, &policy_id2, &4000, &2592000);

    let schedules = client.get_premium_schedules(&owner);
    assert_eq!(schedules.len(), 2);
}

// ── events ──────────────────────────────────────────────────────────────────

#[test]
fn test_create_policy_emits_event() {
    use soroban_sdk::testutils::Events;
    use soroban_sdk::{symbol_short, vec, IntoVal};

    let env = setup_env();
    let (contract_id, client) = new_client(&env);
    let owner = Address::generate(&env);

    let name = String::from_str(&env, "Health Policy");
    let policy_id = client.create_policy(
        &owner,
        &name,
        &CoverageType::Health,
        &100,
        &10000,
        &None,
    );

    let events = env.events().all();
    assert!(events.len() >= 2);

    let audit_event = events.last().unwrap();

    let expected_topics = vec![
        &env,
        symbol_short!("insure").into_val(&env),
        InsuranceEvent::PolicyCreated.into_val(&env),
    ];

    assert_eq!(audit_event.1, expected_topics);

    let data: (u32, Address) = soroban_sdk::FromVal::from_val(&env, &audit_event.2);
    assert_eq!(data.0, policy_id);
    assert_eq!(audit_event.0, contract_id);
}

#[test]
fn test_pay_premium_emits_event() {
    use soroban_sdk::testutils::Events;
    use soroban_sdk::{symbol_short, vec, IntoVal};

    let env = setup_env();
    let (contract_id, client) = new_client(&env);
    let owner = Address::generate(&env);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Health Policy"),
        &CoverageType::Health,
        &100,
        &10000,
        &None,
    );

    client.pay_premium(&owner, &policy_id);

    let events = env.events().all();
    assert!(events.len() >= 2);

    let audit_event = events.last().unwrap();

    let expected_topics = vec![
        &env,
        symbol_short!("insure").into_val(&env),
        InsuranceEvent::PremiumPaid.into_val(&env),
    ];

    assert_eq!(audit_event.1, expected_topics);
    assert_eq!(audit_event.0, contract_id);
}

// ── error codes ─────────────────────────────────────────────────────────────

#[test]
fn test_error_code_policy_not_found_is_1() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let result = client.try_pay_premium(&owner, &999);
    assert_eq!(result, Err(Ok(InsuranceError::PolicyNotFound)));
    // Discriminant must be 1
    assert_eq!(InsuranceError::PolicyNotFound as u32, 1);
}

#[test]
fn test_error_code_unauthorized_is_2() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);

    let id = client.create_policy(
        &owner,
        &String::from_str(&env, "P"),
        &CoverageType::Health,
        &100,
        &1000,
        &None,
    );

    let result = client.try_deactivate_policy(&other, &id);
    assert_eq!(result, Err(Ok(InsuranceError::Unauthorized)));
    assert_eq!(InsuranceError::Unauthorized as u32, 2);
}

#[test]
fn test_error_code_invalid_amount_is_3() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let result = client.try_create_policy(
        &owner,
        &String::from_str(&env, "P"),
        &CoverageType::Health,
        &0,
        &1000,
        &None,
    );
    assert!(result.is_err());
    assert_eq!(InsuranceError::InvalidAmount as u32, 3);
}

#[test]
fn test_error_code_policy_inactive_is_4() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let id = client.create_policy(
        &owner,
        &String::from_str(&env, "P"),
        &CoverageType::Health,
        &100,
        &1000,
        &None,
    );
    client.deactivate_policy(&owner, &id);

    let result = client.try_pay_premium(&owner, &id);
    assert_eq!(result, Err(Ok(InsuranceError::PolicyInactive)));
    assert_eq!(InsuranceError::PolicyInactive as u32, 4);
}

#[test]
fn test_error_code_batch_too_large_is_8() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let mut ids = soroban_sdk::Vec::new(&env);
    for _ in 0..51u32 {
        let id = client.create_policy(
            &owner,
            &String::from_str(&env, "P"),
            &CoverageType::Health,
            &100,
            &1000,
            &None,
        );
        ids.push_back(id);
    }

    let result = client.try_batch_pay_premiums(&owner, &ids);
    assert_eq!(result, Err(Ok(InsuranceError::BatchTooLarge)));
    assert_eq!(InsuranceError::BatchTooLarge as u32, 8);
}

// ── time-drift resilience ────────────────────────────────────────────────────

#[test]
fn test_time_drift_schedule_not_executed_before_next_due() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    let next_due = 5000u64;
    set_time(&env, 1000);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Life Cover"),
        &CoverageType::Life,
        &200,
        &100000,
        &None,
    );
    client.create_premium_schedule(&owner, &policy_id, &next_due, &2592000);

    set_time(&env, next_due - 1);
    let executed = client.execute_due_premium_schedules();
    assert_eq!(
        executed.len(),
        0,
        "Must not execute one second before next_due"
    );
}

#[test]
fn test_time_drift_schedule_executes_at_exact_next_due() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    let next_due = 5000u64;
    set_time(&env, 1000);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Health Plan"),
        &CoverageType::Health,
        &150,
        &75000,
        &None,
    );
    let schedule_id = client.create_premium_schedule(&owner, &policy_id, &next_due, &2592000);

    set_time(&env, next_due);
    let executed = client.execute_due_premium_schedules();
    assert_eq!(executed.len(), 1);
    assert_eq!(executed.get(0).unwrap(), schedule_id);

    let policy = client.get_policy(&policy_id).unwrap();
    assert_eq!(policy.next_payment_date, next_due + 30 * 86400);
}

#[test]
fn test_time_drift_next_payment_uses_actual_payment_time() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    let next_due = 5000u64;
    let late_time = next_due + 7 * 86400;
    set_time(&env, 1000);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Property Plan"),
        &CoverageType::Property,
        &300,
        &200000,
        &None,
    );
    client.create_premium_schedule(&owner, &policy_id, &next_due, &2592000);

    set_time(&env, late_time);
    client.execute_due_premium_schedules();

    let policy = client.get_policy(&policy_id).unwrap();
    assert_eq!(policy.next_payment_date, late_time + 30 * 86400);
    assert!(policy.next_payment_date > next_due + 30 * 86400);
}

#[test]
fn test_time_drift_no_double_execution_after_advance() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);
    let next_due = 5000u64;
    let interval = 2_592_000u64;
    set_time(&env, 1000);

    let policy_id = client.create_policy(
        &owner,
        &String::from_str(&env, "Auto Cover"),
        &CoverageType::Auto,
        &100,
        &50000,
        &None,
    );
    client.create_premium_schedule(&owner, &policy_id, &next_due, &interval);

    set_time(&env, next_due);
    let executed = client.execute_due_premium_schedules();
    assert_eq!(executed.len(), 1);

    set_time(&env, next_due + 1000);
    let executed_again = client.execute_due_premium_schedules();
    assert_eq!(
        executed_again.len(),
        0,
        "Must not re-execute before new next_due"
    );
}

// ── multi-policy scenarios ───────────────────────────────────────────────────

#[test]
fn test_multiple_policies_full_lifecycle() {
    let env = setup_env();
    let (_, client) = new_client(&env);
    let owner = Address::generate(&env);

    let p1 = client.create_policy(
        &owner,
        &String::from_str(&env, "Health"),
        &CoverageType::Health,
        &100,
        &10000,
        &None,
    );
    let p2 = client.create_policy(
        &owner,
        &String::from_str(&env, "Life"),
        &CoverageType::Life,
        &200,
        &20000,
        &None,
    );
    let p3 = client.create_policy(
        &owner,
        &String::from_str(&env, "Auto"),
        &CoverageType::Auto,
        &300,
        &30000,
        &None,
    );

    assert!(client.get_policy(&p1).unwrap().active);
    assert!(client.get_policy(&p2).unwrap().active);
    assert!(client.get_policy(&p3).unwrap().active);

    set_time(&env, env.ledger().timestamp() + 86400);
    client.pay_premium(&owner, &p1);
    client.pay_premium(&owner, &p2);
    client.pay_premium(&owner, &p3);

    client.deactivate_policy(&owner, &p1);
    client.deactivate_policy(&owner, &p2);
    client.deactivate_policy(&owner, &p3);

    assert!(!client.get_policy(&p1).unwrap().active);
    assert!(!client.get_policy(&p2).unwrap().active);
    assert!(!client.get_policy(&p3).unwrap().active);

    let active = client.get_active_policies(&owner);
    assert_eq!(active.len(), 0);

    let total = client.get_total_monthly_premium(&owner);
    assert_eq!(total, 0);
}
