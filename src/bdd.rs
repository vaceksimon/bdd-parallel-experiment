use crate::{Bdd, BddParallel, Node, NodeId, Variable, WorkerPool};
use dashmap::DashMap;
use std::cmp::min;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::Duration;

impl Variable {
    const TERMINAL_VARIABLE: Variable = Variable(u32::MAX);
    const UNDEFINED_VARIABLE: Variable = Variable(u32::MAX - 1);

    fn is_undefined(&self) -> bool {
        self == &Self::UNDEFINED_VARIABLE
    }
}

impl NodeId {
    const TERMINAL_0: Self = NodeId(0);
    const TERMINAL_1: Self = NodeId(1);

    fn as_usize(self) -> usize {
        self.0
    }

    fn is_terminal(&self) -> bool {
        self == &Self::TERMINAL_0 || self == &Self::TERMINAL_1
    }

    #[cfg(test)]
    fn is_zero(self) -> bool {
        self == Self::TERMINAL_0
    }

    fn is_one(self) -> bool {
        self == Self::TERMINAL_1
    }
}

impl Node {
    pub(crate) fn new(variable: Variable, low_child: NodeId, high_child: NodeId) -> Self {
        Self {
            variable,
            low_child,
            high_child,
        }
    }

    fn one() -> Self {
        Self::new(
            Variable::TERMINAL_VARIABLE,
            NodeId::TERMINAL_1,
            NodeId::TERMINAL_1,
        )
    }

    fn zero() -> Self {
        Self::new(
            Variable::TERMINAL_VARIABLE,
            NodeId::TERMINAL_0,
            NodeId::TERMINAL_0,
        )
    }
}

impl Default for Bdd {
    fn default() -> Self {
        Self::new()
    }
}

impl Bdd {
    pub fn new() -> Self {
        let terminal_0 = Node::zero();
        let terminal_1 = Node::one();

        Bdd {
            nodes: Vec::from([terminal_0, terminal_1]),
            node_table: HashMap::new(),
            task_cache: HashMap::new(),
        }
    }

    pub fn apply_recursive(&mut self, a_id: NodeId, b_id: NodeId) -> (NodeId, Node) {
        if a_id.is_terminal() && b_id.is_terminal() {
            return if a_id.is_one() && b_id.is_one() {
                (
                    NodeId::TERMINAL_1,
                    self.nodes[NodeId::TERMINAL_1.as_usize()],
                )
            } else {
                (
                    NodeId::TERMINAL_0,
                    self.nodes[NodeId::TERMINAL_0.as_usize()],
                )
            };
        }

        if let Some(found_node_id) = self.task_cache.get(&(a_id, b_id)) {
            return (*found_node_id, self.nodes[found_node_id.as_usize()]);
        }

        let a = self.nodes[a_id.as_usize()];
        let b = self.nodes[b_id.as_usize()];
        let v = min(a.variable, b.variable);

        let (low_a, high_a) = if a.variable == v {
            (a.low_child, a.high_child)
        } else {
            (a_id, a_id)
        };

        let (low_b, high_b) = if b.variable == v {
            (b.low_child, b.high_child)
        } else {
            (b_id, b_id)
        };

        let l = self.apply_recursive(low_a, low_b);
        let h = self.apply_recursive(high_a, high_b);

        let (c_node_id, c) = if l != h {
            self.ensure_node(v, l.0, h.0)
        } else {
            l
        };

        self.task_cache.insert((a_id, b_id), c_node_id);
        (c_node_id, c)
    }

    pub fn apply_iterative(&mut self, a_id: NodeId, b_id: NodeId) -> (NodeId, Node) {
        // stack contains tasks that need to be done
        let mut stack: Vec<(NodeId, NodeId, Variable)> =
            vec![(a_id, b_id, Variable::UNDEFINED_VARIABLE)];
        // results contains results of tasks
        let mut results: Vec<(NodeId, Node)> = vec![];

        while let Some((a_id, b_id, variable)) = stack.pop() {
            if a_id.is_terminal() && b_id.is_terminal() {
                if a_id.is_one() && b_id.is_one() {
                    results.push((
                        NodeId::TERMINAL_1,
                        self.nodes[NodeId::TERMINAL_1.as_usize()],
                    ));
                } else {
                    results.push((
                        NodeId::TERMINAL_0,
                        self.nodes[NodeId::TERMINAL_0.as_usize()],
                    ));
                };
                continue;
            }

            if variable.is_undefined() {
                if let Some(found_node_id) = self.task_cache.get(&(a_id, b_id)) {
                    results.push((*found_node_id, self.nodes[found_node_id.as_usize()]));
                    continue;
                }

                let a = self.nodes[a_id.as_usize()];
                let b = self.nodes[b_id.as_usize()];
                let v = min(a.variable, b.variable);

                let (low_a, high_a) = if a.variable == v {
                    (a.low_child, a.high_child)
                } else {
                    (a_id, a_id)
                };

                let (low_b, high_b) = if b.variable == v {
                    (b.low_child, b.high_child)
                } else {
                    (b_id, b_id)
                };

                stack.push((a_id, b_id, v));
                stack.push((high_a, high_b, Variable::UNDEFINED_VARIABLE));
                stack.push((low_a, low_b, Variable::UNDEFINED_VARIABLE));

                continue;
            }

            let h = results.pop().expect("low result present in result stack");
            let l = results.pop().expect("high result present in result stack");

            let (c_node_id, c) = if l != h {
                self.ensure_node(variable, l.0, h.0)
            } else {
                l
            };

            self.task_cache.insert((a_id, b_id), c_node_id);
            results.push((c_node_id, c));
        }

        let (root_id, root) = results.pop().expect("only one result expected");
        assert!(results.is_empty());
        (root_id, root)
    }

    fn ensure_node(
        &mut self,
        variable: Variable,
        low_child: NodeId,
        high_child: NodeId,
    ) -> (NodeId, Node) {
        let needle = Node::new(variable, low_child, high_child);
        if let Some(found) = self.node_table.get(&needle) {
            (*found, needle)
        } else {
            let node_id = NodeId(self.nodes.len());
            self.nodes.push(needle);
            self.node_table.insert(needle, node_id);
            (node_id, needle)
        }
    }

    /// Merge all nodes from `other` into this BDD's storage.
    ///
    /// Child links are rebuilt using this BDD's node IDs. Uniqueness is preserved
    /// through the internal node table, so structurally identical nodes are shared.
    ///
    /// Returns a mapping from node IDs in `other` to their corresponding IDs here.
    pub fn merge(&mut self, other: &Bdd) -> HashMap<NodeId, NodeId> {
        let mut id_map = HashMap::new();
        id_map.insert(NodeId::TERMINAL_0, NodeId::TERMINAL_0);
        id_map.insert(NodeId::TERMINAL_1, NodeId::TERMINAL_1);

        for i in 2..other.nodes.len() {
            Self::merge_node(self, other, NodeId(i), &mut id_map);
        }

        id_map
    }

    fn merge_node(
        &mut self,
        other: &Bdd,
        id: NodeId,
        id_map: &mut HashMap<NodeId, NodeId>,
    ) -> NodeId {
        if let Some(&mapped) = id_map.get(&id) {
            return mapped;
        }

        let node = other.nodes[id.as_usize()];
        let low = Self::merge_node(self, other, node.low_child, id_map);
        let high = Self::merge_node(self, other, node.high_child, id_map);
        let (new_id, _) = self.ensure_node(node.variable, low, high);
        id_map.insert(id, new_id);
        new_id
    }

    /// Extract the subgraph reachable from `root` into a new BDD.
    ///
    /// Node IDs are recomputed for the extracted BDD. Returns the new BDD together
    /// with the remapped ID of `root`.
    pub fn extract(&self, root: NodeId) -> (Bdd, NodeId) {
        let mut extracted = Bdd::new();
        let mut id_map = HashMap::new();
        id_map.insert(NodeId::TERMINAL_0, NodeId::TERMINAL_0);
        id_map.insert(NodeId::TERMINAL_1, NodeId::TERMINAL_1);

        let new_root = Self::extract_node(&mut extracted, self, root, &mut id_map);
        (extracted, new_root)
    }

    fn extract_node(
        extracted: &mut Bdd,
        source: &Bdd,
        id: NodeId,
        id_map: &mut HashMap<NodeId, NodeId>,
    ) -> NodeId {
        if let Some(&mapped) = id_map.get(&id) {
            return mapped;
        }

        let node = source.nodes[id.as_usize()];
        let low = Self::extract_node(extracted, source, node.low_child, id_map);
        let high = Self::extract_node(extracted, source, node.high_child, id_map);
        let (new_id, _) = extracted.ensure_node(node.variable, low, high);
        id_map.insert(id, new_id);
        new_id
    }
}

impl Default for BddParallel {
    fn default() -> Self {
        Self::new()
    }
}

impl BddParallel {
    pub fn new() -> Self {
        let terminal_0 = Node::zero();
        let terminal_1 = Node::one();

        BddParallel {
            nodes: RwLock::new(Vec::from([terminal_0, terminal_1])),
            node_table: DashMap::new(),
            task_cache: DashMap::new(),
        }
    }

    pub fn apply(&mut self, a_id: NodeId, b_id: NodeId) -> (NodeId, Node) {
        let total_workers = 3;
        let pool = Arc::new(WorkerPool {
            stack: Mutex::new(vec![(a_id, b_id, Variable::UNDEFINED_VARIABLE)]),
            results: Mutex::new(Vec::new()),
            is_done: AtomicBool::new(false),
            idle_workers: AtomicUsize::new(0),
            sleep_lock: Mutex::new(()),
            cvar: Condvar::new(),
        });

        thread::scope(|s| {
            for _ in 0..total_workers {
                s.spawn(|| {
                    let pool = pool.clone();
                    loop {
                        if pool.is_done.load(Ordering::Acquire) {
                            break;
                        }

                        let top_of_stack = pool.stack.lock().unwrap().pop(); // to avoid locking for the whole `if let` statement
                        if let Some((a_id, b_id, variable)) = top_of_stack {
                            if a_id.is_terminal() && b_id.is_terminal() {
                                if a_id.is_one() && b_id.is_one() {
                                    pool.results.lock().unwrap().push((
                                        NodeId::TERMINAL_1,
                                        self.nodes.read().unwrap()[NodeId::TERMINAL_1.as_usize()],
                                    ));
                                } else {
                                    pool.results.lock().unwrap().push((
                                        NodeId::TERMINAL_0,
                                        self.nodes.read().unwrap()[NodeId::TERMINAL_0.as_usize()],
                                    ));
                                };
                                continue;
                            }
                            println!(
                                "[{:?}] just POPPED [{:?}, {:?}, {:?}]",
                                thread::current().id(),
                                a_id,
                                b_id,
                                variable
                            );

                            if variable.is_undefined() {
                                if let Some(found_node_id) = self.task_cache.get(&(a_id, b_id)) {
                                    pool.results.lock().unwrap().push((
                                        *found_node_id,
                                        self.nodes.read().unwrap()[found_node_id.as_usize()],
                                    ));
                                    continue;
                                }

                                let a = self.nodes.read().unwrap()[a_id.as_usize()];
                                let b = self.nodes.read().unwrap()[b_id.as_usize()];
                                let v = min(a.variable, b.variable);

                                let (low_a, high_a) = if a.variable == v {
                                    (a.low_child, a.high_child)
                                } else {
                                    (a_id, a_id)
                                };

                                let (low_b, high_b) = if b.variable == v {
                                    (b.low_child, b.high_child)
                                } else {
                                    (b_id, b_id)
                                };

                                let mut stack = pool.stack.lock().unwrap();
                                stack.push((a_id, b_id, v));
                                println!(
                                    "[{:?}] just pushed [{:?}, {:?}, {:?}]",
                                    thread::current().id(),
                                    a_id,
                                    b_id,
                                    v
                                );
                                stack.push((high_a, high_b, Variable::UNDEFINED_VARIABLE));
                                println!(
                                    "[{:?}] just pushed [{:?}, {:?}, {:?}]",
                                    thread::current().id(),
                                    high_a,
                                    high_b,
                                    Variable::UNDEFINED_VARIABLE
                                );
                                stack.push((low_a, low_b, Variable::UNDEFINED_VARIABLE));
                                println!(
                                    "[{:?}] just pushed [{:?}, {:?}, {:?}]",
                                    thread::current().id(),
                                    low_a,
                                    low_b,
                                    Variable::UNDEFINED_VARIABLE
                                );
                                drop(stack);
                                pool.cvar.notify_one();

                                continue;
                            }

                            let mut results = pool.results.lock().unwrap();
                            let h = results.pop().expect("high result present in result stack"); // can I guarantee the top of stack contains the children results?
                            let l = results.pop().expect("low result present in result stack");

                            let (c_node_id, c) = if l != h {
                                self.ensure_node(variable, l.0, h.0)
                            } else {
                                l
                            };

                            self.task_cache.insert((a_id, b_id), c_node_id);
                            results.push((c_node_id, c));
                            drop(results);
                        }

                        let idle_workers = pool.idle_workers.fetch_add(1, Ordering::SeqCst) + 1;
                        if idle_workers == total_workers {
                            pool.is_done.store(true, Ordering::Release);
                            pool.cvar.notify_all();
                            break;
                        }

                        // --- 4. SLEEP AND WAIT ---
                        let mut guard = pool.sleep_lock.lock().unwrap();

                        // We sleep while the system is NOT done, and the queue is still empty
                        while !pool.is_done.load(Ordering::Acquire)
                            && pool.stack.lock().unwrap().is_empty()
                        {
                            // We use `wait_timeout` to periodically wake up.
                            // This is crucial when mixing Lock-Free Queues with Mutex-based Condvars to
                            // prevent "missed notification" race conditions.
                            let (new_guard, result) = pool
                                .cvar
                                .wait_timeout(guard, Duration::from_millis(10)) // todo change as needed
                                .unwrap();

                            guard = new_guard; // Reassign the lock guard

                            if result.timed_out() {
                                break; // Loop around, drop the lock, and re-check the queue
                            }
                        }
                        drop(guard); // Explicitly drop the lock before transitioning states
                        pool.idle_workers.fetch_sub(1, Ordering::SeqCst);
                    }
                });
            }
        });

        let mut results = pool.results.lock().unwrap();
        let (root_id, root) = results.pop().expect("only one result expected");
        assert!(results.is_empty());
        (root_id, root)
    }

    fn ensure_node(
        &self,
        variable: Variable,
        low_child: NodeId,
        high_child: NodeId,
    ) -> (NodeId, Node) {
        let needle = Node::new(variable, low_child, high_child);
        if let Some(found) = self.node_table.get(&needle) {
            (*found, needle)
        } else {
            let node_id = NodeId(self.nodes.read().unwrap().len());
            self.nodes.write().unwrap().push(needle);
            self.node_table.insert(needle, node_id);
            (node_id, needle)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type ApplyFn = fn(&mut Bdd, NodeId, NodeId) -> (NodeId, Node);

    fn basic_apply_invariants(apply: ApplyFn) {
        // Taken from Ruddy:
        // https://github.com/sybila/ruddy/blob/e9b014b7fe3f5b1e8929632dc8a5ca4f9cde717e/src/split/apply.rs#L424
        let mut bdd = Bdd::new();

        let (a_id, a) = bdd.ensure_node(Variable(1), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (b_id, _) = bdd.ensure_node(Variable(2), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (tt_id, _) = (NodeId::TERMINAL_1, Node::one());
        let (ff_id, ff) = (NodeId::TERMINAL_0, Node::zero());

        let (res_id, res) = apply(&mut bdd, a_id, a_id);
        assert_eq!(res_id, a_id);
        assert_eq!(res, a);

        let (res_id, res) = apply(&mut bdd, a_id, tt_id);
        assert_eq!(res_id, a_id);
        assert_eq!(res, a);

        let (res_id, res) = apply(&mut bdd, a_id, ff_id);
        assert_eq!(res_id, ff_id);
        assert_eq!(res, ff);

        let (res_id, res) = apply(&mut bdd, a_id, b_id);
        let (res2_id, res2) = apply(&mut bdd, b_id, a_id);
        assert_eq!(res_id, res2_id);
        assert_eq!(res2, res);
    }

    #[test]
    fn basic_apply_recursive_invariants() {
        basic_apply_invariants(Bdd::apply_recursive);
    }

    #[test]
    fn basic_apply_iterative_invariants() {
        basic_apply_invariants(Bdd::apply_iterative);
    }

    fn make_thesis_example_bdds() -> (Bdd, NodeId, NodeId) {
        // Two BDDs taken from Lukas Urban's Thesis
        // https://is.muni.cz/th/danz1/Thesis.pdf#page=20
        let mut bdd = Bdd::new();

        let (a4_id, _) = bdd.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (a3_id, _) = bdd.ensure_node(Variable(2), NodeId::TERMINAL_1, a4_id);
        let (a2_id, _) = bdd.ensure_node(Variable(2), NodeId::TERMINAL_0, a4_id);
        let (a1_id, _) = bdd.ensure_node(Variable(1), a2_id, a3_id);

        let (b3_id, _) = bdd.ensure_node(Variable(3), NodeId::TERMINAL_1, NodeId::TERMINAL_0);
        let (b2_id, _) = bdd.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (b1_id, _) = bdd.ensure_node(Variable(2), b2_id, b3_id);

        (bdd, a1_id, b1_id)
    }

    #[test]
    fn thesis_example_constructs_correctly() {
        let (bdd, a_root_id, b_root_id) = make_thesis_example_bdds();

        let a1 = bdd.nodes[a_root_id.as_usize()];
        assert_eq!(a1.variable, Variable(1));

        let a2 = bdd.nodes[a1.low_child.as_usize()];
        assert_eq!(a2.variable, Variable(2));
        assert!(a2.low_child.is_zero());

        let a3 = bdd.nodes[a1.high_child.as_usize()];
        assert_eq!(a3.variable, a2.variable);
        assert!(a3.low_child.is_one());

        assert_eq!(a2.high_child, a3.high_child);
        let a4_id = a3.high_child;
        let a4 = bdd.nodes[a4_id.as_usize()];
        assert_eq!(a4.variable, Variable(3));
        assert_eq!(a4.low_child, a2.low_child);
        assert_eq!(a4.high_child, a3.low_child);

        let b1 = bdd.nodes[b_root_id.as_usize()];
        assert_eq!(b1.variable, Variable(2));

        let b2_id = b1.low_child;
        let b2 = bdd.nodes[b2_id.as_usize()];
        assert_eq!(a4_id, b2_id);
        assert_eq!(a4, b2);

        let b3 = bdd.nodes[b1.high_child.as_usize()];
        assert_eq!(b3.variable, Variable(3));
        assert!(b3.low_child.is_one());
        assert!(b3.high_child.is_zero());

        assert_eq!(bdd.nodes.len(), 8);
    }

    fn assert_thesis_example_apply(apply: ApplyFn) {
        let (mut bdd, a_root_id, b_root_id) = make_thesis_example_bdds();

        let (_, c1) = apply(&mut bdd, a_root_id, b_root_id);
        assert_eq!(c1.variable, Variable(1));
        assert_eq!(c1.low_child.as_usize(), 0);

        let c2 = bdd.nodes[c1.high_child.as_usize()];
        assert_eq!(c2.variable, Variable(2));
        assert_eq!(c2.high_child.as_usize(), 0);

        let c3 = bdd.nodes[c2.low_child.as_usize()];
        assert_eq!(c3.variable, Variable(3));
        assert_eq!(c3.low_child.as_usize(), 0);
        assert_eq!(c3.high_child.as_usize(), 1);

        assert_eq!(bdd.nodes.len(), 10);
    }

    #[test]
    fn apply_recursion_thesis_example() {
        assert_thesis_example_apply(Bdd::apply_recursive);
    }

    #[test]
    fn apply_iterative_thesis_example() {
        assert_thesis_example_apply(Bdd::apply_iterative);
    }

    #[test]
    fn merge_remaps_ids_and_deduplicates_nodes() {
        let mut bdd_a = Bdd::new();
        let (a4_id, _) = bdd_a.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (a3_id, _) = bdd_a.ensure_node(Variable(2), NodeId::TERMINAL_1, a4_id);
        let (a2_id, _) = bdd_a.ensure_node(Variable(2), NodeId::TERMINAL_0, a4_id);
        bdd_a.ensure_node(Variable(1), a2_id, a3_id);

        let mut bdd_b = Bdd::new();
        let (b3_id, _) = bdd_b.ensure_node(Variable(3), NodeId::TERMINAL_1, NodeId::TERMINAL_0);
        let (b2_id, _) = bdd_b.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (b1_id, _) = bdd_b.ensure_node(Variable(2), b2_id, b3_id);

        let nodes_before = bdd_a.nodes.len();
        let id_map = bdd_a.merge(&bdd_b);

        assert_eq!(id_map[&NodeId::TERMINAL_0], NodeId::TERMINAL_0);
        assert_eq!(id_map[&NodeId::TERMINAL_1], NodeId::TERMINAL_1);
        assert_eq!(id_map[&b2_id], a4_id);

        let merged_b1_id = id_map[&b1_id];
        let merged_b1 = bdd_a.nodes[merged_b1_id.as_usize()];
        assert_eq!(merged_b1.variable, Variable(2));
        assert_eq!(merged_b1.low_child, a4_id);
        assert_eq!(merged_b1.high_child, id_map[&b3_id]);

        let merged_b3 = bdd_a.nodes[id_map[&b3_id].as_usize()];
        assert_eq!(merged_b3.variable, Variable(3));
        assert!(merged_b3.low_child.is_one());
        assert!(merged_b3.high_child.is_zero());

        assert_eq!(bdd_a.nodes.len(), nodes_before + 2);
    }

    #[test]
    fn extract_remaps_ids_and_preserves_structure() {
        let (bdd, a_root_id, b_root_id) = make_thesis_example_bdds();

        let (extracted, new_a_root_id) = bdd.extract(a_root_id);

        assert_eq!(extracted.nodes.len(), 6);

        let a1 = extracted.nodes[new_a_root_id.as_usize()];
        assert_eq!(a1.variable, Variable(1));

        let a2 = extracted.nodes[a1.low_child.as_usize()];
        assert_eq!(a2.variable, Variable(2));
        assert!(a2.low_child.is_zero());

        let a3 = extracted.nodes[a1.high_child.as_usize()];
        assert_eq!(a3.variable, Variable(2));
        assert!(a3.low_child.is_one());

        assert_eq!(a2.high_child, a3.high_child);
        let a4 = extracted.nodes[a2.high_child.as_usize()];
        assert_eq!(a4.variable, Variable(3));
        assert_eq!(a4.low_child, a2.low_child);
        assert_eq!(a4.high_child, a3.low_child);

        let (extracted_b, new_b_root_id) = bdd.extract(b_root_id);
        assert_eq!(extracted_b.nodes.len(), 5);

        let b1 = extracted_b.nodes[new_b_root_id.as_usize()];
        assert_eq!(b1.variable, Variable(2));

        let b2 = extracted_b.nodes[b1.low_child.as_usize()];
        assert_eq!(b2, a4);

        let b3 = extracted_b.nodes[b1.high_child.as_usize()];
        assert_eq!(b3.variable, Variable(3));
        assert!(b3.low_child.is_one());
        assert!(b3.high_child.is_zero());
    }

    #[test]
    fn extract_terminal_returns_terminals_only() {
        let bdd = Bdd::new();
        let (extracted, root) = bdd.extract(NodeId::TERMINAL_1);

        assert_eq!(root, NodeId::TERMINAL_1);
        assert_eq!(extracted.nodes.len(), 2);
    }

    #[test]
    fn merge_then_apply_matches_single_bdd_apply() {
        let (mut bdd_ab, a_root_id, b_root_id) = make_thesis_example_bdds();
        let (_, expected) = bdd_ab.apply_recursive(a_root_id, b_root_id);

        let mut bdd_a = Bdd::new();
        let (a4_id, _) = bdd_a.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (a3_id, _) = bdd_a.ensure_node(Variable(2), NodeId::TERMINAL_1, a4_id);
        let (a2_id, _) = bdd_a.ensure_node(Variable(2), NodeId::TERMINAL_0, a4_id);
        let (a1_id, _) = bdd_a.ensure_node(Variable(1), a2_id, a3_id);

        let mut bdd_b = Bdd::new();
        let (b3_id, _) = bdd_b.ensure_node(Variable(3), NodeId::TERMINAL_1, NodeId::TERMINAL_0);
        let (b2_id, _) = bdd_b.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (b1_id, _) = bdd_b.ensure_node(Variable(2), b2_id, b3_id);

        let id_map = bdd_a.merge(&bdd_b);
        let (_, actual) = bdd_a.apply_recursive(a1_id, id_map[&b1_id]);

        assert_eq!(actual, expected);
    }

    #[test]
    fn parallel_apply() {
        let mut nodes: Vec<Node> = Vec::with_capacity(8);

        let zero = Node::zero();
        let zero_id = NodeId::TERMINAL_0;
        nodes.insert(zero_id.as_usize(), zero);
        let one = Node::one();
        let one_id = NodeId::TERMINAL_1;
        nodes.insert(one_id.as_usize(), one);

        let a4 = Node::new(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let a4_id = NodeId(2);
        nodes.insert(a4_id.as_usize(), a4);

        let a3 = Node::new(Variable(2), NodeId::TERMINAL_1, a4_id);
        let a3_id = NodeId(3);
        nodes.insert(a3_id.as_usize(), a3);
        let a2 = Node::new(Variable(2), NodeId::TERMINAL_0, a4_id);
        let a2_id = NodeId(4);
        nodes.insert(a2_id.as_usize(), a2);

        let a1 = Node::new(Variable(1), a2_id, a3_id);
        let a1_id = NodeId(5);
        nodes.insert(a1_id.as_usize(), a1);

        let b3 = Node::new(Variable(3), NodeId::TERMINAL_1, NodeId::TERMINAL_0);
        let b3_id = NodeId(6);
        nodes.insert(b3_id.as_usize(), b3);

        let _b2 = a4;
        let b2_id = a4_id;
        // nodes.insert(b2_id.as_usize(), b2); // avoid duplicities - node is identical to a4

        let b1 = Node::new(Variable(2), b2_id, b3_id);
        let b1_id = NodeId(7);
        nodes.insert(b1_id.as_usize(), b1);

        let mut bdd = BddParallel::new();
        bdd.nodes = RwLock::new(nodes);

        let node_table: DashMap<Node, NodeId> = DashMap::with_capacity(8);
        node_table.insert(zero, zero_id);
        node_table.insert(one, one_id);
        node_table.insert(a1, a1_id);
        node_table.insert(a2, a2_id);
        node_table.insert(a3, a3_id);
        node_table.insert(a4, a4_id);
        node_table.insert(b1, b1_id);
        // node_table.insert(b2, b2_id); // avoid duplicities - node is identical to a4
        node_table.insert(b3, b3_id);
        bdd.node_table = node_table;

        let (_, c1) = bdd.apply(a1_id, b1_id);
        assert_eq!(c1.variable, Variable(1));
        assert_eq!(c1.low_child.as_usize(), 0);

        let c2 = bdd.nodes.read().unwrap()[c1.high_child.as_usize()];
        assert_eq!(c2.variable, Variable(2));
        assert_eq!(c2.high_child.as_usize(), 0);

        let c3 = bdd.nodes.read().unwrap()[c2.low_child.as_usize()];
        assert_eq!(c3.variable, Variable(3));
        assert_eq!(c3.low_child.as_usize(), 0);
        assert_eq!(c3.high_child.as_usize(), 1);

        assert_eq!(bdd.nodes.read().unwrap().len(), 10);
    }
}
