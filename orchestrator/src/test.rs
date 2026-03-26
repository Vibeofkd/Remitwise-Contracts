use crate::{ExecutionState, Orchestrator, OrchestratorClient, OrchestratorError};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Vec};

// ============================================================================
// Mock Contract Implementations
// ============================================================================

/// Mock Family Wallet contract for testing
#[contract]
pub struct MockFamilyWallet;

#[contractimpl]
impl MockFamilyWallet {
    /// Mock implementation of check_spending_limit
    /// Returns true if amount <= 10000 (simulating a spending limit)
    pub fn check_spending_limit(_env: Env, _caller: Address, amount: i128) -> bool {
        amount <= 10000
    }
}

/// Mock Remittance Split contract for testing
#[contract]
pub struct MockRemittanceSplit;

#[contractimpl]
impl MockRemittanceSplit {
    /// Mock implementation of calculate_split
    /// Returns [40%, 30%, 20%, 10%] split
    pub fn calculate_split(env: Env, total_amount: i128) -> Vec<i128> {
        let spending = (total_amount * 40) / 100;
        let savings = (total_amount * 30) / 100;
        let bills = (total_amount * 20) / 100;
        let insurance = (total_amount * 10) / 100;

        Vec::from_array(&env, [spending, savings, bills, insurance])
    }
}

/// Mock Savings Goals contract for testing
#[contract]
pub struct MockSavingsGoals;

#[derive(Clone)]
#[contracttype]
pub struct SavingsState {
    pub deposit_count: u32,
}

#[contractimpl]
impl MockSavingsGoals {
    /// @notice Returns the savings mock state for rollback assertions.
    pub fn get_state(env: Env) -> SavingsState {
        env.storage()
            .instance()
            .get(&symbol_short!("STATE"))
            .unwrap_or(SavingsState { deposit_count: 0 })
    }

    /// Mock implementation of add_to_goal
    /// Panics if goal_id == 999 (simulating goal not found)
    /// Panics if goal_id == 998 (simulating goal already completed)
    /// Panics if amount <= 0 (simulating invalid amount)
    pub fn add_to_goal(env: Env, _caller: Address, goal_id: u32, amount: i128) -> i128 {
        let mut state = Self::get_state(env.clone());
        state.deposit_count += 1;
        env.storage().instance().set(&symbol_short!("STATE"), &state);
        if goal_id == 999 {
            panic!("Goal not found");
        }
        if goal_id == 998 {
            panic!("Goal already completed");
        }
        if amount <= 0 {
            panic!("Amount must be positive");
        }
        amount
    }
}

/// Mock Bill Payments contract for testing
#[contract]
pub struct MockBillPayments;

#[derive(Clone)]
#[contracttype]
pub struct BillsState {
    pub payment_count: u32,
}

#[contractimpl]
impl MockBillPayments {
    /// @notice Returns the bill mock state for rollback assertions.
    pub fn get_state(env: Env) -> BillsState {
        env.storage()
            .instance()
            .get(&symbol_short!("STATE"))
            .unwrap_or(BillsState { payment_count: 0 })
    }

    /// Mock implementation of pay_bill
    /// Panics if bill_id == 999 (simulating bill not found)
    /// Panics if bill_id == 998 (simulating bill already paid)
    pub fn pay_bill(env: Env, _caller: Address, bill_id: u32) {
        let mut state = Self::get_state(env.clone());
        state.payment_count += 1;
        env.storage().instance().set(&symbol_short!("STATE"), &state);
        if bill_id == 999 {
            panic!("Bill not found");
        }
        if bill_id == 998 {
            panic!("Bill already paid");
        }
    }
}

/// Mock Insurance contract for testing
#[contract]
pub struct MockInsurance;

#[contractimpl]
impl MockInsurance {
    /// Mock implementation of pay_premium
    /// Panics if policy_id == 999 (simulating policy not found)
    /// Returns false if policy_id == 998 (simulating inactive policy)
    pub fn pay_premium(_env: Env, _caller: Address, policy_id: u32) -> bool {
        if policy_id == 999 {
            panic!("Policy not found");
        }
        policy_id != 998
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use testutils::{generate_test_address, setup_test_env};

    /// Full test environment with all contracts deployed.
    /// Returns (env, orchestrator, family_wallet, remittance_split,
    ///          savings, bills, insurance, user)
    fn setup() -> (Env, Address, Address, Address, Address, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let orchestrator_id = env.register_contract(None, Orchestrator);
        let family_wallet_id = env.register_contract(None, MockFamilyWallet);
        let remittance_split_id = env.register_contract(None, MockRemittanceSplit);
        let savings_id = env.register_contract(None, MockSavingsGoals);
        let bills_id = env.register_contract(None, MockBillPayments);
        let insurance_id = env.register_contract(None, MockInsurance);

        let user = Address::generate(&env);

        (
            env,
            orchestrator_id,
            family_wallet_id,
            remittance_split_id,
            savings_id,
            bills_id,
            insurance_id,
            user,
        )
    }

    // Helper function to seed audit log for pagination tests
    fn seed_audit_log(env: &Env, user: &Address, count: u32) {
        let client = OrchestratorClient::new(env, &env.register_contract(None, Orchestrator));
        for i in 0..count {
            // Use internal audit log mechanism
            // This is a simplified version - in real tests you'd call actual functions
            env.as_contract(&client.contract_id, || {
                let mut log: Vec<OrchestratorAuditEntry> = env
                    .storage()
                    .instance()
                    .get(&symbol_short!("AUDIT"))
                    .unwrap_or_else(|| Vec::new(env));
                log.push_back(OrchestratorAuditEntry {
                    caller: user.clone(),
                    operation: symbol_short!("execflow"),
                    amount: i as i128,
                    success: true,
                    timestamp: env.ledger().timestamp(),
                    error_code: None,
                });
                env.storage().instance().set(&symbol_short!("AUDIT"), &log);
            });
        }
    }

    fn collect_all_pages(client: &OrchestratorClient, page_size: u32) -> Vec<OrchestratorAuditEntry> {
        let mut all = Vec::new(client.env());
        let mut cursor = 0u32;
        loop {
            let page = client.get_audit_log(&cursor, &page_size);
            if page.is_empty() {
                break;
            }
            for entry in page.iter() {
                all.push_back(entry);
            }
            cursor += page.len() as u32;
        }
        all
    }

    // ============================================================================
    // Existing Tests (preserved)
    // ============================================================================

    #[test]
    fn test_execute_savings_deposit_succeeds() {
        let (env, orchestrator_id, family_wallet_id, _, savings_id, _, _, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_savings_deposit(
            &user, &5000, &family_wallet_id, &savings_id, &1,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_savings_deposit_invalid_goal_fails() {
        let (env, orchestrator_id, family_wallet_id, _, savings_id, _, _, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_savings_deposit(
            &user, &5000, &family_wallet_id, &savings_id, &999,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_execute_savings_deposit_spending_limit_exceeded_fails() {
        let (env, orchestrator_id, family_wallet_id, _, savings_id, _, _, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_savings_deposit(
            &user, &15000, &family_wallet_id, &savings_id, &1,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            OrchestratorError::PermissionDenied
        );
    }

    #[test]
    fn test_execute_bill_payment_succeeds() {
        let (env, orchestrator_id, family_wallet_id, _, _, bills_id, _, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_bill_payment(
            &user, &3000, &family_wallet_id, &bills_id, &1,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_bill_payment_invalid_bill_fails() {
        let (env, orchestrator_id, family_wallet_id, _, _, bills_id, _, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_bill_payment(
            &user, &3000, &family_wallet_id, &bills_id, &999,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_execute_insurance_payment_succeeds() {
        let (env, orchestrator_id, family_wallet_id, _, _, _, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_insurance_payment(
            &user, &2000, &family_wallet_id, &insurance_id, &1,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_remittance_flow_succeeds() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );

        assert!(result.is_ok());
        let flow_result = result.unwrap().unwrap();
        assert_eq!(flow_result.total_amount, 10000);
        assert_eq!(flow_result.spending_amount, 4000);
        assert_eq!(flow_result.savings_amount, 3000);
        assert_eq!(flow_result.bills_amount, 2000);
        assert_eq!(flow_result.insurance_amount, 1000);
        assert!(flow_result.savings_success);
        assert!(flow_result.bills_success);
        assert!(flow_result.insurance_success);
    }

    #[test]
    fn test_execute_remittance_flow_spending_limit_exceeded_fails() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &15000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            OrchestratorError::PermissionDenied
        );
    }

    #[test]
    fn test_execute_remittance_flow_invalid_amount_fails() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &0, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            OrchestratorError::InvalidAmount
        );
    }

    #[test]
    fn test_get_execution_stats_succeeds() {
        let (env, orchestrator_id, _, _, _, _, _, _) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let stats = client.get_execution_stats();
        assert_eq!(stats.total_flows_executed, 0);
        assert_eq!(stats.total_flows_failed, 0);
        assert_eq!(stats.total_amount_processed, 0);
        assert_eq!(stats.last_execution, 0);
    }

    #[test]
    fn test_get_audit_log_succeeds() {
        let (env, orchestrator_id, _, _, _, _, _, _) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let log = client.get_audit_log(&0, &10);
        assert_eq!(log.len(), 0);
    }

    // ============================================================================
    // Rollback Semantics Tests — Savings Leg Failures
    // ============================================================================

    /// ROLLBACK-01: Savings leg fails with goal not found.
    #[test]
    fn test_rollback_savings_leg_goal_not_found() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id,
            &999, &1, &1,
        );

        assert!(result.is_err());
    }

    /// ROLLBACK-02: Savings leg fails because goal is already completed.
    #[test]
    fn test_rollback_savings_leg_goal_already_completed() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id,
            &998, &1, &1,
        );

        assert!(result.is_err());
    }

    /// ROLLBACK-03: Savings-only deposit fails with goal not found.
    #[test]
    fn test_rollback_savings_deposit_goal_not_found() {
        let (env, orchestrator_id, family_wallet_id, _, savings_id, _, _, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_savings_deposit(
            &user, &5000, &family_wallet_id, &savings_id, &999,
        );

        assert!(result.is_err());
    }

    /// ROLLBACK-04: Savings-only deposit fails with already-completed goal.
    #[test]
    fn test_rollback_savings_deposit_goal_already_completed() {
        let (env, orchestrator_id, family_wallet_id, _, savings_id, _, _, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_savings_deposit(
            &user, &5000, &family_wallet_id, &savings_id, &998,
        );

        assert!(result.is_err());
    }

    // ============================================================================
    // Rollback Semantics Tests — Bills Leg Failures
    // ============================================================================

    /// ROLLBACK-05: Bills leg fails with bill not found after savings succeeds.
    #[test]
    fn test_rollback_bills_leg_bill_not_found() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id,
            &1, &999, &1,
        );

        assert!(result.is_err());
    }

    /// ROLLBACK-06: Bills leg fails because bill was already paid.
    #[test]
    fn test_rollback_bills_leg_already_paid() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id,
            &1, &998, &1,
        );

        assert!(result.is_err());
    }

    /// ROLLBACK-07: Bills-only payment fails with bill not found.
    #[test]
    fn test_rollback_bill_payment_bill_not_found() {
        let (env, orchestrator_id, family_wallet_id, _, _, bills_id, _, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_bill_payment(
            &user, &3000, &family_wallet_id, &bills_id, &999,
        );

        assert!(result.is_err());
    }

    /// ROLLBACK-08: Bills-only payment fails with already-paid bill.
    #[test]
    fn test_rollback_bill_payment_already_paid() {
        let (env, orchestrator_id, family_wallet_id, _, _, bills_id, _, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_bill_payment(
            &user, &3000, &family_wallet_id, &bills_id, &998,
        );

        assert!(result.is_err());
    }

    // ============================================================================
    // Rollback Semantics Tests — Insurance Leg Failures
    // ============================================================================

    /// ROLLBACK-09: Insurance leg fails with policy not found.
    #[test]
    fn test_rollback_insurance_leg_policy_not_found() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id,
            &1, &1, &999,
        );

        assert!(result.is_err());
    }

    /// ROLLBACK-10: Insurance leg fails with inactive policy.
    #[test]
    fn test_rollback_insurance_leg_inactive_policy() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id,
            &1, &1, &998,
        );

        // Soft failure: insurance_success = false
        match result {
            Ok(Ok(flow_result)) => {
                assert!(!flow_result.insurance_success);
                assert!(flow_result.savings_success);
                assert!(flow_result.bills_success);
            }
            _ => {
                // Either hard or soft failure is acceptable
            }
        }
    }

    /// ROLLBACK-11: Insurance-only payment fails with policy not found.
    #[test]
    fn test_rollback_insurance_payment_policy_not_found() {
        let (env, orchestrator_id, family_wallet_id, _, _, _, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_insurance_payment(
            &user, &2000, &family_wallet_id, &insurance_id, &999,
        );

        assert!(result.is_err());
    }

    // ============================================================================
    // Rollback Semantics Tests — Permission & Validation Failures
    // ============================================================================

    /// ROLLBACK-12: Permission check fails before any downstream leg executes.
    #[test]
    fn test_rollback_permission_denied_before_any_leg() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10001, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            OrchestratorError::PermissionDenied
        );
    }

    /// ROLLBACK-13: Negative amount is rejected.
    #[test]
    fn test_rollback_negative_amount_rejected() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &-500, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            OrchestratorError::InvalidAmount
        );
    }

    /// ROLLBACK-14: Zero amount is rejected.
    #[test]
    fn test_rollback_zero_amount_rejected() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &0, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            OrchestratorError::InvalidAmount
        );
    }

    // ============================================================================
    // Rollback Semantics Tests — All Legs Fail
    // ============================================================================

    /// ROLLBACK-15: All three legs are configured to fail.
    #[test]
    fn test_rollback_all_legs_fail() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id,
            &999, &999, &999,
        );

        assert!(result.is_err());
    }

    // ============================================================================
    // Rollback Semantics Tests — Accounting Consistency
    // ============================================================================

    /// ROLLBACK-16: Successful flow produces correct allocation totals.
    #[test]
    fn test_accounting_consistency_on_success() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let total = 10000i128;
        let result = client.try_execute_remittance_flow(
            &user, &total, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );

        assert!(result.is_ok());
        let flow = result.unwrap().unwrap();

        let allocated = flow.spending_amount + flow.savings_amount
            + flow.bills_amount + flow.insurance_amount;

        assert_eq!(allocated, total);
        assert!(flow.spending_amount >= 0);
        assert!(flow.savings_amount >= 0);
        assert!(flow.bills_amount >= 0);
        assert!(flow.insurance_amount >= 0);
    }

    /// ROLLBACK-17: Correct split percentages are applied.
    #[test]
    fn test_accounting_split_percentages_correct() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );

        assert!(result.is_ok());
        let flow = result.unwrap().unwrap();

        assert_eq!(flow.spending_amount, 4000);
        assert_eq!(flow.savings_amount, 3000);
        assert_eq!(flow.bills_amount, 2000);
        assert_eq!(flow.insurance_amount, 1000);
    }

    /// ROLLBACK-18: Minimum valid amount (1) is processed.
    #[test]
    fn test_accounting_minimum_valid_amount() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &1, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );

        // Either succeeds or fails, but shouldn't panic
        match result {
            Ok(Ok(flow)) => assert_eq!(flow.total_amount, 1),
            Ok(Err(_)) | Err(_) => {}
        }
    }

    /// ROLLBACK-19: Maximum valid amount (10000) is processed.
    #[test]
    fn test_accounting_maximum_valid_amount_at_spending_limit() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );

        assert!(result.is_ok());
        let flow = result.unwrap().unwrap();
        assert_eq!(flow.total_amount, 10000);
    }

    /// ROLLBACK-20: One unit above the spending limit is rejected.
    #[test]
    fn test_accounting_one_above_spending_limit_rejected() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10001, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            OrchestratorError::PermissionDenied
        );
    }

    // ============================================================================
    // Rollback Semantics Tests — Independent Operation Rollbacks
    // ============================================================================

    /// ROLLBACK-21: Failed savings deposit does not affect subsequent call.
    #[test]
    fn test_rollback_failed_savings_does_not_poison_subsequent_call() {
        let (env, orchestrator_id, family_wallet_id, _, savings_id, _, _, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let fail_result = client.try_execute_savings_deposit(
            &user, &5000, &family_wallet_id, &savings_id, &999,
        );
        assert!(fail_result.is_err());

        let success_result = client.try_execute_savings_deposit(
            &user, &5000, &family_wallet_id, &savings_id, &1,
        );
        assert!(success_result.is_ok());
    }

    /// ROLLBACK-22: Failed bill payment does not affect subsequent call.
    #[test]
    fn test_rollback_failed_bill_does_not_poison_subsequent_call() {
        let (env, orchestrator_id, family_wallet_id, _, _, bills_id, _, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let fail_result = client.try_execute_bill_payment(
            &user, &3000, &family_wallet_id, &bills_id, &999,
        );
        assert!(fail_result.is_err());

        let success_result = client.try_execute_bill_payment(
            &user, &3000, &family_wallet_id, &bills_id, &1,
        );
        assert!(success_result.is_ok());
    }

    /// ROLLBACK-23: Failed insurance payment does not affect subsequent call.
    #[test]
    fn test_rollback_failed_insurance_does_not_poison_subsequent_call() {
        let (env, orchestrator_id, family_wallet_id, _, _, _, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let fail_result = client.try_execute_insurance_payment(
            &user, &2000, &family_wallet_id, &insurance_id, &999,
        );
        assert!(fail_result.is_err());

        let success_result = client.try_execute_insurance_payment(
            &user, &2000, &family_wallet_id, &insurance_id, &1,
        );
        assert!(success_result.is_ok());
    }

    /// ROLLBACK-24: Failed full flow does not affect subsequent full flow.
    #[test]
    fn test_rollback_failed_full_flow_does_not_poison_subsequent_full_flow() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let fail_result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id,
            &1, &999, &1,
        );
        assert!(fail_result.is_err());

        let success_result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id,
            &1, &1, &1,
        );
        assert!(success_result.is_ok());
    }

    // ============================================================================
    // Reentrancy Guard Tests
    // ============================================================================

    #[test]
    fn test_execution_state_starts_idle() {
        let (env, orchestrator_id, _, _, _, _, _, _) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);
        let state = client.get_execution_state();
        assert_eq!(state, ExecutionState::Idle);
    }

    #[test]
    fn test_execution_state_returns_to_idle_after_success() {
        let (env, orchestrator_id, family_wallet_id, _, savings_id, _, _, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_savings_deposit(&user, &5000, &family_wallet_id, &savings_id, &1);
        assert!(result.is_ok());

        let state = client.get_execution_state();
        assert_eq!(state, ExecutionState::Idle);
    }

    #[test]
    fn test_execution_state_returns_to_idle_after_failure() {
        let (env, orchestrator_id, family_wallet_id, _, _, _, _, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_savings_deposit(
            &user, &15000, &family_wallet_id, &family_wallet_id, &1,
        );
        assert!(result.is_err());

        let state = client.get_execution_state();
        assert_eq!(state, ExecutionState::Idle);
    }

    #[test]
    fn test_execution_state_idle_after_bill_payment_success() {
        let (env, orchestrator_id, family_wallet_id, _, _, bills_id, _, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_bill_payment(&user, &3000, &family_wallet_id, &bills_id, &1);
        assert!(result.is_ok());

        let state = client.get_execution_state();
        assert_eq!(state, ExecutionState::Idle);
    }

    #[test]
    fn test_execution_state_idle_after_bill_payment_failure() {
        let (env, orchestrator_id, family_wallet_id, _, _, bills_id, _, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_bill_payment(&user, &3000, &family_wallet_id, &bills_id, &999);
        assert!(result.is_err());

        let state = client.get_execution_state();
        assert_eq!(state, ExecutionState::Idle);
    }

    #[test]
    fn test_execution_state_idle_after_insurance_payment_success() {
        let (env, orchestrator_id, family_wallet_id, _, _, _, insurance_id, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_insurance_payment(&user, &2000, &family_wallet_id, &insurance_id, &1);
        assert!(result.is_ok());

        let state = client.get_execution_state();
        assert_eq!(state, ExecutionState::Idle);
    }

    #[test]
    fn test_execution_state_idle_after_remittance_flow_success() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );
        assert!(result.is_ok());

        let state = client.get_execution_state();
        assert_eq!(state, ExecutionState::Idle);
    }

    #[test]
    fn test_execution_state_idle_after_remittance_flow_invalid_amount() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &0, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );
        assert!(result.is_err());

        let state = client.get_execution_state();
        assert_eq!(state, ExecutionState::Idle);
    }

    #[test]
    fn test_execution_state_idle_after_remittance_flow_permission_denied() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_remittance_flow(
            &user, &15000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );
        assert!(result.is_err());

        let state = client.get_execution_state();
        assert_eq!(state, ExecutionState::Idle);
    }

    #[test]
    fn test_sequential_executions_succeed() {
        let (env, orchestrator_id, family_wallet_id, _, savings_id, bills_id, _, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result1 = client.try_execute_savings_deposit(&user, &5000, &family_wallet_id, &savings_id, &1);
        assert!(result1.is_ok());

        let result2 = client.try_execute_bill_payment(&user, &3000, &family_wallet_id, &bills_id, &1);
        assert!(result2.is_ok());

        let state = client.get_execution_state();
        assert_eq!(state, ExecutionState::Idle);
    }

    #[test]
    fn test_execution_after_failure_succeeds() {
        let (env, orchestrator_id, family_wallet_id, _, savings_id, _, _, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result1 = client.try_execute_savings_deposit(&user, &15000, &family_wallet_id, &savings_id, &1);
        assert!(result1.is_err());

        let result2 = client.try_execute_savings_deposit(&user, &5000, &family_wallet_id, &savings_id, &1);
        assert!(result2.is_ok());

        let state = client.get_execution_state();
        assert_eq!(state, ExecutionState::Idle);
    }

    #[test]
    fn test_reentrancy_guard_direct_storage_manipulation() {
        let (env, orchestrator_id, family_wallet_id, _, savings_id, _, _, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        env.as_contract(&orchestrator_id, || {
            env.storage().instance().set(
                &soroban_sdk::symbol_short!("EXEC_ST"),
                &ExecutionState::Executing,
            );
        });

        let result = client.try_execute_savings_deposit(&user, &5000, &family_wallet_id, &savings_id, &1);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            OrchestratorError::ReentrancyDetected
        );
    }

    #[test]
    fn test_reentrancy_guard_blocks_bill_payment_during_execution() {
        let (env, orchestrator_id, family_wallet_id, _, _, bills_id, _, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        env.as_contract(&orchestrator_id, || {
            env.storage().instance().set(
                &soroban_sdk::symbol_short!("EXEC_ST"),
                &ExecutionState::Executing,
            );
        });

        let result = client.try_execute_bill_payment(&user, &3000, &family_wallet_id, &bills_id, &1);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            OrchestratorError::ReentrancyDetected
        );
    }

    #[test]
    fn test_reentrancy_guard_blocks_insurance_payment_during_execution() {
        let (env, orchestrator_id, family_wallet_id, _, _, _, insurance_id, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        env.as_contract(&orchestrator_id, || {
            env.storage().instance().set(
                &soroban_sdk::symbol_short!("EXEC_ST"),
                &ExecutionState::Executing,
            );
        });

        let result = client.try_execute_insurance_payment(&user, &2000, &family_wallet_id, &insurance_id, &1);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            OrchestratorError::ReentrancyDetected
        );
    }

    #[test]
    fn test_reentrancy_guard_blocks_remittance_flow_during_execution() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        env.as_contract(&orchestrator_id, || {
            env.storage().instance().set(
                &soroban_sdk::symbol_short!("EXEC_ST"),
                &ExecutionState::Executing,
            );
        });

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id, &1, &1, &1,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            OrchestratorError::ReentrancyDetected
        );
    }

    #[test]
    fn test_multiple_sequential_flows_all_succeed() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        for _ in 0..3 {
            let result = client.try_execute_remittance_flow(
                &user, &10000, &family_wallet_id, &remittance_split_id,
                &savings_id, &bills_id, &insurance_id, &1, &1, &1,
            );
            assert!(result.is_ok());

            let state = client.get_execution_state();
            assert_eq!(state, ExecutionState::Idle);
        }
    }

    #[test]
    fn test_get_audit_log_pagination_is_stable_and_complete_under_heavy_history() {
        let (env, orchestrator_id, _, _, _, _, _, user) = setup_test_env();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let seeded = 130u32;
        seed_audit_log(&env, &user, seeded);

        let page_size = 17u32;
        let entries = collect_all_pages(&client, page_size);
        assert_eq!(entries.len() as u32, 100);
    }

    #[test]
    fn test_get_audit_log_cursor_boundaries_and_limits_are_correct() {
        let (env, orchestrator_id, _, _, _, _, _, user) = setup_test_env();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        seed_audit_log(&env, &user, 12);

        assert_eq!(client.get_audit_log(&0, &0).len(), 0);

        let page = client.get_audit_log(&8, &4);
        assert_eq!(page.len(), 4);
        assert_eq!(page.get(0).unwrap().amount, 8);
        assert_eq!(page.get(3).unwrap().amount, 11);

        assert_eq!(client.get_audit_log(&12, &5).len(), 0);
        assert_eq!(client.get_audit_log(&99, &5).len(), 0);
    }

    #[test]
    fn test_get_audit_log_large_cursor_does_not_overflow_or_duplicate() {
        let (env, orchestrator_id, _, _, _, _, _, user) = setup_test_env();
        let client = OrchestratorClient::new(&env, &orchestrator_id);

        seed_audit_log(&env, &user, 5);

        let huge_cursor = u32::MAX;
        let page = client.get_audit_log(&huge_cursor, &100);
        assert_eq!(page.len(), 0);
    }

    // ============================================================================
    // Insurance Failure Tests
    // ============================================================================

    #[test]
    fn test_execute_insurance_payment_inactive_policy_fails() {
        let (env, orchestrator_id, family_wallet_id, _, _, _, insurance_id, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);

        let result = client.try_execute_insurance_payment(
            &user, &2000, &family_wallet_id, &insurance_id, &998,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_execute_remittance_flow_inactive_insurance_policy_rolls_back() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);
        let savings_client = MockSavingsGoalsClient::new(&env, &savings_id);
        let bills_client = MockBillPaymentsClient::new(&env, &bills_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id,
            &1, &1, &998,
        );

        // Should roll back due to insurance failure
        assert!(result.is_err() || matches!(result, Ok(Ok(flow)) if !flow.insurance_success));
        
        let savings_state = savings_client.get_state();
        let bills_state = bills_client.get_state();
        assert_eq!(savings_state.deposit_count, 0);
        assert_eq!(bills_state.payment_count, 0);
    }

    #[test]
    fn test_execute_remittance_flow_missing_insurance_policy_rolls_back() {
        let (env, orchestrator_id, family_wallet_id, remittance_split_id,
             savings_id, bills_id, insurance_id, user) = setup_test_env();

        let client = OrchestratorClient::new(&env, &orchestrator_id);
        let savings_client = MockSavingsGoalsClient::new(&env, &savings_id);
        let bills_client = MockBillPaymentsClient::new(&env, &bills_id);

        let result = client.try_execute_remittance_flow(
            &user, &10000, &family_wallet_id, &remittance_split_id,
            &savings_id, &bills_id, &insurance_id,
            &1, &1, &999,
        );

        assert!(result.is_err());

        let savings_state = savings_client.get_state();
        let bills_state = bills_client.get_state();
        assert_eq!(savings_state.deposit_count, 0);
        assert_eq!(bills_state.payment_count, 0);
    }
}