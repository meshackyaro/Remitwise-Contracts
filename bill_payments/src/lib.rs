#![no_std]

mod events;
use events::{RemitwiseEvents, EventCategory, EventPriority};

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Env, Map, String, Symbol, Vec,
};

// Event topics
const BILL_CREATED: Symbol = symbol_short!("created");
const BILL_PAID: Symbol = symbol_short!("paid");
const RECURRING_BILL_CREATED: Symbol = symbol_short!("recurring");

// Event data structures
#[derive(Clone)]
#[contracttype]
pub struct BillCreatedEvent {
    pub bill_id: u32,
    pub name: String,
    pub amount: i128,
    pub due_date: u64,
    pub recurring: bool,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct BillPaidEvent {
    pub bill_id: u32,
    pub name: String,
    pub amount: i128,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct RecurringBillCreatedEvent {
    pub bill_id: u32,
    pub parent_bill_id: u32,
    pub name: String,
    pub amount: i128,
    pub due_date: u64,
    pub timestamp: u64,
}
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Map, String,
    Vec,
};

mod schedule;
use schedule::{Schedule, ScheduleEvent};

// Storage TTL constants
const INSTANCE_LIFETIME_THRESHOLD: u32 = 17280; // ~1 day
const INSTANCE_BUMP_AMOUNT: u32 = 518400; // ~30 days

/// Bill data structure with owner tracking for access control
#[derive(Clone)]
#[contracttype]
pub struct Bill {
    pub id: u32,
    pub owner: Address,
    pub name: String,
    pub amount: i128,
    pub due_date: u64,
    pub recurring: bool,
    pub frequency_days: u32,
    pub paid: bool,
    pub created_at: u64,
    pub paid_at: Option<u64>,
    pub schedule_id: Option<u32>,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    BillNotFound = 1,
    BillAlreadyPaid = 2,
    InvalidAmount = 3,
    InvalidFrequency = 4,
    Unauthorized = 5,
}

/// Events emitted by the contract for audit trail
#[contracttype]
#[derive(Clone)]
pub enum BillEvent {
    Created,
    Paid,
}

#[contract]
pub struct BillPayments;

#[contractimpl]
impl BillPayments {
    /// Create a new bill
    pub fn create_bill(
        env: Env,
        owner: Address,
        name: String,
        amount: i128,
        due_date: u64,
        recurring: bool,
        frequency_days: u32,
    ) -> Result<u32, Error> {
        owner.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        if recurring && frequency_days == 0 {
            return Err(Error::InvalidFrequency);
        }

        Self::extend_instance_ttl(&env);
        let mut bills: Map<u32, Bill> = env
            .storage()
            .instance()
            .get(&symbol_short!("BILLS"))
            .unwrap_or_else(|| Map::new(&env));

        let next_id = env
            .storage()
            .instance()
            .get(&symbol_short!("NEXT_ID"))
            .unwrap_or(0u32)
            + 1;

        let current_time = env.ledger().timestamp();
        let bill = Bill {
            id: next_id,
            owner: owner.clone(),
            name: name.clone(),
            amount,
            due_date,
            recurring,
            frequency_days,
            paid: false,
            created_at: current_time,
            paid_at: None,
            schedule_id: None,
        };

        let bill_owner = bill.owner.clone();
        bills.set(next_id, bill);
        env.storage()
            .instance()
            .set(&symbol_short!("BILLS"), &bills);
        env.storage()
            .instance()
            .set(&symbol_short!("NEXT_ID"), &next_id);

        // Emit event for audit trail
        env.events().publish(
            (symbol_short!("bill"), BillEvent::Created),
            (next_id, bill_owner),
        );

        Ok(next_id)
    }

    /// Mark a bill as paid
    pub fn pay_bill(env: Env, caller: Address, bill_id: u32) -> Result<(), Error> {
        caller.require_auth();

        Self::extend_instance_ttl(&env);
        let mut bills: Map<u32, Bill> = env
            .storage()
            .instance()
            .get(&symbol_short!("BILLS"))
            .unwrap_or_else(|| Map::new(&env));

        let mut bill = bills.get(bill_id).ok_or(Error::BillNotFound)?;

        // Access control: verify caller is the owner
        if bill.owner != caller {
            return Err(Error::Unauthorized);
        }

        if bill.paid {
            return Err(Error::BillAlreadyPaid);
        }

        let current_time = env.ledger().timestamp();
        bill.paid = true;
        bill.paid_at = Some(current_time);

        if bill.recurring {
            let next_due_date = bill.due_date + (bill.frequency_days as u64 * 86400);
            let next_id = env
                .storage()
                .instance()
                .get(&symbol_short!("NEXT_ID"))
                .unwrap_or(0u32)
                + 1;

            let next_bill = Bill {
                id: next_id,
                owner: bill.owner.clone(),
                name: bill.name.clone(),
                amount: bill.amount,
                due_date: next_due_date,
                recurring: true,
                frequency_days: bill.frequency_days,
                paid: false,
                created_at: current_time,
                paid_at: None,
                schedule_id: bill.schedule_id,
            };
            bills.set(next_id, next_bill);
            env.storage()
                .instance()
                .set(&symbol_short!("NEXT_ID"), &next_id);
        }

        // Capture amount before we re-insert the bill (for event data)
        let paid_amount = bill.amount;

        bills.set(bill_id, bill);
        env.storage()
            .instance()
            .set(&symbol_short!("BILLS"), &bills);

        // Standardized Payment Event
        let event_data = (bill_id, caller, paid_amount);
        RemitwiseEvents::emit(
            &env,
            EventCategory::Transaction, // Money moved -> Transaction
            EventPriority::High,        // Payment is High priority
            symbol_short!("paid"),
            event_data
        );

        Ok(())
    }

    /// Get a bill by ID
    pub fn get_bill(env: Env, bill_id: u32) -> Option<Bill> {
        let bills: Map<u32, Bill> = env
            .storage()
            .instance()
            .get(&symbol_short!("BILLS"))
            .unwrap_or_else(|| Map::new(&env));

        bills.get(bill_id)
    }

    /// Get all unpaid bills for a specific owner
    pub fn get_unpaid_bills(env: Env, owner: Address) -> Vec<Bill> {
        let bills: Map<u32, Bill> = env
            .storage()
            .instance()
            .get(&symbol_short!("BILLS"))
            .unwrap_or_else(|| Map::new(&env));

        let mut result = Vec::new(&env);
        for (_, bill) in bills.iter() {
            if !bill.paid && bill.owner == owner {
                result.push_back(bill);
            }
        }
        result
    }

    /// Get all overdue unpaid bills
    pub fn get_overdue_bills(env: Env) -> Vec<Bill> {
        let current_time = env.ledger().timestamp();
        let bills: Map<u32, Bill> = env
            .storage()
            .instance()
            .get(&symbol_short!("BILLS"))
            .unwrap_or_else(|| Map::new(&env));

        let mut result = Vec::new(&env);
        for (_, bill) in bills.iter() {
            if !bill.paid && bill.due_date < current_time {
                result.push_back(bill);
            }
        }
        result
    }

    /// Get total amount of unpaid bills for a specific owner
    pub fn get_total_unpaid(env: Env, owner: Address) -> i128 {
        let mut total = 0i128;
        let bills: Map<u32, Bill> = env
            .storage()
            .instance()
            .get(&symbol_short!("BILLS"))
            .unwrap_or_else(|| Map::new(&env));

        for (_, bill) in bills.iter() {
            if !bill.paid && bill.owner == owner {
                total += bill.amount;
            }
        }
        total
    }

    /// Cancel/delete a bill
    pub fn cancel_bill(env: Env, bill_id: u32) -> Result<(), Error> {
        let mut bills: Map<u32, Bill> = env
            .storage()
            .instance()
            .get(&symbol_short!("BILLS"))
            .unwrap_or_else(|| Map::new(&env));

        if bills.get(bill_id).is_none() {
            return Err(Error::BillNotFound);
        }

        bills.remove(bill_id);
        env.storage()
            .instance()
            .set(&symbol_short!("BILLS"), &bills);

        // Added Cancellation Event
        RemitwiseEvents::emit(
            &env,
            EventCategory::State,
            EventPriority::Medium,
            symbol_short!("canceled"),
            bill_id
        );

        Ok(())
    }

    /// Get all bills (paid and unpaid)
    pub fn get_all_bills(env: Env) -> Vec<Bill> {
        let bills: Map<u32, Bill> = env
            .storage()
            .instance()
            .get(&symbol_short!("BILLS"))
            .unwrap_or_else(|| Map::new(&env));

        let mut result = Vec::new(&env);
        for (_, bill) in bills.iter() {
            result.push_back(bill);
        }
        result
    }

    /// Archive paid bills that were paid before the specified timestamp.
    /// Moves paid bills from active storage to archive storage.
    ///
    /// # Arguments
    /// * `caller` - Address of the caller (must authorize)
    /// * `before_timestamp` - Archive bills paid before this timestamp
    ///
    /// # Returns
    /// Number of bills archived
    pub fn archive_paid_bills(env: Env, caller: Address, before_timestamp: u64) -> u32 {
        caller.require_auth();
        Self::extend_instance_ttl(&env);

        let bills: Map<u32, Bill> = env
            .storage()
            .instance()
            .get(&symbol_short!("BILLS"))
            .unwrap_or_else(|| Map::new(&env));

        let mut bill = bills.get(bill_id).ok_or(Error::BillNotFound)?;

        if bill.owner != owner {
            return Err(Error::Unauthorized);
        }

        let current_time = env.ledger().timestamp();
        if next_due <= current_time {
            return Err(Error::InvalidAmount);
        }

        Self::extend_instance_ttl(&env);

        let mut schedules: Map<u32, Schedule> = env
            .storage()
            .instance()
            .get(&symbol_short!("SCHEDULES"))
            .unwrap_or_else(|| Map::new(&env));

        let next_schedule_id = env
            .storage()
            .instance()
            .get(&symbol_short!("NEXT_SCH"))
            .unwrap_or(0u32)
            + 1;

        let schedule = Schedule {
            id: next_schedule_id,
            owner: owner.clone(),
            next_due,
            interval,
            recurring: interval > 0,
            active: true,
            created_at: current_time,
            last_executed: None,
            missed_count: 0,
        };

        bill.schedule_id = Some(next_schedule_id);

        schedules.set(next_schedule_id, schedule);
        env.storage()
            .instance()
            .set(&symbol_short!("SCHEDULES"), &schedules);
        env.storage()
            .instance()
            .set(&symbol_short!("NEXT_SCH"), &next_schedule_id);

        let mut bills_mut = bills;
        bills_mut.set(bill_id, bill);
        env.storage()
            .instance()
            .set(&symbol_short!("BILLS"), &bills_mut);

        env.events().publish(
            (symbol_short!("bill"), ArchiveEvent::BillsArchived),
            (archived_count, caller),
        );

        Ok(next_schedule_id)
    }

    /// Get all archived bills for a specific owner
    ///
    /// # Arguments
    /// * `owner` - Address of the bill owner
    ///
    /// # Returns
    /// Vec of all ArchivedBill structs belonging to the owner
    pub fn get_archived_bills(env: Env, owner: Address) -> Vec<ArchivedBill> {
        let archived: Map<u32, ArchivedBill> = env
            .storage()
            .instance()
            .get(&symbol_short!("SCHEDULES"))
            .unwrap_or_else(|| Map::new(&env));

        let mut result = Vec::new(&env);
        for (_, bill) in archived.iter() {
            if bill.owner == owner {
                result.push_back(bill);
            }
        }
        result
    }

    /// Get a specific archived bill by ID
    ///
    /// # Arguments
    /// * `bill_id` - ID of the archived bill
    ///
    /// # Returns
    /// ArchivedBill struct or None if not found
    pub fn get_archived_bill(env: Env, bill_id: u32) -> Option<ArchivedBill> {
        let archived: Map<u32, ArchivedBill> = env
            .storage()
            .instance()
            .get(&symbol_short!("ARCH_BILL"))
            .unwrap_or_else(|| Map::new(&env));

        archived.get(bill_id)
    }

    /// Restore an archived bill back to active storage
    ///
    /// # Arguments
    /// * `caller` - Address of the caller (must be the bill owner)
    /// * `bill_id` - ID of the bill to restore
    ///
    /// # Returns
    /// Ok(()) if restoration was successful
    ///
    /// # Errors
    /// * `BillNotFound` - If bill is not found in archive
    /// * `Unauthorized` - If caller is not the bill owner
    pub fn restore_bill(env: Env, caller: Address, bill_id: u32) -> Result<(), Error> {
        caller.require_auth();

        Self::extend_instance_ttl(&env);

        let mut schedules: Map<u32, Schedule> = env
            .storage()
            .instance()
            .get(&symbol_short!("SCHEDULES"))
            .unwrap_or_else(|| Map::new(&env));

        let mut schedule = schedules.get(schedule_id).ok_or(Error::BillNotFound)?;

        if schedule.owner != caller {
            return Err(Error::Unauthorized);
        }

        schedule.active = false;

        schedules.set(schedule_id, schedule);
        env.storage()
            .instance()
            .set(&symbol_short!("SCHEDULES"), &schedules);

        env.events().publish(
            (symbol_short!("bill"), ArchiveEvent::BillRestored),
            (bill_id, caller),
        );

        Ok(())
    }

    /// Permanently delete old archives before specified timestamp
    ///
    /// # Arguments
    /// * `caller` - Address of the caller (must authorize)
    /// * `before_timestamp` - Delete archives created before this timestamp
    ///
    /// # Returns
    /// Number of archives deleted
    pub fn bulk_cleanup_bills(env: Env, caller: Address, before_timestamp: u64) -> u32 {
        caller.require_auth();
        Self::extend_instance_ttl(&env);

        let current_time = env.ledger().timestamp();
        let mut executed = Vec::new(&env);

        let mut schedules: Map<u32, Schedule> = env
            .storage()
            .instance()
            .get(&symbol_short!("SCHEDULES"))
            .unwrap_or_else(|| Map::new(&env));

        let mut bills: Map<u32, Bill> = env
            .storage()
            .instance()
            .get(&symbol_short!("BILLS"))
            .unwrap_or_else(|| Map::new(&env));

        for (schedule_id, mut schedule) in schedules.iter() {
            if !schedule.active || schedule.next_due > current_time {
                continue;
            }

            let bill_id = Self::find_bill_by_schedule(&bills, schedule_id);
            if let Some(bid) = bill_id {
                if let Some(mut bill) = bills.get(bid) {
                    if !bill.paid {
                        bill.paid = true;
                        bill.paid_at = Some(current_time);

                        if bill.recurring {
                            let next_due_date =
                                bill.due_date + (bill.frequency_days as u64 * 86400);
                            let next_id = env
                                .storage()
                                .instance()
                                .get(&symbol_short!("NEXT_ID"))
                                .unwrap_or(0u32)
                                + 1;

                            let next_bill = Bill {
                                id: next_id,
                                owner: bill.owner.clone(),
                                name: bill.name.clone(),
                                amount: bill.amount,
                                due_date: next_due_date,
                                recurring: true,
                                frequency_days: bill.frequency_days,
                                paid: false,
                                created_at: current_time,
                                paid_at: None,
                                schedule_id: bill.schedule_id,
                            };
                            bills.set(next_id, next_bill);
                            env.storage()
                                .instance()
                                .set(&symbol_short!("NEXT_ID"), &next_id);
                        }

                        bills.set(bid, bill);

                        env.events().publish(
                            (symbol_short!("bill"), BillEvent::Paid),
                            (bid, schedule.owner.clone()),
                        );
                    }
                }
            }

            schedule.last_executed = Some(current_time);

            if schedule.recurring && schedule.interval > 0 {
                let mut missed = 0u32;
                let mut next = schedule.next_due + schedule.interval;
                while next <= current_time {
                    missed += 1;
                    next += schedule.interval;
                }
                schedule.missed_count += missed;
                schedule.next_due = next;

                if missed > 0 {
                    env.events().publish(
                        (symbol_short!("schedule"), ScheduleEvent::Missed),
                        (schedule_id, missed),
                    );
                }
            } else {
                schedule.active = false;
            }

            schedules.set(schedule_id, schedule);
            executed.push_back(schedule_id);

            env.events().publish(
                (symbol_short!("schedule"), ScheduleEvent::Executed),
                schedule_id,
            );
        }

        env.storage()
            .instance()
            .set(&symbol_short!("ARCH_BILL"), &archived);

        Self::update_storage_stats(&env);

        env.events().publish(
            (symbol_short!("bill"), ArchiveEvent::ArchivesCleaned),
            (deleted_count, caller),
        );

        deleted_count
    }

    /// Get storage usage statistics
    ///
    /// # Returns
    /// StorageStats struct with current storage metrics
    pub fn get_storage_stats(env: Env) -> StorageStats {
        env.storage()
            .instance()
            .set(&symbol_short!("BILLS"), &bills);

        executed
    }

    /// Get all schedules for an owner
    pub fn get_schedules(env: Env, owner: Address) -> Vec<Schedule> {
        let schedules: Map<u32, Schedule> = env
            .storage()
            .instance()
            .get(&symbol_short!("SCHEDULES"))
            .unwrap_or_else(|| Map::new(&env));

        let mut result = Vec::new(&env);
        for (_, schedule) in schedules.iter() {
            if schedule.owner == owner {
                result.push_back(schedule);
            }
        }
        result
    }

    /// Get a specific schedule
    pub fn get_schedule(env: Env, schedule_id: u32) -> Option<Schedule> {
        let schedules: Map<u32, Schedule> = env
            .storage()
            .instance()
            .get(&symbol_short!("SCHEDULES"))
            .unwrap_or_else(|| Map::new(&env));

        schedules.get(schedule_id)
    }

    fn find_bill_by_schedule(bills: &Map<u32, Bill>, schedule_id: u32) -> Option<u32> {
        for (bill_id, bill) in bills.iter() {
            if bill.schedule_id == Some(schedule_id) {
                return Some(bill_id);
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Events;

    #[test]
    fn test_create_bill_emits_event() {
        let env = Env::default();
        let contract_id = env.register_contract(None, BillPayments);
        let client = BillPaymentsClient::new(&env, &contract_id);

        // Create a bill
        let bill_id = client.create_bill(
            &String::from_str(&env, "Electricity"),
            &500,
            &1735689600,
            &false,
            &0,
        );
        assert_eq!(bill_id, 1);

        // Verify event was emitted
        let events = env.events().all();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_pay_bill_emits_event() {
        let env = Env::default();
        let contract_id = env.register_contract(None, BillPayments);
        let client = BillPaymentsClient::new(&env, &contract_id);

        // Create a bill
        let bill_id = client.create_bill(
            &String::from_str(&env, "Water Bill"),
            &300,
            &1735689600,
            &false,
            &0,
        );

        // Get events before paying
        let events_before = env.events().all().len();

        // Pay the bill
        let result = client.pay_bill(&bill_id);
        assert!(result);

        // Verify BillPaid event was emitted (1 new event)
        let events_after = env.events().all().len();
        assert_eq!(events_after - events_before, 1);
    }

    #[test]
    fn test_pay_recurring_bill_emits_multiple_events() {
        let env = Env::default();
        let contract_id = env.register_contract(None, BillPayments);
        let client = BillPaymentsClient::new(&env, &contract_id);

        // Create a recurring bill
        let bill_id = client.create_bill(
            &String::from_str(&env, "Rent"),
            &1000,
            &1735689600,
            &true,
            &30, // Monthly
        );

        // Get events before paying
        let events_before = env.events().all().len();

        // Pay the recurring bill
        client.pay_bill(&bill_id);

        // Should emit BillPaid and RecurringBillCreated events (2 new events)
        let events_after = env.events().all().len();
        assert_eq!(events_after - events_before, 2);
    }

    #[test]
    fn test_multiple_bills_emit_separate_events() {
        let env = Env::default();
        let contract_id = env.register_contract(None, BillPayments);
        let client = BillPaymentsClient::new(&env, &contract_id);

        // Create multiple bills
        client.create_bill(
            &String::from_str(&env, "Bill 1"),
            &100,
            &1735689600,
            &false,
            &0,
        );
        client.create_bill(
            &String::from_str(&env, "Bill 2"),
            &200,
            &1735689600,
            &false,
            &0,
        );
        client.create_bill(
            &String::from_str(&env, "Bill 3"),
            &300,
            &1735689600,
            &true,
            &30,
        );

        // Should have 3 BillCreated events
        let events = env.events().all();
        assert_eq!(events.len(), 3);
    }
}
mod test;