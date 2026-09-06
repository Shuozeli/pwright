use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::ObjectId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectRecord<K> {
    id: ObjectId,
    parent: Option<ObjectId>,
    kind: K,
}

impl<K> ObjectRecord<K> {
    pub const fn id(&self) -> ObjectId {
        self.id
    }

    pub const fn parent(&self) -> Option<ObjectId> {
        self.parent
    }

    pub const fn kind(&self) -> &K {
        &self.kind
    }

    pub fn into_kind(self) -> K {
        self.kind
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectGraphError {
    DuplicateObject {
        object_id: ObjectId,
    },
    MissingObject {
        object_id: ObjectId,
    },
    MissingParent {
        object_id: ObjectId,
        parent: ObjectId,
    },
    AdoptionCycle {
        object_id: ObjectId,
        requested_parent: ObjectId,
    },
    CapacityExceeded {
        limit: usize,
    },
}

impl Display for ObjectGraphError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateObject { object_id } => {
                write!(formatter, "object `{object_id}` already exists")
            }
            Self::MissingObject { object_id } => {
                write!(formatter, "object `{object_id}` does not exist")
            }
            Self::MissingParent { object_id, parent } => write!(
                formatter,
                "object `{object_id}` cannot use missing parent `{parent}`"
            ),
            Self::AdoptionCycle {
                object_id,
                requested_parent,
            } => write!(
                formatter,
                "adopting object `{object_id}` under `{requested_parent}` would create a cycle"
            ),
            Self::CapacityExceeded { limit } => {
                write!(formatter, "live object limit of {limit} was reached")
            }
        }
    }
}

impl Error for ObjectGraphError {}

/// A bounded ownership graph for runtime protocol objects.
///
/// The graph contains only live records. Disposing an unknown ID is an
/// idempotent no-op, while normal calls should use `get` to reject stale IDs.
#[derive(Clone, Debug)]
pub struct ObjectGraph<K> {
    limit: usize,
    peak_len: usize,
    objects: BTreeMap<ObjectId, ObjectRecord<K>>,
    children: BTreeMap<ObjectId, BTreeSet<ObjectId>>,
}

impl<K> ObjectGraph<K> {
    pub const fn new(limit: usize) -> Self {
        Self {
            limit,
            peak_len: 0,
            objects: BTreeMap::new(),
            children: BTreeMap::new(),
        }
    }

    pub const fn limit(&self) -> usize {
        self.limit
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    pub const fn peak_len(&self) -> usize {
        self.peak_len
    }

    pub fn get(&self, object_id: ObjectId) -> Option<&ObjectRecord<K>> {
        self.objects.get(&object_id)
    }

    pub fn children_of(&self, object_id: ObjectId) -> impl Iterator<Item = ObjectId> + '_ {
        self.children
            .get(&object_id)
            .into_iter()
            .flat_map(|children| children.iter().copied())
    }

    /// Returns the live subtree in deterministic child-first order without
    /// changing the graph.
    pub fn subtree_ids_child_first(&self, object_id: ObjectId) -> Vec<ObjectId> {
        if !self.objects.contains_key(&object_id) {
            return Vec::new();
        }
        self.child_first_order(object_id)
    }

    pub fn insert(
        &mut self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: K,
    ) -> Result<(), ObjectGraphError> {
        self.validate_insert(object_id, parent)?;

        self.objects.insert(
            object_id,
            ObjectRecord {
                id: object_id,
                parent,
                kind,
            },
        );
        self.children.entry(object_id).or_default();
        if let Some(parent_id) = parent {
            self.children
                .entry(parent_id)
                .or_default()
                .insert(object_id);
        }
        self.peak_len = self.peak_len.max(self.objects.len());
        Ok(())
    }

    /// Checks insertion invariants without changing the graph.
    pub fn validate_insert(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
    ) -> Result<(), ObjectGraphError> {
        if self.objects.contains_key(&object_id) {
            return Err(ObjectGraphError::DuplicateObject { object_id });
        }
        if self.objects.len() >= self.limit {
            return Err(ObjectGraphError::CapacityExceeded { limit: self.limit });
        }
        if let Some(parent_id) = parent
            && !self.objects.contains_key(&parent_id)
        {
            return Err(ObjectGraphError::MissingParent {
                object_id,
                parent: parent_id,
            });
        }
        Ok(())
    }

    pub fn adopt(
        &mut self,
        object_id: ObjectId,
        new_parent: ObjectId,
    ) -> Result<(), ObjectGraphError> {
        self.validate_adopt(object_id, new_parent)?;
        let previous_parent = self
            .objects
            .get(&object_id)
            .expect("adoption validation requires a live object")
            .parent;
        if previous_parent == Some(new_parent) {
            return Ok(());
        }
        if let Some(previous_parent) = previous_parent
            && let Some(children) = self.children.get_mut(&previous_parent)
        {
            children.remove(&object_id);
        }
        self.children
            .entry(new_parent)
            .or_default()
            .insert(object_id);
        self.objects
            .get_mut(&object_id)
            .expect("object existence was checked")
            .parent = Some(new_parent);
        Ok(())
    }

    /// Validates an ownership move without changing the graph.
    pub fn validate_adopt(
        &self,
        object_id: ObjectId,
        new_parent: ObjectId,
    ) -> Result<(), ObjectGraphError> {
        if !self.objects.contains_key(&object_id) {
            return Err(ObjectGraphError::MissingObject { object_id });
        }
        if !self.objects.contains_key(&new_parent) {
            return Err(ObjectGraphError::MissingParent {
                object_id,
                parent: new_parent,
            });
        }

        let mut ancestor = Some(new_parent);
        while let Some(candidate) = ancestor {
            if candidate == object_id {
                return Err(ObjectGraphError::AdoptionCycle {
                    object_id,
                    requested_parent: new_parent,
                });
            }
            ancestor = self.objects.get(&candidate).and_then(ObjectRecord::parent);
        }

        Ok(())
    }

    /// Removes one object and all descendants in child-first cleanup order.
    pub fn dispose_subtree(&mut self, object_id: ObjectId) -> Vec<ObjectRecord<K>> {
        if !self.objects.contains_key(&object_id) {
            return Vec::new();
        }

        let order = self.child_first_order(object_id);
        let mut removed = Vec::with_capacity(order.len());
        for current in order {
            let Some(record) = self.objects.remove(&current) else {
                continue;
            };
            if let Some(parent) = record.parent
                && let Some(children) = self.children.get_mut(&parent)
            {
                children.remove(&current);
            }
            self.children.remove(&current);
            removed.push(record);
        }
        removed
    }

    /// Removes every object in deterministic child-first order.
    ///
    /// Independent root trees are visited in ascending root-ID order.
    pub fn drain_child_first(&mut self) -> Vec<ObjectRecord<K>> {
        let roots = self
            .objects
            .values()
            .filter(|record| record.parent().is_none())
            .map(ObjectRecord::id)
            .collect::<Vec<_>>();
        let mut removed = Vec::with_capacity(self.objects.len());
        for root in roots {
            removed.extend(self.dispose_subtree(root));
        }
        removed
    }

    fn child_first_order(&self, object_id: ObjectId) -> Vec<ObjectId> {
        let mut order = Vec::new();
        let mut stack = vec![(object_id, false)];
        while let Some((current, expanded)) = stack.pop() {
            if expanded {
                order.push(current);
                continue;
            }
            stack.push((current, true));
            if let Some(children) = self.children.get(&current) {
                for child in children.iter().rev() {
                    stack.push((*child, false));
                }
            }
        }
        order
    }
}

#[cfg(test)]
mod tests {
    use crate::ObjectId;

    use super::{ObjectGraph, ObjectGraphError, ObjectRecord};

    #[test]
    fn insert_rejects_a_child_whose_parent_is_not_live() {
        // Arrange
        let mut graph = ObjectGraph::new(2);

        // Act
        let error = graph
            .insert(ObjectId::new(2), Some(ObjectId::new(1)), "page")
            .expect_err("missing parent should fail");

        // Assert
        assert_eq!(
            error,
            ObjectGraphError::MissingParent {
                object_id: ObjectId::new(2),
                parent: ObjectId::new(1),
            }
        );
        assert!(graph.is_empty());
    }

    #[test]
    fn insert_enforces_the_live_object_limit() {
        // Arrange
        let mut graph = ObjectGraph::new(1);
        graph
            .insert(ObjectId::new(1), None, "browser")
            .expect("root should fit");

        // Act
        let error = graph
            .insert(ObjectId::new(2), Some(ObjectId::new(1)), "page")
            .expect_err("second object should exceed the limit");

        // Assert
        assert_eq!(error, ObjectGraphError::CapacityExceeded { limit: 1 });
        assert_eq!(graph.len(), 1);
        assert_eq!(graph.peak_len(), 1);
    }

    #[test]
    fn insert_rejects_a_duplicate_object_id_without_replacing_the_record() {
        // Arrange
        let mut graph = ObjectGraph::new(2);
        graph
            .insert(ObjectId::new(1), None, "browser")
            .expect("first object should register");

        // Act
        let error = graph
            .insert(ObjectId::new(1), None, "replacement")
            .expect_err("duplicate object should fail");

        // Assert
        assert_eq!(
            error,
            ObjectGraphError::DuplicateObject {
                object_id: ObjectId::new(1),
            }
        );
        assert_eq!(
            graph.get(ObjectId::new(1)).map(|record| record.kind()),
            Some(&"browser")
        );
    }

    #[test]
    fn adopt_moves_an_object_to_a_new_live_parent() {
        // Arrange
        let mut graph = ObjectGraph::new(3);
        graph
            .insert(ObjectId::new(1), None, "browser-a")
            .expect("first root should register");
        graph
            .insert(ObjectId::new(2), None, "browser-b")
            .expect("second root should register");
        graph
            .insert(ObjectId::new(3), Some(ObjectId::new(1)), "page")
            .expect("page should register");

        // Act
        graph
            .adopt(ObjectId::new(3), ObjectId::new(2))
            .expect("adoption should succeed");

        // Assert
        assert_eq!(
            graph
                .get(ObjectId::new(3))
                .and_then(|record| record.parent()),
            Some(ObjectId::new(2))
        );
        assert_eq!(graph.children_of(ObjectId::new(1)).count(), 0);
        assert_eq!(
            graph.children_of(ObjectId::new(2)).collect::<Vec<_>>(),
            vec![ObjectId::new(3)]
        );
    }

    #[test]
    fn adopt_rejects_a_cycle() {
        // Arrange
        let mut graph = ObjectGraph::new(3);
        graph
            .insert(ObjectId::new(1), None, "browser")
            .expect("root should register");
        graph
            .insert(ObjectId::new(2), Some(ObjectId::new(1)), "context")
            .expect("context should register");
        graph
            .insert(ObjectId::new(3), Some(ObjectId::new(2)), "page")
            .expect("page should register");

        // Act
        let error = graph
            .adopt(ObjectId::new(1), ObjectId::new(3))
            .expect_err("cycle should fail");

        // Assert
        assert_eq!(
            error,
            ObjectGraphError::AdoptionCycle {
                object_id: ObjectId::new(1),
                requested_parent: ObjectId::new(3),
            }
        );
        assert_eq!(
            graph
                .get(ObjectId::new(1))
                .and_then(|record| record.parent()),
            None
        );
    }

    #[test]
    fn dispose_parent_returns_descendants_in_child_first_order() {
        // Arrange
        let mut graph = ObjectGraph::new(3);
        graph
            .insert(ObjectId::new(1), None, "browser")
            .expect("browser should register");
        graph
            .insert(ObjectId::new(2), Some(ObjectId::new(1)), "context")
            .expect("context should register");
        graph
            .insert(ObjectId::new(3), Some(ObjectId::new(2)), "page")
            .expect("page should register");

        // Act
        let removed = graph.dispose_subtree(ObjectId::new(1));

        // Assert
        let removed_ids: Vec<_> = removed.iter().map(|record| record.id()).collect();
        assert_eq!(
            removed_ids,
            vec![ObjectId::new(3), ObjectId::new(2), ObjectId::new(1)]
        );
        assert!(graph.is_empty());
    }

    #[test]
    fn dispose_of_an_absent_object_is_idempotent() {
        // Arrange
        let mut graph = ObjectGraph::<&str>::new(1);

        // Act
        let removed = graph.dispose_subtree(ObjectId::new(99));

        // Assert
        assert!(removed.is_empty());
        assert!(graph.is_empty());
    }

    #[test]
    fn subtree_preview_matches_disposal_order_without_removing_objects() {
        // Arrange
        let mut graph = ObjectGraph::new(3);
        graph
            .insert(ObjectId::new(1), None, "browser")
            .expect("root should register");
        graph
            .insert(ObjectId::new(2), Some(ObjectId::new(1)), "context")
            .expect("context should register");
        graph
            .insert(ObjectId::new(3), Some(ObjectId::new(2)), "page")
            .expect("page should register");

        // Act
        let preview = graph.subtree_ids_child_first(ObjectId::new(1));

        // Assert
        assert_eq!(
            preview,
            vec![ObjectId::new(3), ObjectId::new(2), ObjectId::new(1)]
        );
        assert_eq!(graph.len(), 3);
    }

    #[test]
    fn drain_returns_all_root_trees_in_deterministic_child_first_order() {
        // Arrange
        let mut graph = ObjectGraph::new(5);
        graph
            .insert(ObjectId::new(10), None, "root-a")
            .expect("first root should insert");
        graph
            .insert(ObjectId::new(12), Some(ObjectId::new(10)), "child-a")
            .expect("first child should insert");
        graph
            .insert(ObjectId::new(20), None, "root-b")
            .expect("second root should insert");
        graph
            .insert(ObjectId::new(21), Some(ObjectId::new(20)), "child-b")
            .expect("second child should insert");

        // Act
        let removed = graph.drain_child_first();

        // Assert
        assert_eq!(
            removed.iter().map(ObjectRecord::id).collect::<Vec<_>>(),
            vec![
                ObjectId::new(12),
                ObjectId::new(10),
                ObjectId::new(21),
                ObjectId::new(20),
            ]
        );
        assert!(graph.is_empty());
    }
}
