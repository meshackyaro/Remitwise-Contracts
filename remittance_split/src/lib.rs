#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, vec, Env, Vec};

#[contract]
pub struct RemittanceSplit;

#[contractimpl]
impl RemittanceSplit {
    /// Set or update the split percentages used to allocate remittances.
    ///
    /// # Arguments
    /// * `spending_percent` - Percent allocated to spending
    /// * `savings_percent` - Percent allocated to savings
    /// * `bills_percent` - Percent allocated to bills
    /// * `insurance_percent` - Percent allocated to insurance
    ///
    /// # Returns
    /// `true` when the inputs are valid and stored, `false` otherwise.
    pub fn initialize_split(
        env: Env,
        spending_percent: u32,
        savings_percent: u32,
        bills_percent: u32,
        insurance_percent: u32,
    ) -> bool {
        if !Self::is_valid_split(
            spending_percent,
            savings_percent,
            bills_percent,
            insurance_percent,
        ) {
            return false;
        }

        env.storage().instance().set(
            &symbol_short!("SPLIT"),
            &vec![
                &env,
                spending_percent,
                savings_percent,
                bills_percent,
                insurance_percent,
            ],
        );

        true
    }

    /// Read the current split configuration.
    ///
    /// # Returns
    /// A Vec with [spending, savings, bills, insurance] percentages.
    /// If no configuration is stored, returns the default (50, 30, 15, 5).
    pub fn get_split(env: Env) -> Vec<u32> {
        env.storage()
            .instance()
            .get(&symbol_short!("SPLIT"))
            .unwrap_or_else(|| vec![&env, 50, 30, 15, 5])
    }

    /// Calculate split amounts from a total remittance amount.
    ///
    /// # Arguments
    /// * `total_amount` - Total remittance amount to split
    ///
    /// # Returns
    /// A Vec with [spending, savings, bills, insurance] amounts.
    /// Returns zeros when `total_amount` is zero or negative.
    pub fn calculate_split(env: Env, total_amount: i128) -> Vec<i128> {
        if total_amount <= 0 {
            // Skip storage read for zero/invalid inputs to save gas.
            return vec![&env, 0, 0, 0, 0];
        }

        let split = Self::get_split(env.clone());
        let spending_percent = split.get(0).unwrap();
        let savings_percent = split.get(1).unwrap();
        let bills_percent = split.get(2).unwrap();

        let spending = Self::split_amount(total_amount, spending_percent);
        let savings = Self::split_amount(total_amount, savings_percent);
        let bills = Self::split_amount(total_amount, bills_percent);
        let insurance = total_amount - spending - savings - bills;

        vec![&env, spending, savings, bills, insurance]
    }

    /// Validate a percentage split for bounds and sum.
    fn is_valid_split(
        spending_percent: u32,
        savings_percent: u32,
        bills_percent: u32,
        insurance_percent: u32,
    ) -> bool {
        if spending_percent > 100
            || savings_percent > 100
            || bills_percent > 100
            || insurance_percent > 100
        {
            return false;
        }

        let total = spending_percent as u64
            + savings_percent as u64
            + bills_percent as u64
            + insurance_percent as u64;
        total == 100
    }

    /// Compute a percentage share without risking multiplication overflow.
    fn split_amount(total_amount: i128, percent: u32) -> i128 {
        let percent = percent as i128;
        let quotient = total_amount / 100;
        let remainder = total_amount % 100;

        quotient * percent + (remainder * percent) / 100
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{contractclient, vec, Env, Vec};

    #[contractclient(name = "RemittanceSplitClient")]
    pub trait RemittanceSplitTrait {
        fn initialize_split(
            env: Env,
            spending_percent: u32,
            savings_percent: u32,
            bills_percent: u32,
            insurance_percent: u32,
        ) -> bool;
        fn get_split(env: Env) -> Vec<u32>;
        fn calculate_split(env: Env, total_amount: i128) -> Vec<i128>;
    }

    fn setup() -> (Env, soroban_sdk::Address) {
        let env = Env::default();
        let contract_id = env.register_contract(None, RemittanceSplit);
        (env, contract_id)
    }

    #[test]
    fn test_initialize_split_valid() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        let ok = client.initialize_split(&40, &30, &20, &10);
        assert!(ok);

        let split = client.get_split();
        assert_eq!(split, vec![&env, 40, 30, 20, 10]);
    }

    #[test]
    fn test_initialize_split_invalid_sum() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        let ok = client.initialize_split(&50, &30, &10, &5);
        assert!(!ok);

        let split = client.get_split();
        assert_eq!(split, vec![&env, 50, 30, 15, 5]);
    }

    #[test]
    fn test_initialize_split_invalid_over_100() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        let ok = client.initialize_split(&101, &0, &0, &0);
        assert!(!ok);

        let split = client.get_split();
        assert_eq!(split, vec![&env, 50, 30, 15, 5]);
    }

    #[test]
    fn test_initialize_split_update_existing() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        assert!(client.initialize_split(&60, &20, &15, &5));
        assert!(client.initialize_split(&25, &25, &25, &25));

        let split = client.get_split();
        assert_eq!(split, vec![&env, 25, 25, 25, 25]);
    }

    #[test]
    fn test_initialize_split_invalid_does_not_overwrite() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        assert!(client.initialize_split(&60, &20, &15, &5));
        assert!(!client.initialize_split(&60, &20, &15, &10));

        let split = client.get_split();
        assert_eq!(split, vec![&env, 60, 20, 15, 5]);
    }

    #[test]
    fn test_initialize_split_with_zero_percentages() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        let ok = client.initialize_split(&100, &0, &0, &0);
        assert!(ok);

        let split = client.get_split();
        assert_eq!(split, vec![&env, 100, 0, 0, 0]);
    }

    #[test]
    fn test_get_split_default() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        let split = client.get_split();
        assert_eq!(split, vec![&env, 50, 30, 15, 5]);
    }

    #[test]
    fn test_get_split_configured_values() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        assert!(client.initialize_split(&70, &20, &5, &5));
        let split = client.get_split();
        assert_eq!(split, vec![&env, 70, 20, 5, 5]);
    }

    #[test]
    fn test_calculate_split_basic() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        assert!(client.initialize_split(&50, &30, &15, &5));
        let amounts = client.calculate_split(&1000);
        assert_eq!(amounts, vec![&env, 500, 300, 150, 50]);
    }

    #[test]
    fn test_calculate_split_rounding_total_matches() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        assert!(client.initialize_split(&50, &30, &15, &5));
        let total_amount: i128 = 101;
        let amounts = client.calculate_split(&total_amount);
        let total = amounts.get(0).unwrap()
            + amounts.get(1).unwrap()
            + amounts.get(2).unwrap()
            + amounts.get(3).unwrap();

        assert_eq!(total, total_amount);
        assert_eq!(amounts, vec![&env, 50, 30, 15, 6]);
    }

    #[test]
    fn test_calculate_split_rounding_varied_percentages() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        assert!(client.initialize_split(&33, &33, &33, &1));
        let total_amount: i128 = 101;
        let amounts = client.calculate_split(&total_amount);
        let total = amounts.get(0).unwrap()
            + amounts.get(1).unwrap()
            + amounts.get(2).unwrap()
            + amounts.get(3).unwrap();

        assert_eq!(total, total_amount);
        assert_eq!(amounts, vec![&env, 33, 33, 33, 2]);
    }

    #[test]
    fn test_calculate_split_small_amount_rounding() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        let amounts = client.calculate_split(&1);
        assert_eq!(amounts, vec![&env, 0, 0, 0, 1]);
    }

    #[test]
    fn test_calculate_split_zero_amount() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        let amounts = client.calculate_split(&0);
        assert_eq!(amounts, vec![&env, 0, 0, 0, 0]);
    }

    #[test]
    fn test_calculate_split_negative_amount() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        let amounts = client.calculate_split(&-10);
        assert_eq!(amounts, vec![&env, 0, 0, 0, 0]);
    }

    #[test]
    fn test_calculate_split_default_when_uninitialized() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        let amounts = client.calculate_split(&200);
        assert_eq!(amounts, vec![&env, 100, 60, 30, 10]);
    }

    #[test]
    fn test_calculate_split_with_zero_categories() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        assert!(client.initialize_split(&100, &0, &0, &0));
        let amounts = client.calculate_split(&123);
        assert_eq!(amounts, vec![&env, 123, 0, 0, 0]);
    }

    #[test]
    fn test_calculate_split_large_non_divisible_amount() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        let total_amount: i128 = 1_000_000_000_000_000_001;
        let amounts = client.calculate_split(&total_amount);
        let total = amounts.get(0).unwrap()
            + amounts.get(1).unwrap()
            + amounts.get(2).unwrap()
            + amounts.get(3).unwrap();

        assert_eq!(total, total_amount);
        assert_eq!(
            amounts,
            vec![&env, 500_000_000_000_000_000, 300_000_000_000_000_000, 150_000_000_000_000_000, 50_000_000_000_000_001]
        );
    }

    #[test]
    fn test_calculate_split_large_amount() {
        let (env, contract_id) = setup();
        let client = RemittanceSplitClient::new(&env, &contract_id);

        let total_amount: i128 = 1_000_000_000_000_000_000;
        let amounts = client.calculate_split(&total_amount);
        let expected = vec![
            &env,
            500_000_000_000_000_000,
            300_000_000_000_000_000,
            150_000_000_000_000_000,
            50_000_000_000_000_000,
        ];

        assert_eq!(amounts, expected);
    }
}
