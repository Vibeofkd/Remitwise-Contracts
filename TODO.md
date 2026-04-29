# Access Audit Coverage Expansion - TODO

## Phase 1: Contracts with existing audit log infrastructure

### family_wallet
- [x] Add `append_access_audit` to `pause()`
- [x] Add `append_access_audit` to `unpause()`
- [x] Add `append_access_audit` to `set_pause_admin()`
- [x] Add `append_access_audit` to `configure_multisig()`
- [x] Add `append_access_audit` to `archive_old_transactions()`
- [x] Add `append_access_audit` to `cleanup_expired_pending()`
- [x] Add `append_access_audit` to `set_precision_spending_limit()`
- [x] Add `append_access_audit` to `set_proposal_expiry()`
- [x] Add `append_access_audit` to `set_upgrade_admin()`
- [x] Add `append_access_audit` to `set_version()`
- [ ] Add regression tests in `family_wallet/src/test.rs`

### savings_goals
- [ ] Add `append_audit` to `pause()`
- [ ] Add `append_audit` to `unpause()`
- [ ] Add `append_audit` to `pause_function()`
- [ ] Add `append_audit` to `unpause_function()`
- [ ] Add `append_audit` to `set_pause_admin()`
- [ ] Add `append_audit` to `set_upgrade_admin()`
- [ ] Add `append_audit` to `set_version()`
- [ ] Add regression tests in `savings_goals/src/test.rs`

### remittance_split
- [ ] Add `append_audit` to `set_pause_admin()`
- [ ] Add `append_audit` to `pause()`
- [ ] Add `append_audit` to `unpause()`
- [ ] Add `append_audit` to `set_upgrade_admin()`
- [ ] Add `append_audit` to `set_version()`
- [ ] Add regression tests in `remittance_split/src/test.rs`

### orchestrator
- [ ] Add `append_audit` to `set_version()`
- [ ] Add regression test in `orchestrator/src/test.rs`

## Phase 2: Contracts without audit log infrastructure

### insurance
- [ ] Add `RemitwiseEvents::emit` to `set_pause_admin()`
- [ ] Add `RemitwiseEvents::emit` to `archive_policy()`
- [ ] Add `RemitwiseEvents::emit` to `restore_policy()`
- [ ] Add tests in `insurance/src/test.rs`

### bill_payments
- [ ] Add `RemitwiseEvents::emit` to `set_pause_admin()`
- [ ] Add `RemitwiseEvents::emit` to `pause()`
- [ ] Add `RemitwiseEvents::emit` to `unpause()`
- [ ] Add `RemitwiseEvents::emit` to `pause_function()`
- [ ] Add `RemitwiseEvents::emit` to `unpause_function()`
- [ ] Add `RemitwiseEvents::emit` to `schedule_unpause()`
- [ ] Add `RemitwiseEvents::emit` to `emergency_pause_all()`
- [ ] Add `RemitwiseEvents::emit` to `archive_paid_bills()`
- [ ] Add `RemitwiseEvents::emit` to `restore_bill()`
- [ ] Add `RemitwiseEvents::emit` to `bulk_cleanup_bills()`
- [ ] Add `RemitwiseEvents::emit` to `set_upgrade_admin()`
- [ ] Add `RemitwiseEvents::emit` to `set_version()`
- [ ] Add tests in `bill_payments/src/test.rs`

### emergency_killswitch
- [ ] Add `env.events().publish` to `transfer_admin()`
- [ ] Add `env.events().publish` to `schedule_unpause()`
- [ ] Add tests in `emergency_killswitch/src/test.rs`

## Phase 3: Validation
- [ ] Run `cargo test` for all modified contracts
- [ ] Fix compilation errors
- [ ] Verify all new tests pass

