use cached::Cached;
use std::collections::HashMap;

use crate::transaction::DisputableTransaction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Committed,
    Resolved,
    Disputed,
    ChargedBack,
    ChargedBackFinal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateFailure {
    NotFound,
    WrongState(State),
}

pub trait Client {
    fn store(&mut self, t: DisputableTransaction);
    fn access(&mut self, id: u32) -> Option<(DisputableTransaction, State)>;
    //
    fn update(&mut self, id: u32, state: State) -> Result<DisputableTransaction, UpdateFailure>;
}

#[derive(Default)]
pub struct MemoryClient(HashMap<u32, (DisputableTransaction, State)>);

impl Client for MemoryClient {
    fn store(&mut self, t: DisputableTransaction) {
        self.0.insert(t.transaction_id, (t, State::Committed));
    }
    fn access(&mut self, id: u32) -> Option<(DisputableTransaction, State)> {
        self.0.get_key_value(&id).map(|(_, (t, s))| (t.clone(), *s))
    }
    fn update(&mut self, id: u32, state: State) -> Result<DisputableTransaction, UpdateFailure> {
        use State::*;
        match (self.0.get_mut(&id), state) {
            (None, _) => Err(UpdateFailure::NotFound),
            (Some(&mut (ref t, ref mut s @ Resolved)), Committed)
            | (Some(&mut (ref t, ref mut s @ ChargedBack)), ChargedBackFinal) => {
                *s = state;
                Ok(t.clone())
            }

            (Some(&mut (_, s @ ChargedBackFinal)), _)
            | (Some(&mut (_, s)), ChargedBackFinal)
            | (Some(&mut (_, s @ ChargedBack)), _)
            | (Some(&mut (_, s @ Committed)), ChargedBack)
            | (Some(&mut (_, s @ Committed)), Committed)
            | (Some(&mut (_, s @ Resolved)), _) => Err(UpdateFailure::WrongState(s)),
            (Some(&mut (ref t, ref mut s)), state) => {
                *s = state;
                Ok(t.clone())
            }
        }
    }
}

#[derive(Default)]
pub struct CachedClient<Cl: Client, Ca: Cached<u32, (DisputableTransaction, State)>> {
    client: Cl,
    cache: Ca,
}

impl<Cl: Client, Ca: Cached<u32, (DisputableTransaction, State)>> CachedClient<Cl, Ca> {
    pub fn new(client: Cl, cache: Ca) -> Self {
        CachedClient { client, cache }
    }
}

impl<Cl: Client, Ca: Cached<u32, (DisputableTransaction, State)>> Client for CachedClient<Cl, Ca> {
    fn store(&mut self, t: DisputableTransaction) {
        self.client.store(t);
    }

    fn access(&mut self, id: u32) -> Option<(DisputableTransaction, State)> {
        self.cache
            .cache_get(&id)
            .map(|(t, s)| (t.clone(), *s))
            .or_else(|| self.client.access(id))
    }

    fn update(&mut self, id: u32, state: State) -> Result<DisputableTransaction, UpdateFailure> {
        let transaction = self.client.update(id, state)?;
        if let Some(cached) = self.cache.cache_get_mut(&id) {
            cached.1 = state;
            // TODO check that cache matches
        }
        Ok(transaction)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::decimal::Decimal;
    use crate::transaction::{DisputableTransaction, DisputableType};

    #[test]
    fn basic_client_test() {
        let mut client = MemoryClient::default();

        assert_eq!(client.access(0), None);
        client.store(DisputableTransaction {
            client_id: 501,
            transaction_id: 16,
            type_: DisputableType::Withdrawal(Decimal::zero()),
        });
        assert_eq!(client.access(0), None);
        assert_eq!(client.access(16).is_some(), true);
    }
}
