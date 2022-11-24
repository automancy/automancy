use crate::util::pool::Pool;

use super::id::{Id, RawId, AUTOMANCY_NAMESPACE, NONE_NAME};

pub type Ns = Pool<String, String>;
pub type IdPool = Pool<String, (String, Ns)>;

impl IdPool {
    pub fn new() -> Self {
        let automancy = Ns::from_iter(
            [NONE_NAME]
                .into_iter()
                .map(|e| (e.to_owned(), e.to_owned())),
        );

        Self::from_iter(
            [(AUTOMANCY_NAMESPACE, automancy)]
                .into_iter()
                .map(|(k, v)| (k.to_owned(), (k.to_owned(), v))),
        )
    }

    pub fn raw_id(&self, id: Id) -> RawId {
        let (namespace, ns) = self.to_value(id.namespace);
        let name = ns.to_value(id.name);

        RawId {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
        }
    }

    pub fn id_mut(&mut self, raw_id: &RawId) -> Id {
        let namespace = self.index(&raw_id.namespace);
        let ns = &mut self.to_value_mut(namespace).1;

        let name = ns.index(&raw_id.name);

        Id { namespace, name }
    }

    pub fn id(&self, raw_id: &RawId) -> Option<Id> {
        let namespace = self.get(&raw_id.namespace)?;
        let ns = &self.to_value(namespace).1;

        let name = ns.get(&raw_id.name)?;

        Some(Id { namespace, name })
    }
}
