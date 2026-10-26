//! Single cached-engine ownership boundary. A producer lease includes the exact
//! load instance, so returning a predecessor cannot resurrect it after an unload
//! or another load of the same root/model/content identity.
use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineMarker<K> {
    key: K,
    epoch: u64,
}
pub struct LoadTicket<K>(EngineMarker<K>);
pub struct EngineLease<K, E> {
    marker: EngineMarker<K>,
    engine: E,
}
impl<K, E> EngineLease<K, E> {
    pub fn marker(&self) -> &EngineMarker<K> {
        &self.marker
    }
}
impl<K, E> Deref for EngineLease<K, E> {
    type Target = E;
    fn deref(&self) -> &E {
        &self.engine
    }
}
impl<K, E> DerefMut for EngineLease<K, E> {
    fn deref_mut(&mut self) -> &mut E {
        &mut self.engine
    }
}

pub struct EngineState<K, E> {
    epoch: u64,
    current: Option<EngineMarker<K>>,
    pending: Option<EngineMarker<K>>,
    cached: Option<EngineLease<K, E>>,
}
impl<K: Clone + Eq, E> Default for EngineState<K, E> {
    fn default() -> Self {
        Self {
            epoch: 0,
            current: None,
            pending: None,
            cached: None,
        }
    }
}
impl<K: Clone + Eq, E> EngineState<K, E> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn begin_load(&mut self, key: K) -> LoadTicket<K> {
        self.clear();
        let marker = EngineMarker {
            key,
            epoch: self.epoch,
        };
        self.pending = Some(marker.clone());
        LoadTicket(marker)
    }
    pub fn publish(&mut self, ticket: LoadTicket<K>, engine: E) -> bool {
        if self.pending.as_ref() != Some(&ticket.0) {
            return false;
        }
        self.pending = None;
        self.current = Some(ticket.0.clone());
        self.cached = Some(EngineLease {
            marker: ticket.0,
            engine,
        });
        true
    }
    pub fn current_key(&self) -> Option<&K> {
        self.current.as_ref().map(|m| &m.key)
    }
    pub fn take(&mut self) -> Option<EngineLease<K, E>> {
        self.cached.take()
    }
    pub fn as_ref(&self) -> Option<&E> {
        self.cached.as_ref().map(|lease| &lease.engine)
    }
    pub fn is_some(&self) -> bool {
        self.cached.is_some()
    }
    pub fn is_none(&self) -> bool {
        self.cached.is_none()
    }
    pub fn return_engine(&mut self, engine: EngineLease<K, E>) -> bool {
        if self.current.as_ref() != Some(&engine.marker)
            || self.pending.is_some()
            || self.cached.is_some()
        {
            return false;
        }
        self.cached = Some(engine);
        true
    }
    pub fn clear(&mut self) {
        self.epoch = self.epoch.checked_add(1).expect("engine-epoch-exhausted");
        self.cached = None;
        self.current = None;
        self.pending = None;
    }
    pub fn clear_if(&mut self, marker: &EngineMarker<K>) -> bool {
        if self.current.as_ref() != Some(marker) {
            return false;
        }
        self.clear();
        true
    }
}
