# RemitWise Smart Contracts

Stellar Soroban smart contracts for the RemitWise remittance platform.

## Overview

This workspace contains the core smart contracts that power RemitWise's post-remittance financial planning features:

- **remittance_split**: Automatically splits remittances into spending, savings, bills, and insurance
- **savings_goals**: Goal-based savings with target dates and locked funds
- **bill_payments**: Automated bill payment tracking and scheduling
- **insurance**: Micro-insurance policy management and premium payments

## Prerequisites

- Rust (latest stable version)
- Stellar CLI (soroban-cli)
- Cargo

## Compatibility

### Tested Versions

These contracts have been developed and tested with the following versions:

- **Soroban SDK**: `21.0.0`
- **Soroban CLI**: `21.0.0`
- **Rust Toolchain**: `stable` (with `wasm32-unknown-unknown` and `wasm32v1-none` targets)
- **Protocol Version**: Compatible with Stellar Protocol 20+ (Soroban Phase 1)
- **Network**: Testnet and Mainnet ready

### Version Compatibility Matrix

| Component | Version | Status | Notes |
|-----------|---------|--------|-------|
| soroban-sdk | 21.0.0 | ✅ Tested | Current stable release |
| soroban-cli | 21.0.0 | ✅ Tested | Matches SDK version |
| Protocol 20 | - | ✅ Compatible | Soroban Phase 1 features |
| Protocol 21+ | - | ⚠️ Untested | Should be compatible, validation recommended |

### Upgrading to New Soroban Versions

When a new Soroban SDK or protocol version is released, follow these steps to validate and upgrade:

#### 1. Review Release Notes

Check the [Soroban SDK releases](https://github.com/stellar/rs-soroban-sdk/releases) for:
- Breaking changes in contract APIs
- New features or optimizations
- Deprecated functions
- Protocol version requirements

#### 2. Update Dependencies

Update the SDK version in all contract `Cargo.toml` files:

```toml
[dependencies]
soroban-sdk = "X.Y.Z"

[dev-dependencies]
soroban-sdk = { version = "X.Y.Z", features = ["testutils"] }
```

Contracts to update:
- `remittance_split/Cargo.toml`
- `savings_goals/Cargo.toml`
- `bill_payments/Cargo.toml`
- `insurance/Cargo.toml`
- `family_wallet/Cargo.toml`
- `data_migration/Cargo.toml`
- `reporting/Cargo.toml`
- `orchestrator/Cargo.toml`

#### 3. Update Soroban CLI

```bash
cargo install --locked --version X.Y.Z soroban-cli
```

Verify installation:
```bash
soroban version
```

#### 4. Run Full Test Suite

```bash
# Clean build artifacts
cargo clean

# Run all tests
cargo test

# Run gas benchmarks to check for performance regressions
./scripts/run_gas_benchmarks.sh
```

#### 5. Validate on Testnet

Deploy contracts to testnet and run integration tests:

```bash
# Build optimized contracts
cargo build --release --target wasm32-unknown-unknown

# Deploy to testnet
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/<contract_name>.wasm \
  --source <your-key> \
  --network testnet

# Test contract interactions
soroban contract invoke \
  --id <contract-id> \
  --source <your-key> \
  --network testnet \
  -- <function-name> <args>
```

#### 6. Check for Breaking Changes

Common breaking changes to watch for:

- **Storage API changes**: TTL management, archival patterns
- **Event emission**: Topic structure or data format changes
- **Authorization**: Auth context or signature verification changes
- **Numeric types**: Changes to `i128`, `u128`, or fixed-point math
- **Contract lifecycle**: Initialization or upgrade patterns

#### 7. Update Documentation

After successful validation:
- Update this compatibility section with new versions
- Document any migration steps in `DEPLOYMENT.md`
- Update code examples if APIs changed
- Regenerate contract bindings if needed

### Known Breaking Changes

#### SDK 21.0.0 (Current)

No breaking changes from previous stable releases affecting these contracts.

#### Future Considerations

- **Protocol 21+**: May introduce new storage pricing or TTL requirements
- **SDK 22.0.0+**: Monitor for changes to contract storage patterns, event APIs, or authorization flows

### Network Protocol Versions

The contracts are designed to be compatible with:

- **Testnet**: Currently running Protocol 20+
- **Mainnet**: Currently running Protocol 20+

Check current network protocol versions:
```bash
# Testnet
soroban network container logs stellar 2>&1 | grep "protocol version"

# Or via RPC
curl -X POST https://soroban-testnet.stellar.org \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getNetwork","params":[]}'
```

### Troubleshooting Version Issues

**Build Errors After Upgrade:**
```bash
# Clear all caches
cargo clean
rm -rf target/
rm Cargo.lock

# Rebuild
cargo build --release --target wasm32-unknown-unknown
```

**Test Failures:**
- Check for deprecated test utilities in SDK release notes
- Verify mock contract behavior hasn't changed
- Review event emission format changes

**Deployment Issues:**
- Ensure CLI version matches SDK version
- Verify network is running compatible protocol version
- Check for new deployment flags or requirements

### Reporting Compatibility Issues

If you encounter issues with a specific Soroban version:

1. Check existing [GitHub Issues](https://github.com/stellar/rs-soroban-sdk/issues)
2. Verify your environment matches tested versions
3. Create a minimal reproduction case
4. Report with version details and error logs

### Additional Resources

- **[UPGRADE_GUIDE.md](UPGRADE_GUIDE.md)** - Comprehensive upgrade procedures and version-specific migration guides
- **[VERSION_COMPATIBILITY.md](VERSION_COMPATIBILITY.md)** - Detailed compatibility matrix and testing status
- **[COMPATIBILITY_QUICK_REFERENCE.md](COMPATIBILITY_QUICK_REFERENCE.md)** - Quick reference for common compatibility tasks
- **[.github/SOROBAN_VERSION_CHECKLIST.md](.github/SOROBAN_VERSION_CHECKLIST.md)** - Validation checklist for new versions

## Installation

```bash
# Install Soroban CLI
cargo install --locked --version 21.0.0 soroban-cli

# Build all contracts
cargo build --release --target wasm32-unknown-unknown
```

## Contracts

### Remittance Split

Handles automatic allocation of remittance funds into different categories.

**Key Functions:**

- `initialize_split`: Set percentage allocation (spending, savings, bills, insurance)
- `get_split`: Get current split configuration
- `calculate_split`: Calculate actual amounts from total remittance

**Events:**
- `SplitInitializedEvent`: Emitted when split configuration is initialized
  - `spending_percent`, `savings_percent`, `bills_percent`, `insurance_percent`, `timestamp`
- `SplitCalculatedEvent`: Emitted when split amounts are calculated
  - `total_amount`, `spending_amount`, `savings_amount`, `bills_amount`, `insurance_amount`, `timestamp`

### Savings Goals

Manages goal-based savings with target dates.

**Key Functions:**

- `create_goal`: Create a new savings goal (education, medical, etc.)
- `add_to_goal`: Add funds to a goal
- `get_goal`: Get goal details
- `is_goal_completed`: Check if goal target is reached
- `archive_completed_goals`: Archive completed goals to reduce storage
- `get_archived_goals`: Query archived goals
- `restore_goal`: Restore archived goal to active storage
- `cleanup_old_archives`: Permanently delete old archives
- `get_storage_stats`: Get storage usage statistics

**Events:**
- `GoalCreatedEvent`: Emitted when a new savings goal is created
  - `goal_id`, `name`, `target_amount`, `target_date`, `timestamp`
- `FundsAddedEvent`: Emitted when funds are added to a goal
  - `goal_id`, `amount`, `new_total`, `timestamp`
- `GoalCompletedEvent`: Emitted when a goal reaches its target amount
  - `goal_id`, `name`, `final_amount`, `timestamp`

### Bill Payments

Tracks and manages bill payments with recurring support.

**Key Functions:**

- `create_bill`: Create a new bill (electricity, school fees, etc.)
- `pay_bill`: Mark a bill as paid and create next recurring bill if applicable
- `get_unpaid_bills`: Get all unpaid bills
- `get_total_unpaid`: Get total amount of unpaid bills
- `archive_paid_bills`: Archive paid bills to reduce storage
- `get_archived_bills`: Query archived bills
- `restore_bill`: Restore archived bill to active storage
- `bulk_cleanup_bills`: Permanently delete old archives
- `get_storage_stats`: Get storage usage statistics

**Events:**
- `BillCreatedEvent`: Emitted when a new bill is created
  - `bill_id`, `name`, `amount`, `due_date`, `recurring`, `timestamp`
- `BillPaidEvent`: Emitted when a bill is marked as paid
  - `bill_id`, `name`, `amount`, `timestamp`
- `RecurringBillCreatedEvent`: Emitted when a recurring bill generates the next bill
  - `bill_id`, `parent_bill_id`, `name`, `amount`, `due_date`, `timestamp`

### Insurance

Manages micro-insurance policies and premium payments.

**Key Functions:**

- `create_policy`: Create a new insurance policy
- `pay_premium`: Pay monthly premium
- `get_active_policies`: Get all active policies
- `get_total_monthly_premium`: Calculate total monthly premium cost
- `deactivate_policy`: Deactivate an insurance policy

**Events:**
- `PolicyCreatedEvent`: Emitted when a new insurance policy is created
  - `policy_id`, `name`, `coverage_type`, `monthly_premium`, `coverage_amount`, `timestamp`
- `PremiumPaidEvent`: Emitted when a premium is paid
  - `policy_id`, `name`, `amount`, `next_payment_date`, `timestamp`
- `PolicyDeactivatedEvent`: Emitted when a policy is deactivated
  - `policy_id`, `name`, `timestamp`

## Events

All contracts emit events for important state changes, enabling real-time tracking and frontend integration. Events follow Soroban best practices and include:

- **Relevant IDs**: All events include the ID of the entity being acted upon
- **Amounts**: Financial events include transaction amounts
- **Timestamps**: All events include the ledger timestamp for accurate tracking
- **Context Data**: Additional contextual information (names, dates, etc.)

### Event Topics

Each contract uses short symbol topics for efficient event identification:
- **Remittance Split**: `init`, `calc`
- **Savings Goals**: `created`, `added`, `completed`
- **Bill Payments**: `created`, `paid`, `recurring`
- **Insurance**: `created`, `paid`, `deactive`

### Querying Events

Events can be queried from the Stellar network using the Soroban SDK or via the Horizon API for frontend integration. Each event structure is exported and can be decoded using the contract's schema.

## Testing

Run tests for all contracts:

```bash
cargo test
```

Run tests for a specific contract:

```bash
cd remittance_split
cargo test
```

### USDC remittance split checks (local & CI)

- `cargo test -p remittance_split` exercises the USDC distribution logic with a mocked Stellar Asset Contract (`env.register_stellar_asset_contract_v2`) and built-in auth mocking.
- The suite covers minting the payer account, splitting across spending/savings/bills/insurance, and asserting balances along with the new allocation metadata helper.
- The same command is intended for CI so it runs without manual setup; re-run locally whenever split logic changes or new USDC paths are added.

## Gas Benchmarks

See `docs/gas-optimization.md` for methodology, before/after results, and assumptions.

### Running Locally

Run all benchmarks and generate a JSON report:

```bash
./scripts/run_gas_benchmarks.sh
```

This creates `gas_results.json` with CPU and memory costs for all contract operations.

Or run individual contract benchmarks:

```bash
RUST_TEST_THREADS=1 cargo test -p bill_payments --test gas_bench -- --nocapture
RUST_TEST_THREADS=1 cargo test -p savings_goals --test gas_bench -- --nocapture
RUST_TEST_THREADS=1 cargo test -p insurance --test gas_bench -- --nocapture
RUST_TEST_THREADS=1 cargo test -p family_wallet --test gas_bench -- --nocapture
RUST_TEST_THREADS=1 cargo test -p remittance_split --test gas_bench -- --nocapture
```

### Regression Detection

Compare current results against a baseline:

```bash
# Save current results as baseline
cp gas_results.json baseline.json

# Make changes, then compare
./scripts/run_gas_benchmarks.sh
./scripts/compare_gas_results.sh baseline.json gas_results.json 10
```

The comparison script fails if CPU or memory increases by more than the threshold (default 10%).

### CI Integration

Gas benchmarks run automatically in CI on every push and pull request. Results are uploaded as artifacts and retained for 30 days.

To view results:
1. Go to Actions tab in GitHub
2. Select a workflow run
3. Download the `gas-benchmarks` artifact
4. View `gas_results.json` for metrics

## Deployment

See the [Deployment Guide](DEPLOYMENT.md) for comprehensive deployment instructions.

Quick deploy to testnet:

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/remittance_split.wasm \
  --source <your-key> \
  --network testnet
```

## Documentation

- [README.md](README.md) - Main documentation and getting started
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture and design
- [DEPLOYMENT.md](DEPLOYMENT.md) - Deployment guide for testnet and mainnet
- [UPGRADE_GUIDE.md](UPGRADE_GUIDE.md) - Detailed Soroban version upgrade procedures
- [VERSION_COMPATIBILITY.md](VERSION_COMPATIBILITY.md) - Version compatibility matrix and testing status
- [docs/adr-admin-role.md](docs/adr-admin-role.md) - Architecture decision records

## Development

This is a basic MVP implementation. Future enhancements:

- Integration with Stellar Asset Contract (USDC)
- Cross-contract calls for automated allocation
- Multi-signature support for family wallets
- Emergency mode with priority processing

## License

MIT
