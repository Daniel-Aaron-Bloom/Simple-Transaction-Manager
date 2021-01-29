use crate::decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;
use csv::{ReaderBuilder, Trim};

//! Sadly csv does not support tags :(

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Transaction {
    Deposit{
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Withdrawal{
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Dispute {
        client: u16,
        tx: u32,
    },
    Resolve {
        client: u16,
        tx: u32,
    },
    Chargeback {
        client: u16,
        tx: u32,
    },
}

pub fn read_from_csv<P: AsRef<Path>>(path: P) -> io::Result<impl Iterator<Item=Transaction>> {
    Ok(ReaderBuilder::new().trim(Trim::All).from_path(path)?.into_deserialize().filter_map(Result::ok))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::decimal::Decimal;
    use csv::{ReaderBuilder, Trim};

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase", tag = "type")]
    pub enum Transaction2 {
        Deposit { name: i32 },
    }
    
    #[test]
    fn mango() {
        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.serialize(Transaction::Deposit{client: 1, tx: 1, amount: Decimal::new(1, 0)}).unwrap();
        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        let mut rdr = ReaderBuilder::new().trim(Trim::All).from_reader(data.as_bytes());
        {
            let result = rdr.deserialize().next().unwrap();
            let record: Transaction = result.unwrap();
            assert_eq!(record, Transaction::Deposit{client: 1, tx: 1, amount: Decimal::new(1, 0)});
        }
        assert_eq!(data, "\
type,client,tx,amount
deposit,1,1,1.0000
");
    }

    #[test]
    fn deserialize_basic_csv_transaction() {
        let data = "\
type,       client,  tx, amount
deposit,         1,   1,    1.0
chargeback,         10,   21,
";

        let mut rdr = ReaderBuilder::new().trim(Trim::All).from_reader(data.as_bytes());
        {
            let result = rdr.deserialize().next().unwrap();
            let record: Transaction = result.unwrap();
            assert_eq!(record, Transaction::Deposit{client: 1, tx: 1, amount: Decimal::new(1, 0)});
        }

        {
            let result = rdr.deserialize().next().unwrap();
            let record: Transaction = result.unwrap();
            assert_eq!(record, Transaction::Chargeback{client: 10, tx: 21});
        }
    }
}
