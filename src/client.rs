use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::mem::replace;

use crate::decimal::Decimal;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(into = "ClientOutput")]
pub struct Client {
    id: u16,
    available: Decimal,
    held: Decimal,
    // An amount we're trying to hold, but was withdrawn before we got the chance
    // If any ever comes back, we will hold it.
    held_reserve: Decimal,
    // A tenative amount from disputed withdrawal
    reserve: Decimal,
    locked: bool,
}

impl Hash for Client {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl Client {
    pub fn new(id: u16) -> Self {
        Client {
            id,
            available: Decimal::zero(),
            held: Decimal::zero(),
            held_reserve: Decimal::zero(),
            reserve: Decimal::zero(),
            locked: false,
        }
    }

    pub fn deposit(&mut self, amount: Decimal) {
        match self.held_reserve.clone() - amount.clone() {
            Ok(v) => {
                self.held_reserve = v;
                self.held += amount;
            }
            Err(v) => {
                self.held += replace(&mut self.held_reserve, Decimal::zero());
                self.available += v;
            }
        }
    }

    pub fn withdraw(&mut self, amount: Decimal) -> Result<(), Option<Decimal>> {
        match (self.locked, self.available.clone() - amount) {
            (true, _) => Err(None),
            (false, Ok(v)) => Ok({
                self.available = v;
            }),
            (false, Err(_)) => Err(Some(self.available.clone())),
        }
    }

    pub fn dispute_deposit(&mut self, amount: Decimal) {
        match self.available.clone() - amount.clone() {
            Ok(v) => {
                self.held += amount;
                self.available = v;
            }
            Err(v) => {
                self.held += replace(&mut self.available, Decimal::zero());
                self.held_reserve += v;
            }
        }
    }

    pub fn dispute_withdrawal(&mut self, amount: Decimal) {
        self.reserve += amount;
    }

    pub fn resolve_deposit(&mut self, amount: Decimal) -> Result<(), Decimal> {
        match self.held_reserve.clone() - amount {
            // Relieved some of the reserve burden
            Ok(v) => Ok({
                self.held_reserve = v;
            }),
            // No reserve burden remains, apply to actual held
            Err(amount) => match self.held.clone() - amount.clone() {
                // Some held relieved
                Ok(new_held) => Ok({
                    self.held_reserve = Decimal::zero();
                    self.held = new_held;
                    self.available += amount;
                }),
                // You can't resolve more than is being held
                Err(_) => Err(self.held.clone() + self.held_reserve.clone()),
            },
        }
    }
    pub fn resolve_withdrawal(&mut self, amount: Decimal) -> Result<(), Decimal> {
        match self.reserve.clone() - amount {
            Ok(new_reserve) => Ok({
                self.reserve = new_reserve;
            }),
            Err(_) => Err(self.reserve.clone()),
        }
    }

    pub fn chargeback_deposit(&mut self, amount: Decimal) -> Result<(), Decimal> {
        self.locked = true;
        match self.held_reserve.clone() - amount {
            // Relieved some of the reserve burden
            Ok(v) => Ok({
                self.held_reserve = v;
            }),
            // No reserve burden remains, apply to actual held
            Err(amount) => match self.held.clone() - amount {
                // Some held relieved
                Ok(v) => Ok({
                    self.held_reserve = Decimal::zero();
                    self.held = v;
                }),
                // You can't chargeback more than is being held
                Err(_) => Err(self.held.clone() + self.held_reserve.clone()),
            },
        }
    }

    pub fn chargeback_withdrawal(&mut self, amount: Decimal) -> Result<(), Decimal> {
        self.locked = true;
        match self.reserve.clone() - amount.clone() {
            Ok(new_reserve) => Ok({
                self.reserve = new_reserve;
                self.available += amount;
            }),
            Err(_) => Err(self.reserve.clone()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ClientOutput {
    client: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

impl From<Client> for ClientOutput {
    fn from(c: Client) -> Self {
        ClientOutput {
            client: c.id,
            total: c.available.clone() + c.held.clone() + c.held_reserve.clone(),
            available: c.available,
            held: c.held + c.held_reserve,
            locked: c.locked,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::decimal::Decimal;

    #[test]
    fn test_freeze() {
        let mut c = Client::new(100);
        c.chargeback_deposit(Decimal::zero());

        assert_eq!(c.withdraw(Decimal::zero()), Err(None));
        c.deposit(Decimal::new(10, 1));
        assert_eq!(c.withdraw(Decimal::zero()), Err(None));
        assert_eq!(c.withdraw(Decimal::new(1, 1)), Err(None));

        let mut c = Client::new(100);
        c.deposit(Decimal::new(10, 1));

        assert_eq!(c.withdraw(Decimal::zero()), Err(None));
    }
}
