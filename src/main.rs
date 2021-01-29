use cached::SizedCache;
use client::Client;
use std::collections::HashMap;
use std::env::args_os;
use transaction::{
    read_from_csv_file, DisputableTransaction, DisputableType::*, Transaction, Type::*,
};
use transaction_set::{
    CachedClient, Client as TransactionSetClient, MemoryClient, State::*, UpdateFailure::*,
};

mod client;
pub mod decimal;
mod transaction;
mod transaction_set;

const CACHE_SIZE: usize = 10;

pub fn process_transaction<T: TransactionSetClient>(
    transaction: Transaction,
    clients: &mut HashMap<u16, Client>,
    tx_record: &mut T,
) {
    let client = clients
        .entry(transaction.client_id)
        .or_insert(Client::new(transaction.client_id));
    match transaction.type_ {
        Disputable(Deposit(ref deposit)) => {
            client.deposit(deposit.clone());
            tx_record.store(DisputableTransaction {
                transaction_id: transaction.transaction_id,
                client_id: transaction.client_id,
                type_: Deposit(deposit.clone()),
            });
        }
        Disputable(Withdrawal(ref withdrawal)) => match client.withdraw(withdrawal.clone()) {
            Err(Some(present)) => eprintln!(
                "tx {}: Failed to withdraw {} from client {}. Only {} funds present",
                transaction.transaction_id, withdrawal, transaction.client_id, present
            ),
            Err(None) => eprintln!(
                "tx {}: Failed to withdraw {} from client {}. Client frozen.",
                transaction.transaction_id, withdrawal, transaction.client_id
            ),
            Ok(()) => tx_record.store(DisputableTransaction {
                transaction_id: transaction.transaction_id,
                client_id: transaction.client_id,
                type_: Withdrawal(withdrawal.clone()),
            }),
        },
        Dispute => match tx_record.update(transaction.transaction_id, Disputed) {
            Err(NotFound) => eprintln!(
                "tx {}: Failed to dispute transaction: Not found.",
                transaction.transaction_id
            ),
            Err(WrongState(s)) => eprintln!(
                "tx {}: Failed to dispute transaction: Wrong state {:?}.",
                transaction.transaction_id, s
            ),
            Ok(disputed) => match disputed.type_ {
                Deposit(value) => client.dispute_deposit(value),
                Withdrawal(value) => client.dispute_withdrawal(value),
            },
        },
        Resolve => match tx_record.update(transaction.transaction_id, Resolved) {
            Err(NotFound) => eprintln!(
                "tx {}: Failed to resolve transaction: Not found.",
                transaction.transaction_id
            ),
            Err(WrongState(s)) => eprintln!(
                "tx {}: Failed to resolve transaction: Wrong state {:?}.",
                transaction.transaction_id, s
            ),
            Ok(disputed) => match match disputed.type_ {
                Deposit(value) => (value.clone(), client.resolve_deposit(value)),
                Withdrawal(value) => (value.clone(), client.resolve_withdrawal(value)),
            } {
                (_, Ok(_)) => {
                    // TODO: error handle?
                    let _ = tx_record.update(transaction.transaction_id, Committed);
                }
                (value, Err(resolveable)) => {
                    eprintln!("tx {}: Failed to resolve transaction: Requested {} funds, only {} available.", transaction.transaction_id, value, resolveable);
                    // TODO: error handle?
                    let _ = tx_record.update(transaction.transaction_id, Disputed);
                }
            },
        },
        Chargeback => match tx_record.update(transaction.transaction_id, ChargedBack) {
            Err(NotFound) => eprintln!(
                "tx {}: Failed to chargeback transaction: Not found.",
                transaction.transaction_id
            ),
            Err(WrongState(s)) => eprintln!(
                "tx {}: Failed to chargeback transaction: Wrong state {:?}.",
                transaction.transaction_id, s
            ),
            Ok(disputed) => match match disputed.type_ {
                Deposit(value) => (value.clone(), client.chargeback_deposit(value)),
                Withdrawal(value) => (value.clone(), client.chargeback_withdrawal(value)),
            } {
                (_, Ok(_)) => {
                    // TODO: error handle?
                    let _ = tx_record.update(transaction.transaction_id, ChargedBackFinal);
                }
                (value, Err(chargeable)) => {
                    eprintln!("tx {}: Failed to chargeback transaction: Requested {} funds, only {} available.", transaction.transaction_id, value, chargeable);
                    // TODO: error handle?
                    let _ = tx_record.update(transaction.transaction_id, Disputed);
                }
            },
        },
    }
}

fn main() -> std::io::Result<()> {
    let path = args_os().skip(1).next().expect("missing filename");
    // Optimization: use hashset
    // Blocked by https://github.com/rust-lang/rust/issues/60896
    //
    // If there are really only 2^16 possible clients though, we could probably store this in memory
    // If client count isn't actually that limited, if it got big enough we'd eventually want to move the data out
    // of RAM and onto disk, possibly even remotely in a distributed KVP datastore using an interface similar to `TransactionSet`
    let mut clients = HashMap::new();

    let mut tx_record =
        CachedClient::new(MemoryClient::default(), SizedCache::with_size(CACHE_SIZE));

    for transaction in read_from_csv_file(path)? {
        let transaction = match transaction {
            Ok(transaction) => transaction,
            Err(e) => {
                eprintln!("failed to parse transaction: {}", e);
                continue;
            }
        };
        process_transaction(transaction, &mut clients, &mut tx_record);
    }

    let mut writer = csv::Writer::from_writer(std::io::stdout());
    for client in clients.values() {
        writer.serialize(client)?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use csv::{ReaderBuilder, Trim};
    use decimal::Decimal;
    use rand::prelude::*;
    use std::collections::HashMap;
    use transaction_set::{CachedClient, MemoryClient};

    #[test]
    fn basic_process_test() {
        let mut clients = HashMap::new();
        let mut tx_record =
            CachedClient::new(MemoryClient::default(), SizedCache::with_size(CACHE_SIZE));

        let data = "\
type,       client,  tx, amount
deposit,         1,   1,    1.0
chargeback,         10,   21,
";

        let mut rdr = ReaderBuilder::new()
            .trim(Trim::All)
            .from_reader(data.as_bytes());

        for transaction in rdr.deserialize() {
            let transaction = match transaction {
                Ok(transaction) => transaction,
                Err(e) => {
                    eprintln!("failed to parse transaction: {}", e);
                    continue;
                }
            };
            process_transaction(transaction, &mut clients, &mut tx_record);
        }
    }

    #[test]
    fn random_process_test() {
        let mut clients = HashMap::new();
        let mut tx_record =
            CachedClient::new(MemoryClient::default(), SizedCache::with_size(CACHE_SIZE));

        fn generate_transaction<F: FnOnce(u32) -> bool>(exists: F) -> Transaction {
            let mut rng = thread_rng();
            let transaction_id = rng.gen();
            let type_ = if !exists(transaction_id) {
                match rng.gen() {
                    false => Disputable(Deposit(Decimal::new(rng.gen_range(0..65000), rng.gen()))),
                    true => Disputable(Withdrawal(Decimal::new(rng.gen_range(0..1000), rng.gen()))),
                }
            } else {
                match rng.gen_range(0..3) {
                    0 => Dispute,
                    1 => Resolve,
                    _ => Chargeback,
                }
            };
            Transaction {
                client_id: rng.gen_range(0..500),
                transaction_id: transaction_id,
                type_: type_,
            }
        }
        for _ in 0..1000 * 1000 {
            process_transaction(
                generate_transaction(|x| tx_record.access(x).is_some()),
                &mut clients,
                &mut tx_record,
            );
        }
    }
}
