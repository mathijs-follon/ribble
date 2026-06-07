use crate::entity::Entity;

pub struct EntityAllocator {
    generations: Vec<u32>,
    free_list: Vec<u32>,
}

impl Default for EntityAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityAllocator {
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> Entity {
        if let Some(index) = self.free_list.pop() {
            // generation was already incremented on free, so its ready
            Entity {
                index,
                generation: self.generations[index as usize],
            }
        } else {
            let index = self.generations.len() as u32;
            self.generations.push(0);
            Entity {
                index,
                generation: 0,
            }
        }
    }
    pub fn free(&mut self, entity: Entity) {
        self.generations[entity.index as usize] += 1;
        self.free_list.push(entity.index);
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations
            .get(entity.index as usize)
            .map(|&generation| generation == entity.generation)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_free_realloc_increments_generation() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.alloc();
        assert_eq!(e1.generation, 0);

        alloc.free(e1);
        assert!(!alloc.is_alive(e1));

        let e2 = alloc.alloc();
        assert_eq!(e1.index, e2.index);
        assert_eq!(e2.generation, 1);
        assert!(alloc.is_alive(e2));
        assert!(!alloc.is_alive(e1));
    }
}
