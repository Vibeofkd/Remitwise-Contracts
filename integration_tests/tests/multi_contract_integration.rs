#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String as SorobanString};

// Import all contract types and clients
use bill_payments::{BillPayments, BillPaymentsClient};
use insurance::{Insurance, InsuranceClient};
use orchestrator::{Orchestrator, OrchestratorClient};
use remittance_split::{RemittanceSplit, RemittanceSplitClient};
use savings_goals::{SavingsGoalContract, SavingsGoalContractClient};
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct IntegrationMockFamilyWallet;

#[contractimpl]
impl IntegrationMockFamilyWallet {
    /// @notice Always allows spending in integration tests.
    pub fn check_spending_limit(_env: Env, _caller: Address, _amount: i128) -> bool {
        true
    }
}

#[contract]
pub struct IntegrationMockRemittanceSplit;

#[contractimpl]
impl IntegrationMockRemittanceSplit {
    /// @notice Deterministic 40/30/20/10 split used by orchestrator tests.
    pub fn calculate_split(env: Env, total_amount: i128) -> soroban_sdk::Vec<i128> {
        let spending = (total_amount * 40) / 100;
        let savings = (total_amount * 30) / 100;
        let bills = (total_amount * 20) / 100;
        let insurance = (total_amount * 10) / 100;
        soroban_sdk::Vec::from_array(&env, [spending, savings, bills, insurance])
    }
}

/// Integration test that simulates a complete user flow:
/// 1. Deploy all contracts (remittance_split, savings_goals, bill_payments, insurance)
/// 2. Initialize split configuration
/// 3. Create goals, bills, and policies
/// 4. Calculate split and verify amounts align with expectations
#[test]
fn test_multi_contract_user_flow() {
    // Setup test environment
    let env = Env::default();
    env.mock_all_auths();

    // Generate test user address
    let user = Address::generate(&env);

    // Deploy all contracts
    let remittance_contract_id = env.register_contract(None, RemittanceSplit);
    let remittance_client = RemittanceSplitClient::new(&env, &remittance_contract_id);

    let savings_contract_id = env.register_contract(None, SavingsGoalContract);
    let savings_client = SavingsGoalContractClient::new(&env, &savings_contract_id);

    let bills_contract_id = env.register_contract(None, BillPayments);
    let bills_client = BillPaymentsClient::new(&env, &bills_contract_id);

    let insurance_contract_id = env.register_contract(None, Insurance);
    let insurance_client = InsuranceClient::new(&env, &insurance_contract_id);

    // Step 1: Initialize remittance split with percentages
    // Spending: 40%, Savings: 30%, Bills: 20%, Insurance: 10%
    let nonce = 0u64;
    remittance_client.initialize_split(
        &user, &nonce, &40u32, // spending
        &30u32, // savings
        &20u32, // bills
        &10u32, // insurance
    );

    // Step 2: Create a savings goal
    let goal_name = SorobanString::from_str(&env, "Education Fund");
    let target_amount = 10_000i128;
    let target_date = env.ledger().timestamp() + (365 * 86400); // 1 year from now

    let goal_id = savings_client.create_goal(&user, &goal_name, &target_amount, &target_date);
    assert_eq!(goal_id, 1u32, "Goal ID should be 1");

    // Step 3: Create a bill
    let bill_name = SorobanString::from_str(&env, "Electricity Bill");
    let bill_amount = 500i128;
    let due_date = env.ledger().timestamp() + (30 * 86400); // 30 days from now
    let recurring = true;
    let frequency_days = 30u32;

    let bill_id = bills_client.create_bill(
        &user,
        &bill_name,
        &bill_amount,
        &due_date,
        &recurring,
        &frequency_days,
        &SorobanString::from_str(&env, "XLM"),
    );
    assert_eq!(bill_id, 1u32, "Bill ID should be 1");

    // Step 4: Create an insurance policy
    let policy_name = SorobanString::from_str(&env, "Health Insurance");
    let coverage_type = SorobanString::from_str(&env, "health");
    let monthly_premium = 200i128;
    let coverage_amount = 50_000i128;

    let policy_id = insurance_client.create_policy(
        &user,
        &policy_name,
        &coverage_type,
        &monthly_premium,
        &coverage_amount,
    );
    assert_eq!(policy_id, 1u32, "Policy ID should be 1");

    // Step 5: Calculate split for a remittance amount
    let total_remittance = 10_000i128;
    let amounts = remittance_client.calculate_split(&total_remittance);
    assert_eq!(amounts.len(), 4, "Should have 4 allocation amounts");

    // Extract amounts
    let spending_amount = amounts.get(0).unwrap();
    let savings_amount = amounts.get(1).unwrap();
    let bills_amount = amounts.get(2).unwrap();
    let insurance_amount = amounts.get(3).unwrap();

    // Step 6: Verify amounts match expected percentages
    // Spending: 40% of 10,000 = 4,000
    assert_eq!(
        spending_amount, 4_000i128,
        "Spending amount should be 4,000"
    );

    // Savings: 30% of 10,000 = 3,000
    assert_eq!(savings_amount, 3_000i128, "Savings amount should be 3,000");

    // Bills: 20% of 10,000 = 2,000
    assert_eq!(bills_amount, 2_000i128, "Bills amount should be 2,000");

    // Insurance: 10% of 10,000 = 1,000 (gets remainder to handle rounding)
    assert_eq!(
        insurance_amount, 1_000i128,
        "Insurance amount should be 1,000"
    );

    // Step 7: Verify total sum equals original amount
    let total_allocated = spending_amount + savings_amount + bills_amount + insurance_amount;
    assert_eq!(
        total_allocated, total_remittance,
        "Total allocated should equal total remittance"
    );

    println!("✅ Multi-contract integration test passed!");
    println!("   Total Remittance: {}", total_remittance);
    println!("   Spending: {} (40%)", spending_amount);
    println!("   Savings: {} (30%)", savings_amount);
    println!("   Bills: {} (20%)", bills_amount);
    println!("   Insurance: {} (10%)", insurance_amount);
}

/// Test with different split percentages and verify rounding behavior
#[test]
fn test_split_with_rounding() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);

    // Deploy remittance split contract
    let remittance_contract_id = env.register_contract(None, RemittanceSplit);
    let remittance_client = RemittanceSplitClient::new(&env, &remittance_contract_id);

    // Initialize with percentages that might cause rounding issues
    // Spending: 33%, Savings: 33%, Bills: 17%, Insurance: 17%
    remittance_client.initialize_split(&user, &0u64, &33u32, &33u32, &17u32, &17u32);

    // Calculate split for an amount that will have rounding
    let total = 1_000i128;
    let amounts = remittance_client.calculate_split(&total);

    let spending = amounts.get(0).unwrap();
    let savings = amounts.get(1).unwrap();
    let bills = amounts.get(2).unwrap();
    let insurance = amounts.get(3).unwrap();

    // Verify total still equals original (insurance gets remainder)
    let total_allocated = spending + savings + bills + insurance;
    assert_eq!(
        total_allocated, total,
        "Total allocated must equal original amount despite rounding"
    );

    println!("✅ Rounding test passed!");
    println!("   Total: {}", total);
    println!("   Spending: {} (33%)", spending);
    println!("   Savings: {} (33%)", savings);
    println!("   Bills: {} (17%)", bills);
    println!("   Insurance: {} (17% + remainder)", insurance);
}

/// Test creating multiple goals, bills, and policies
#[test]
fn test_multiple_entities_creation() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);

    // Deploy contracts
    let savings_contract_id = env.register_contract(None, SavingsGoalContract);
    let savings_client = SavingsGoalContractClient::new(&env, &savings_contract_id);

    let bills_contract_id = env.register_contract(None, BillPayments);
    let bills_client = BillPaymentsClient::new(&env, &bills_contract_id);

    let insurance_contract_id = env.register_contract(None, Insurance);
    let insurance_client = InsuranceClient::new(&env, &insurance_contract_id);

    // Create multiple savings goals
    let goal1 = savings_client.create_goal(
        &user,
        &SorobanString::from_str(&env, "Emergency Fund"),
        &5_000i128,
        &(env.ledger().timestamp() + 180 * 86400),
    );
    assert_eq!(goal1, 1u32);

    let goal2 = savings_client.create_goal(
        &user,
        &SorobanString::from_str(&env, "Vacation"),
        &2_000i128,
        &(env.ledger().timestamp() + 90 * 86400),
    );
    assert_eq!(goal2, 2u32);

    // Create multiple bills
    let bill1 = bills_client.create_bill(
        &user,
        &SorobanString::from_str(&env, "Rent"),
        &1_500i128,
        &(env.ledger().timestamp() + 30 * 86400),
        &true,
        &30u32,
        &SorobanString::from_str(&env, "XLM"),
    );
    assert_eq!(bill1, 1u32);

    let bill2 = bills_client.create_bill(
        &user,
        &SorobanString::from_str(&env, "Internet"),
        &100i128,
        &(env.ledger().timestamp() + 15 * 86400),
        &true,
        &30u32,
        &SorobanString::from_str(&env, "XLM"),
    );
    assert_eq!(bill2, 2u32);

    // Create multiple insurance policies
    let policy1 = insurance_client.create_policy(
        &user,
        &SorobanString::from_str(&env, "Life Insurance"),
        &SorobanString::from_str(&env, "life"),
        &150i128,
        &100_000i128,
    );
    assert_eq!(policy1, 1u32);

    let policy2 = insurance_client.create_policy(
        &user,
        &SorobanString::from_str(&env, "Emergency Coverage"),
        &SorobanString::from_str(&env, "emergency"),
        &50i128,
        &10_000i128,
    );
    assert_eq!(policy2, 2u32);

    println!("✅ Multiple entities creation test passed!");
    println!("   Created 2 savings goals");
    println!("   Created 2 bills");
    println!("   Created 2 insurance policies");
}

/// Workspace-wide event topic compliance tests.
///
/// These tests verify that events emitted by key contracts follow the
/// deterministic Remitwise topic schema:
/// `("Remitwise", category: u32, priority: u32, action: Symbol)`.
///
/// The test triggers representative actions in each contract and inspects
/// `env.events().all()` to validate topics and payload shapes. Any deviation
/// will cause the test to fail, highlighting contracts that must be updated
/// to the shared `RemitwiseEvents` helper.
#[test]
fn test_event_topic_compliance_across_contracts() {
    use soroban_sdk::{symbol_short, Vec, IntoVal};

    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);

    // Deploy representative contracts
    let remittance_id = env.register_contract(None, RemittanceSplit);
    let remittance_client = RemittanceSplitClient::new(&env, &remittance_id);

    let savings_id = env.register_contract(None, SavingsGoalContract);
    let savings_client = SavingsGoalContractClient::new(&env, &savings_id);

    let bills_id = env.register_contract(None, BillPayments);
    let bills_client = BillPaymentsClient::new(&env, &bills_id);

    let insurance_id = env.register_contract(None, Insurance);
    let insurance_client = InsuranceClient::new(&env, &insurance_id);

    // Trigger events in each contract
    remittance_client.initialize_split(&user, &0u64, &40u32, &30u32, &20u32, &10u32);

    let goal_name = SorobanString::from_str(&env, "Compliance Goal");
    let _ = savings_client.create_goal(&user, &goal_name, &1000i128, &(env.ledger().timestamp() + 86400));

    let bill_name = SorobanString::from_str(&env, "Compliance Bill");
    let _ = bills_client.create_bill(
        &user,
        &bill_name,
        &100i128,
        &(env.ledger().timestamp() + 86400),
        &true,
        &30u32,
        &SorobanString::from_str(&env, "XLM"),
    );

    let policy_name = SorobanString::from_str(&env, "Compliance Policy");
    let coverage_type = SorobanString::from_str(&env, "health");
    let _ = insurance_client.create_policy(&user, &policy_name, &coverage_type, &50i128, &1000i128);

    // Collect published events
    let events = env.events().all();
    assert!(events.len() > 0, "No events were emitted by the sample actions");

    // Validate each event's topics conform to Remitwise schema
    let mut non_compliant = Vec::new(&env);

    for ev in events.iter() {
        let topics = &ev.1;
        // Expect topics to be a vector of length 4 starting with symbol_short!("Remitwise")
        let ok = topics.len() == 4 && topics.get(0).unwrap() == symbol_short!("Remitwise").into_val(&env);
        if !ok {
            non_compliant.push_back(ev.clone());
        }
    }

    // Fail if any non-compliant events found, listing one example for debugging
    assert_eq!(non_compliant.len(), 0u32, "Found events that do not follow the Remitwise topic schema. See EVENTS.md and remitwise-common::RemitwiseEvents for guidance.");
}

/// @notice Verifies inactive insurance policy fails orchestrated flow safely.
/// @dev Checks that downstream writes in savings and bills are reverted.
#[test]
fn test_orchestrator_flow_inactive_policy_reverts_downstream_state() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);

    let orchestrator_id = env.register_contract(None, Orchestrator);
    let wallet_id = env.register_contract(None, IntegrationMockFamilyWallet);
    let split_id = env.register_contract(None, IntegrationMockRemittanceSplit);
    let savings_id = env.register_contract(None, SavingsGoalContract);
    let bills_id = env.register_contract(None, BillPayments);
    let insurance_id = env.register_contract(None, Insurance);

    let orchestrator_client = OrchestratorClient::new(&env, &orchestrator_id);
    let savings_client = SavingsGoalContractClient::new(&env, &savings_id);
    let bills_client = BillPaymentsClient::new(&env, &bills_id);
    let insurance_client = InsuranceClient::new(&env, &insurance_id);

    let goal_id = savings_client.create_goal(
        &user,
        &SorobanString::from_str(&env, "Safety Goal"),
        &10_000i128,
        &(env.ledger().timestamp() + 365 * 86400),
    );
    let bill_id = bills_client.create_bill(
        &user,
        &SorobanString::from_str(&env, "Safety Bill"),
        &500i128,
        &(env.ledger().timestamp() + 30 * 86400),
        &true,
        &30u32,
        &SorobanString::from_str(&env, "XLM"),
    );
    let policy_id = insurance_client.create_policy(
        &user,
        &SorobanString::from_str(&env, "Safety Policy"),
        &SorobanString::from_str(&env, "health"),
        &200i128,
        &25_000i128,
    );
    insurance_client.deactivate_policy(&user, &policy_id);

    let result = orchestrator_client.try_execute_remittance_flow(
        &user,
        &10_000i128,
        &wallet_id,
        &split_id,
        &savings_id,
        &bills_id,
        &insurance_id,
        &goal_id,
        &bill_id,
        &policy_id,
    );
    assert!(result.is_err());

    let goal_after = savings_client.get_goal(&goal_id).unwrap();
    assert_eq!(goal_after.current_amount, 0, "Savings mutation must rollback");

    let bill_after = bills_client.get_bill(&bill_id).unwrap();
    assert!(!bill_after.paid, "Bill payment mutation must rollback");
}

/// @notice Verifies missing insurance policy fails orchestrated flow safely.
/// @dev Uses unknown `policy_id` and asserts no persisted mutations.
#[test]
fn test_orchestrator_flow_missing_policy_reverts_downstream_state() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);

    let orchestrator_id = env.register_contract(None, Orchestrator);
    let wallet_id = env.register_contract(None, IntegrationMockFamilyWallet);
    let split_id = env.register_contract(None, IntegrationMockRemittanceSplit);
    let savings_id = env.register_contract(None, SavingsGoalContract);
    let bills_id = env.register_contract(None, BillPayments);
    let insurance_id = env.register_contract(None, Insurance);

    let orchestrator_client = OrchestratorClient::new(&env, &orchestrator_id);
    let savings_client = SavingsGoalContractClient::new(&env, &savings_id);
    let bills_client = BillPaymentsClient::new(&env, &bills_id);

    let goal_id = savings_client.create_goal(
        &user,
        &SorobanString::from_str(&env, "Missing Policy Goal"),
        &10_000i128,
        &(env.ledger().timestamp() + 365 * 86400),
    );
    let bill_id = bills_client.create_bill(
        &user,
        &SorobanString::from_str(&env, "Missing Policy Bill"),
        &500i128,
        &(env.ledger().timestamp() + 30 * 86400),
        &true,
        &30u32,
        &SorobanString::from_str(&env, "XLM"),
    );

    let result = orchestrator_client.try_execute_remittance_flow(
        &user,
        &10_000i128,
        &wallet_id,
        &split_id,
        &savings_id,
        &bills_id,
        &insurance_id,
        &goal_id,
        &bill_id,
        &999_999u32, // missing policy ID
    );
    assert!(result.is_err());

    let goal_after = savings_client.get_goal(&goal_id).unwrap();
    assert_eq!(goal_after.current_amount, 0, "Savings mutation must rollback");

    let bill_after = bills_client.get_bill(&bill_id).unwrap();
    assert!(!bill_after.paid, "Bill payment mutation must rollback");
}
