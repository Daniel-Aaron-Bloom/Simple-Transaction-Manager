use crate::decimal::Decimal;
use csv::{ReaderBuilder, Trim};
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::convert::TryFrom;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::Path;

/// We can't use tags, so use an intermediary
#[derive(Serialize, Deserialize, Debug)]
struct CsvTransaction {
    #[serde(rename = "type")]
    type_: CsvType,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
}

// TODO: A macro might be useful to generate this as a part of `Type`
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum CsvType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

// TODO: Disputes of chargebacks... yay recursion!
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DisputableType {
    Deposit(Decimal),
    Withdrawal(Decimal),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DisputableTransaction {
    pub client_id: u16,
    pub transaction_id: u32,
    pub type_: DisputableType,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(try_from = "CsvTransaction")]
pub struct Transaction {
    pub client_id: u16,
    pub transaction_id: u32,
    pub type_: Type,
}

impl Hash for Transaction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.transaction_id.hash(state);
    }
}

impl Borrow<u32> for Transaction {
    fn borrow(&self) -> &u32 {
        &self.transaction_id
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Type {
    Disputable(DisputableType),
    Dispute,
    Resolve,
    Chargeback,
}

// TODO: improve errors
pub struct Error;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: improve errors
        write!(f, "Missing amount")
    }
}

impl TryFrom<CsvTransaction> for Transaction {
    // TODO: improve errors
    type Error = Error;
    fn try_from(t: CsvTransaction) -> Result<Self, Self::Error> {
        Ok(Transaction {
            client_id: t.client,
            transaction_id: t.tx,
            type_: match (t.type_, t.amount) {
                (CsvType::Deposit, Some(amount)) => {
                    Type::Disputable(DisputableType::Deposit(amount))
                }
                (CsvType::Deposit, None) => return Err(Error),

                (CsvType::Withdrawal, Some(amount)) => {
                    Type::Disputable(DisputableType::Withdrawal(amount))
                }
                (CsvType::Withdrawal, None) => return Err(Error),

                (CsvType::Dispute, _) => Type::Dispute,
                (CsvType::Resolve, _) => Type::Resolve,
                (CsvType::Chargeback, _) => Type::Chargeback,
            },
        })
    }
}

pub fn read_from_csv_file<P: AsRef<Path>>(
    path: P,
) -> io::Result<impl Iterator<Item = csv::Result<Transaction>>> {
    Ok(ReaderBuilder::new()
        .trim(Trim::All)
        .from_path(path)?
        .into_deserialize())
}

#[allow(dead_code)]
pub fn read_from_csv_reader<R: io::Read>(rdr: R) -> impl Iterator<Item = csv::Result<Transaction>> {
    ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(rdr)
        .into_deserialize()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::decimal::Decimal;
    use csv::{ReaderBuilder, Trim};

    #[test]
    fn deserialize_basic_csv_transaction() {
        let data = "\
type,       client,  tx, amount
deposit,         1,   1,    1.0
chargeback,         10,   21,
";

        let mut rdr = ReaderBuilder::new()
            .trim(Trim::All)
            .from_reader(data.as_bytes());
        {
            let result = rdr.deserialize().next().unwrap();
            let record: CsvTransaction = result.unwrap();
            assert_eq!(record.type_, CsvType::Deposit);
            assert_eq!(record.client, 1);
            assert_eq!(record.tx, 1);
            assert_eq!(record.amount, Some(Decimal::new(1, 0)));
        }

        {
            let result = rdr.deserialize().next().unwrap();
            let record: CsvTransaction = result.unwrap();
            assert_eq!(record.type_, CsvType::Chargeback);
            assert_eq!(record.client, 10);
            assert_eq!(record.tx, 21);
            assert_eq!(record.amount, None);
        }
    }
}
