//! Immutable, browser-free Locator values with caller-owned storage admission.

pub mod semantic;

use std::sync::{Arc, Mutex};

use pwright_backend_cdp_selector_runtime::{
    MAX_SELECTOR_DEPTH, MAX_SELECTOR_NODES, MAX_SELECTOR_STRING_BYTES, measure_selector,
};
use pwright_cap_locator::{RegexMatcher, Selector};

use crate::HostError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueryLimits {
    pub recipes: usize,
    pub bytes: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QueryMetrics {
    pub recipes: usize,
    pub bytes: usize,
}

struct BudgetState {
    limits: QueryLimits,
    usage: QueryMetrics,
}

#[derive(Clone)]
pub struct QueryBudget(Arc<Mutex<BudgetState>>);

impl QueryBudget {
    pub fn new(limits: QueryLimits) -> Result<Self, HostError> {
        if limits.recipes == 0 || limits.bytes == 0 {
            return Err(HostError::InvalidArgument(
                "Locator query limits must be nonzero".into(),
            ));
        }
        Ok(Self(Arc::new(Mutex::new(BudgetState {
            limits,
            usage: QueryMetrics::default(),
        }))))
    }

    pub fn metrics(&self) -> Result<QueryMetrics, HostError> {
        Ok(self.0.lock().map_err(HostError::backend)?.usage)
    }

    /// Call before converting borrowed input to an owned Rust string. The
    /// caller declares aggregate string capacity, including regex flags/names.
    pub fn prepare_leaf(&self, string_bytes: usize) -> Result<QueryDraft, HostError> {
        self.prepare(Shape {
            depth: 1,
            nodes: 1,
            strings: string_bytes,
        })
    }

    fn prepare(&self, shape: Shape) -> Result<QueryDraft, HostError> {
        shape.validate()?;
        let bytes = shape.storage_bytes()?;
        let mut state = self.0.lock().map_err(HostError::backend)?;
        if state.usage.recipes >= state.limits.recipes
            || bytes > state.limits.bytes.saturating_sub(state.usage.bytes)
        {
            return Err(exhausted("Locator recipe count or byte capacity exceeded"));
        }
        state.usage.recipes += 1;
        state.usage.bytes += bytes;
        Ok(QueryDraft {
            shape,
            lease: QueryLease {
                budget: self.clone(),
                bytes,
            },
        })
    }
}

fn exhausted(message: &str) -> HostError {
    HostError::ResourceExhausted(message.to_owned())
}

#[derive(Clone, Copy)]
struct Shape {
    depth: usize,
    nodes: usize,
    strings: usize,
}

impl Shape {
    fn validate(self) -> Result<(), HostError> {
        if self.depth > MAX_SELECTOR_DEPTH
            || self.nodes > MAX_SELECTOR_NODES
            || self.strings > MAX_SELECTOR_STRING_BYTES
        {
            return Err(exhausted(
                "Locator recipe exceeds selector depth, node or string limit",
            ));
        }
        Ok(())
    }

    fn storage_bytes(self) -> Result<usize, HostError> {
        self.nodes
            .checked_mul(std::mem::size_of::<Selector>())
            // Includes one temporary conversion/copy overlap, not only the
            // final string. N-API's UTF-8 adapter owns a terminated buffer
            // while making the final String; fixed metadata covers its NUL.
            .and_then(|bytes| bytes.checked_add(self.strings.checked_mul(2)?))
            .and_then(|bytes| {
                bytes.checked_add(
                    std::mem::size_of::<QueryInner>() + 2 * std::mem::size_of::<usize>(),
                )
            })
            .ok_or_else(|| exhausted("Locator recipe storage calculation overflowed"))
    }

    fn unary(self, strings: usize) -> Result<Self, HostError> {
        self.binary(Self {
            depth: 0,
            nodes: 0,
            strings,
        })
    }

    fn binary(self, other: Self) -> Result<Self, HostError> {
        let shape = Self {
            depth: self.depth.max(other.depth).saturating_add(1),
            nodes: self.nodes.saturating_add(other.nodes).saturating_add(1),
            strings: self.strings.saturating_add(other.strings),
        };
        shape.validate()?;
        Ok(shape)
    }
}

pub(crate) struct QueryLease {
    budget: QueryBudget,
    bytes: usize,
}

impl Drop for QueryLease {
    fn drop(&mut self) {
        let mut state = self
            .budget
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.usage.recipes -= 1;
        state.usage.bytes -= self.bytes;
    }
}

/// Validation discovered while constructing a lazy value, surfaced on use.
/// These fixed variants retain no unbounded error message or selector copy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryIssue {
    EmptyCss,
    InvalidRole,
    InvalidRoleOptions,
}

impl QueryIssue {
    fn error(self) -> HostError {
        match self {
            Self::EmptyCss => HostError::Backend("Locator CSS selector must not be empty".into()),
            Self::InvalidRole => HostError::Backend("unknown ARIA role".into()),
            Self::InvalidRoleOptions => {
                HostError::Backend("ARIA role does not support the requested filters".into())
            }
        }
    }
}

#[derive(Clone)]
enum Plan {
    Selector(Selector),
    Issue(QueryIssue),
}

struct QueryInner {
    shape: Shape,
    plan: Plan,
    lease: QueryLease,
}

/// Preparing a value reserves capacity before owned strings or trees exist.
/// Dropping an unfinished draft releases that capacity.
pub struct QueryDraft {
    shape: Shape,
    lease: QueryLease,
}

impl QueryDraft {
    pub fn css(self, source: String) -> Result<LocatorQuery, HostError> {
        if source.capacity() > self.shape.strings {
            return Err(exhausted("Locator input capacity exceeds its reservation"));
        }
        let plan = match Selector::css(source) {
            Ok(selector) => Plan::Selector(selector),
            Err(_) => Plan::Issue(QueryIssue::EmptyCss),
        };
        self.finish(plan)
    }

    fn finish(self, plan: Plan) -> Result<LocatorQuery, HostError> {
        if let Plan::Selector(selector) = &plan {
            let storage = measure_selector(selector).map_err(|error| exhausted(error.message()))?;
            if storage.string_capacity() > self.shape.strings {
                return Err(exhausted("Locator string capacity exceeds its reservation"));
            }
        }
        Ok(LocatorQuery(Arc::new(QueryInner {
            shape: self.shape,
            plan,
            lease: self.lease,
        })))
    }
}

#[derive(Clone)]
pub struct LocatorQuery(Arc<QueryInner>);

#[derive(Clone, Copy)]
pub enum QueryCombination {
    And,
    Or,
    Within,
    Has,
    HasNot,
}

impl LocatorQuery {
    pub fn copy(&self) -> Result<Self, HostError> {
        let draft = self.0.lease.budget.prepare(self.0.shape)?;
        draft.finish(self.0.plan.clone())
    }

    pub fn deferred_issue(&self) -> Option<QueryIssue> {
        match self.0.plan {
            Plan::Issue(issue) => Some(issue),
            Plan::Selector(_) => None,
        }
    }

    pub(crate) fn command_selector(&self) -> Result<(Selector, QueryLease), HostError> {
        match &self.0.plan {
            Plan::Issue(issue) => Err(issue.error()),
            Plan::Selector(selector) => {
                let draft = self.0.lease.budget.prepare(self.0.shape)?;
                Ok((selector.clone(), draft.lease))
            }
        }
    }

    pub fn nth(&self, index: i32) -> Result<Self, HostError> {
        self.unary(0, |selector| selector.nth(index))
    }

    pub fn visible(&self, visible: bool) -> Result<Self, HostError> {
        self.unary(0, |selector| selector.filter_visible(visible))
    }

    pub fn filter_text(&self, text: &str, exclude: bool) -> Result<Self, HostError> {
        // Playwright ignores empty string hasText/hasNotText filters. An empty
        // RegExp remains meaningful and uses the separate regex path.
        if text.is_empty() {
            return Ok(self.clone());
        }
        self.unary(text.len(), |selector| {
            if exclude {
                selector.filter_not_text(text, false)
            } else {
                selector.filter_text(text, false)
            }
        })
    }

    pub fn filter_regex(&self, matcher: &RegexMatcher, exclude: bool) -> Result<Self, HostError> {
        self.unary(matcher.retained_string_capacity(), |selector| {
            if exclude {
                selector.filter_not_text_regex(matcher.clone())
            } else {
                selector.filter_text_regex(matcher.clone())
            }
        })
    }

    fn unary(
        &self,
        strings: usize,
        compose: impl FnOnce(Selector) -> Selector,
    ) -> Result<Self, HostError> {
        let draft = self.0.lease.budget.prepare(self.0.shape.unary(strings)?)?;
        let plan = match &self.0.plan {
            Plan::Selector(selector) => Plan::Selector(compose(selector.clone())),
            Plan::Issue(issue) => Plan::Issue(*issue),
        };
        draft.finish(plan)
    }

    pub fn combine(&self, other: &Self, combination: QueryCombination) -> Result<Self, HostError> {
        if !Arc::ptr_eq(&self.0.lease.budget.0, &other.0.lease.budget.0) {
            return Err(HostError::InvalidArgument(
                "Locator query budgets must share one owner".into(),
            ));
        }
        let draft = self
            .0
            .lease
            .budget
            .prepare(self.0.shape.binary(other.0.shape)?)?;
        let plan = match (&self.0.plan, &other.0.plan) {
            (Plan::Issue(issue), _) | (_, Plan::Issue(issue)) => Plan::Issue(*issue),
            (Plan::Selector(left), Plan::Selector(right)) => Plan::Selector(match combination {
                QueryCombination::And => left.clone().and(right.clone()),
                QueryCombination::Or => left.clone().or(right.clone()),
                QueryCombination::Within => left.clone().within(right.clone()),
                QueryCombination::Has => left.clone().filter_has(right.clone()),
                QueryCombination::HasNot => left.clone().filter_has_not(right.clone()),
            }),
        };
        draft.finish(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> QueryBudget {
        QueryBudget::new(QueryLimits {
            recipes: 1024,
            bytes: 8 * 1024 * 1024,
        })
        .unwrap()
    }

    fn css(budget: &QueryBudget, value: &str) -> LocatorQuery {
        budget
            .prepare_leaf(value.len())
            .unwrap()
            .css(value.to_owned())
            .unwrap()
    }

    #[test]
    fn abandoned_input_admission_releases_its_count_and_bytes() {
        // Arrange
        let budget = budget();
        let draft = budget.prepare_leaf(1024).unwrap();
        assert_eq!(budget.metrics().unwrap().recipes, 1);
        // Act
        drop(draft);
        // Assert
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn empty_css_is_a_lazy_issue_not_a_fake_valid_selector() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "");
        // Act
        let result = query.command_selector();
        // Assert
        assert_eq!(query.deferred_issue(), Some(QueryIssue::EmptyCss));
        assert!(matches!(result, Err(HostError::Backend(_))));
        assert_eq!(budget.metrics().unwrap().recipes, 1);
    }

    #[test]
    fn input_larger_than_its_reservation_fails_without_retention() {
        // Arrange
        let budget = budget();
        let draft = budget.prepare_leaf(1).unwrap();
        // Act
        let result = draft.css("button".to_owned());
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn full_recipe_capacity_rejects_before_copying_a_second_input() {
        // Arrange
        let budget = QueryBudget::new(QueryLimits {
            recipes: 1,
            bytes: 1024 * 1024,
        })
        .unwrap();
        let _query = css(&budget, "button");
        let before = budget.metrics().unwrap();
        // Act
        let result = budget.prepare_leaf(1);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), before);
    }

    #[test]
    fn byte_capacity_includes_the_string_conversion_overlap() {
        // Arrange
        let shape = Shape {
            depth: 1,
            nodes: 1,
            strings: 100,
        };
        let bytes = shape.storage_bytes().unwrap();
        let budget = QueryBudget::new(QueryLimits { recipes: 10, bytes }).unwrap();
        let _query = css(&budget, &"x".repeat(100));
        // Act
        let result = budget.prepare_leaf(0);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap().bytes, bytes);
        assert!(bytes >= 200 + std::mem::size_of::<QueryInner>());
    }

    #[test]
    fn combination_preserves_both_original_recipes() {
        // Arrange
        let budget = budget();
        let left = css(&budget, "section");
        let right = css(&budget, "button");
        // Act
        let combined = left.combine(&right, QueryCombination::Within).unwrap();
        // Assert
        assert!(
            matches!(&left.0.plan, Plan::Selector(Selector::Css(value)) if value.as_str() == "section")
        );
        assert!(
            matches!(&right.0.plan, Plan::Selector(Selector::Css(value)) if value.as_str() == "button")
        );
        assert!(matches!(
            &combined.0.plan,
            Plan::Selector(Selector::Within { .. })
        ));
        assert_eq!(budget.metrics().unwrap().recipes, 3);
    }

    #[test]
    fn depth_overflow_preserves_the_last_admitted_recipe() {
        // Arrange
        let budget = budget();
        let mut query = css(&budget, "button");
        for _ in 1..MAX_SELECTOR_DEPTH {
            query = query.nth(0).unwrap();
        }
        let before = budget.metrics().unwrap();
        // Act
        let result = query.nth(0);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), before);
        assert_eq!(before.recipes, 1);
    }

    #[test]
    fn node_overflow_rejects_before_cloning_an_exponential_recipe() {
        // Arrange
        let budget = budget();
        let mut query = css(&budget, "x");
        for _ in 0..7 {
            query = query.combine(&query, QueryCombination::Or).unwrap();
        }
        let before = budget.metrics().unwrap();
        // Act
        let result = query.combine(&query, QueryCombination::Or);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(query.0.shape.nodes, 255);
        assert_eq!(budget.metrics().unwrap(), before);
    }

    #[test]
    fn commands_admit_and_release_their_own_selector_copy() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "button");
        let before = budget.metrics().unwrap();
        // Act
        let copy = query.command_selector().unwrap();
        let during = budget.metrics().unwrap();
        drop(copy);
        // Assert
        assert_eq!(during.recipes, 2);
        assert_eq!(during.bytes, before.bytes * 2);
        assert_eq!(budget.metrics().unwrap(), before);
    }

    #[test]
    fn cross_owner_combinations_do_not_consume_either_budget() {
        // Arrange
        let first = budget();
        let second = budget();
        let left = css(&first, "a");
        let right = css(&second, "b");
        let before = (first.metrics().unwrap(), second.metrics().unwrap());
        // Act
        let result = left.combine(&right, QueryCombination::And);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
        assert_eq!(
            (first.metrics().unwrap(), second.metrics().unwrap()),
            before
        );
    }

    #[test]
    fn empty_has_not_text_preserves_the_recipe() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "button");
        // Act
        let filtered = query.filter_text("", true).unwrap();
        // Assert
        assert!(Arc::ptr_eq(&query.0, &filtered.0));
        assert_eq!(budget.metrics().unwrap().recipes, 1);
    }

    #[test]
    fn copying_a_recipe_has_independent_storage_admission() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "button");
        // Act
        let copy = query.copy().unwrap();
        // Assert
        assert!(!Arc::ptr_eq(&query.0, &copy.0));
        assert_eq!(budget.metrics().unwrap().recipes, 2);
    }

    #[test]
    fn concurrent_creators_cannot_exceed_the_shared_owner_budget() {
        // Arrange
        let budget = QueryBudget::new(QueryLimits {
            recipes: 32,
            bytes: 1024 * 1024,
        })
        .unwrap();
        let barrier = std::sync::Barrier::new(5);
        // Act
        let (live, admitted) = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        let queries: Vec<_> = (0..16)
                            .filter_map(|_| {
                                budget
                                    .prepare_leaf(1)
                                    .ok()
                                    .map(|draft| draft.css("x".to_owned()).unwrap())
                            })
                            .collect();
                        barrier.wait();
                        barrier.wait();
                        queries.len()
                    })
                })
                .collect();
            barrier.wait();
            let live = budget.metrics().unwrap();
            barrier.wait();
            (
                live,
                workers
                    .into_iter()
                    .map(|worker| worker.join().unwrap())
                    .sum::<usize>(),
            )
        });
        // Assert
        assert_eq!(live.recipes, 32);
        assert_eq!(admitted, 32);
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }
}
