use crate::entity::Entity;
use crate::storage::Column;
use std::any::TypeId;
use std::collections::HashMap;

pub type ArchetypeId = u32;

pub struct Archetype {
    pub id: ArchetypeId,
    pub type_signature: Vec<TypeId>,
    pub columns: HashMap<TypeId, Box<dyn Column>>,
    pub entities: Vec<Entity>,
    pub add_edges: HashMap<TypeId, ArchetypeId>,
    pub remove_edges: HashMap<TypeId, ArchetypeId>,
}

impl Archetype {
    pub fn new(
        id: ArchetypeId,
        signature: Vec<TypeId>,
        columns: HashMap<TypeId, Box<dyn Column>>,
    ) -> Self {
        Self {
            id,
            type_signature: signature,
            columns,
            entities: Vec::new(),
            add_edges: HashMap::new(),
            remove_edges: HashMap::new(),
        }
    }

    pub fn has_component(&self, type_id: TypeId) -> bool {
        self.type_signature.binary_search(&type_id).is_ok()
    }

    pub fn allocate_row(&mut self, entity: Entity) -> usize {
        let row = self.entities.len();
        self.entities.push(entity);
        row
    }

    pub fn free_row(&mut self, row: usize) -> Option<Entity> {
        self.entities.swap_remove(row);
        self.entities.get(row).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Entity;

    #[test]
    fn free_row_displaces_last_entity() {
        let mut arch = Archetype::new(0, vec![], HashMap::new());
        let e0 = Entity {
            index: 0,
            generation: 0,
        };
        let e1 = Entity {
            index: 1,
            generation: 0,
        };
        let e2 = Entity {
            index: 2,
            generation: 0,
        };

        arch.allocate_row(e0);
        arch.allocate_row(e1);
        arch.allocate_row(e2);

        let displaced = arch.free_row(1);
        assert_eq!(displaced, Some(e2));
        assert_eq!(arch.entities, vec![e0, e2]);
    }
}
