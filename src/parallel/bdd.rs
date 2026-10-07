use crate::parallel::{Bdd, GeneratedTask, Node, NodeId, SuccessorResults, Task, TaskId, Variable};
use crossbeam_queue::SegQueue;
use dashmap::DashMap;
use std::cmp::min;
use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::Duration;

struct WorkerPool {
    queue: SegQueue<Task>,

    // Termination detection state
    is_done: AtomicBool,
    idle_workers: AtomicUsize,

    // Standard library locking primitives used purely for thread sleeping
    sleep_lock: Mutex<()>,
    cvar: Condvar,
}

struct ProcessingState {
    queue: SegQueue<GeneratedTask>,
    results: DashMap<TaskId, (NodeId, Node)>,
}

impl Bdd {
    pub fn new() -> Self {
        let terminal_0 = Node::zero();
        let terminal_1 = Node::one();
        let nodes = DashMap::new();
        nodes.insert(NodeId::TERMINAL_0, terminal_0);
        nodes.insert(NodeId::TERMINAL_1, terminal_1);

        let node_table = DashMap::new();
        node_table.insert(terminal_0, NodeId::TERMINAL_0);
        node_table.insert(terminal_1, NodeId::TERMINAL_1);

        Bdd { nodes, node_table }
    }

    fn generate_tasks(
        &mut self,
        a_id: NodeId,
        b_id: NodeId,
        total_workers: usize,
    ) -> DashMap<Variable, SegQueue<GeneratedTask>> {
        let pool = Arc::new(WorkerPool {
            queue: SegQueue::new(),
            is_done: AtomicBool::new(false),
            idle_workers: AtomicUsize::new(0),
            sleep_lock: Mutex::new(()),
            cvar: Condvar::new(),
        });
        pool.queue.push((a_id, b_id, TaskId::new()));
        let generated_tasks: RwLock<DashMap<Variable, SegQueue<GeneratedTask>>> =
            RwLock::new(DashMap::new());
        generated_tasks
            .write()
            .unwrap()
            .insert(Variable::TERMINAL_VARIABLE, SegQueue::new());

        thread::scope(|s| {
            for _ in 0..total_workers {
                s.spawn(|| {
                    let pool = pool.clone();
                    loop {
                        if pool.is_done.load(Ordering::Acquire) {
                            break;
                        }

                        if let Some((a_id, b_id, task_id)) = pool.queue.pop() {
                            if a_id.is_terminal() && b_id.is_terminal() {
                                // TODO terminals could be processed here already
                                generated_tasks
                                    .read()
                                    .unwrap()
                                    .get(&Variable::TERMINAL_VARIABLE)
                                    .expect("A Queue for terminals should be present")
                                    .push((a_id, b_id, task_id, SuccessorResults::TERMINAL));
                                continue;
                            }

                            let a = self.nodes.get(&a_id).unwrap();
                            let b = self.nodes.get(&b_id).unwrap();
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

                            let low_task_id = TaskId::new();
                            let high_task_id = TaskId::new();
                            let successor_results_id =
                                SuccessorResults::new(low_task_id, high_task_id);
                            if let Some(queue) = generated_tasks.read().unwrap().get(&v) {
                                queue.push((a_id, b_id, task_id, successor_results_id));
                            } else {
                                let task_queue: SegQueue<GeneratedTask> = SegQueue::new();
                                task_queue.push((a_id, b_id, task_id, successor_results_id));
                                generated_tasks.write().unwrap().insert(v, task_queue);
                            }

                            pool.queue.push((low_a, low_b, low_task_id));
                            pool.queue.push((high_a, high_b, high_task_id));
                            pool.cvar.notify_one();

                            continue;
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
                        while !pool.is_done.load(Ordering::Acquire) && pool.queue.is_empty() {
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

        generated_tasks.into_inner().unwrap()
    }

    fn process_tasks(
        &mut self,
        tasks: DashMap<Variable, SegQueue<GeneratedTask>>,
        total_workers: usize,
    ) -> (NodeId, Node) {
        let mut keys = BTreeSet::new();
        tasks.iter().for_each(|item| {
            keys.insert(*item.key());
        });

        let mut pool = Arc::new(ProcessingState {
            queue: SegQueue::new(),
            results: DashMap::new(),
        });

        for variable in keys.into_iter().rev() {
            let (variable, queue) = tasks.remove(&variable).unwrap();

            Arc::get_mut(&mut pool).unwrap().queue = queue;

            thread::scope(|s| {
                for _ in 0..total_workers {
                    s.spawn(|| {
                        let pool = pool.clone();

                        while let Some((a_id, b_id, task_id, successor_results)) = pool.queue.pop()
                        {
                            if a_id.is_terminal() && b_id.is_terminal() {
                                if a_id.is_one() && b_id.is_one() {
                                    pool.results.insert(
                                        task_id,
                                        (
                                            NodeId::TERMINAL_1,
                                            *self.nodes.get(&NodeId::TERMINAL_1).unwrap(),
                                        ),
                                    );
                                } else {
                                    pool.results.insert(
                                        task_id,
                                        (
                                            NodeId::TERMINAL_0,
                                            *self.nodes.get(&NodeId::TERMINAL_0).unwrap(),
                                        ),
                                    );
                                }
                                continue;
                            }

                            let (low_result_id, high_result_id) = successor_results.into();

                            let (_, h) = pool
                                .results
                                .remove(&high_result_id)
                                .expect("High result not present");
                            let (_, l) = pool
                                .results
                                .remove(&low_result_id)
                                .expect("Low result not present");

                            let (c_node_id, c) = if l != h {
                                self.ensure_node(variable, l.0, h.0)
                            } else {
                                l
                            };

                            pool.results.insert(task_id, (c_node_id, c));
                        }
                    });
                }
            });
        }

        assert_eq!(pool.results.len(), 1);
        *Arc::into_inner(pool)
            .unwrap()
            .results
            .iter()
            .next()
            .unwrap()
            .value()
    }

    // TODO apply and generating tasks could be run asynchronously. When generate_tasks creates one, it could notify apply to start working
    pub fn apply(&mut self, a_id: NodeId, b_id: NodeId) -> (NodeId, Node) {
        let total_workers = 3;
        let tasks = self.generate_tasks(a_id, b_id, total_workers);

        self.process_tasks(tasks, total_workers)
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
            // TODO the TaskId could be reused for NodeId, meaning I could get rid off the results map and use the nodes map directly
            let node_id = NodeId::new();
            self.nodes.insert(node_id, needle);
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

        for node_id in other.nodes.iter() {
            let node_id = *node_id.key();
            if node_id.is_terminal() {
                continue;
            }

            Self::merge_node(self, other, node_id, &mut id_map);
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

        let node = other.nodes.get(&id).unwrap();
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

        let node = source.nodes.get(&id).unwrap();
        let low = Self::extract_node(extracted, source, node.low_child, id_map);
        let high = Self::extract_node(extracted, source, node.high_child, id_map);
        let (new_id, _) = extracted.ensure_node(node.variable, low, high);
        id_map.insert(id, new_id);
        new_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn basic_apply_invariants() {
        // Taken from Ruddy:
        // https://github.com/sybila/ruddy/blob/e9b014b7fe3f5b1e8929632dc8a5ca4f9cde717e/src/split/apply.rs#L424
        let mut bdd = Bdd::new();

        let (a_id, a) = bdd.ensure_node(Variable(1), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (b_id, _) = bdd.ensure_node(Variable(2), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (tt_id, _) = (NodeId::TERMINAL_1, Node::one());
        let (ff_id, ff) = (NodeId::TERMINAL_0, Node::zero());

        let (res_id, res) = bdd.apply(a_id, a_id);
        assert_eq!(res_id, a_id);
        assert_eq!(res, a);

        let (res_id, res) = bdd.apply(a_id, tt_id);
        assert_eq!(res_id, a_id);
        assert_eq!(res, a);

        let (res_id, res) = bdd.apply(a_id, ff_id);
        assert_eq!(res_id, ff_id);
        assert_eq!(res, ff);

        let (res_id, res) = bdd.apply(a_id, b_id);
        let (res2_id, res2) = bdd.apply(b_id, a_id);
        assert_eq!(res_id, res2_id);
        assert_eq!(res2, res);
    }

    fn make_thesis_example_bdds() -> (Bdd, NodeId, NodeId) {
        // Two BDDs taken from Lukas Urban's Thesis
        // https://is.muni.cz/th/danz1/Thesis.pdf#page=20
        let bdd = Bdd::new();

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

        let a1 = bdd.nodes.get(&a_root_id).unwrap();
        assert_eq!(a1.variable, Variable(1));

        let a2 = bdd.nodes.get(&a1.low_child).unwrap();
        assert_eq!(a2.variable, Variable(2));
        assert!(a2.low_child.is_zero());

        let a3 = bdd.nodes.get(&a1.high_child).unwrap();
        assert_eq!(a3.variable, a2.variable);
        assert!(a3.low_child.is_one());

        assert_eq!(a2.high_child, a3.high_child);
        let a4_id = a3.high_child;
        let a4 = bdd.nodes.get(&a4_id).unwrap();
        assert_eq!(a4.variable, Variable(3));
        assert_eq!(a4.low_child, a2.low_child);
        assert_eq!(a4.high_child, a3.low_child);

        let b1 = bdd.nodes.get(&b_root_id).unwrap();
        assert_eq!(b1.variable, Variable(2));

        let b2_id = b1.low_child;
        let b2 = bdd.nodes.get(&b2_id).unwrap();
        assert_eq!(a4_id, b2_id);
        assert_eq!(a4.value(), b2.value());

        let b3 = bdd.nodes.get(&b1.high_child).unwrap();
        assert_eq!(b3.variable, Variable(3));
        assert!(b3.low_child.is_one());
        assert!(b3.high_child.is_zero());

        assert_eq!(bdd.nodes.len(), 8);
    }

    #[test]
    fn generating_tasks() {
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

        let task_set = bdd
            .generate_tasks(a1_id, b1_id, 5)
            .iter()
            .flat_map(|value| {
                let queue = value.value();
                let mut new_queue = Vec::new();
                while !queue.is_empty() {
                    let (a, b, _, _) = queue.pop().unwrap();
                    new_queue.push((a, b));
                }
                // print!("Tasks for Variable: {:?}:[ ", value.key());
                // new_queue.iter().for_each(|item| print!("{:?}, ", item));
                // println!("]");
                new_queue.into_iter()
            })
            .collect();
        let expected_results = HashSet::from([
            (NodeId::TERMINAL_0, NodeId::TERMINAL_0),
            (NodeId::TERMINAL_0, NodeId::TERMINAL_1),
            (NodeId::TERMINAL_0, b2_id),
            (NodeId::TERMINAL_0, b2_id),
            (NodeId::TERMINAL_1, NodeId::TERMINAL_0),
            (NodeId::TERMINAL_1, NodeId::TERMINAL_1),
            (NodeId::TERMINAL_1, b2_id),
            (NodeId::TERMINAL_1, b2_id),
            (b2_id, b3_id),
            (b2_id, b3_id),
            (a3_id, b1_id),
            (a3_id, b1_id),
            (a2_id, b1_id),
            (a2_id, b1_id),
            (a1_id, b1_id),
        ]);
        assert_eq!(expected_results, task_set);
    }

    #[test]
    fn assert_thesis_example_apply() {
        let (mut bdd, a_root_id, b_root_id) = make_thesis_example_bdds();

        let (_, c1) = bdd.apply(a_root_id, b_root_id);
        assert_eq!(c1.variable, Variable(1));
        assert_eq!(c1.low_child, NodeId::TERMINAL_0);

        let c2 = bdd.nodes.get(&c1.high_child).unwrap();
        assert_eq!(c2.variable, Variable(2));
        assert_eq!(c2.high_child, NodeId::TERMINAL_0);

        let c3 = bdd.nodes.get(&c2.low_child).unwrap();
        assert_eq!(c3.variable, Variable(3));
        assert_eq!(c3.low_child, NodeId::TERMINAL_0);
        assert_eq!(c3.high_child, NodeId::TERMINAL_1);

        assert_eq!(bdd.nodes.len(), 10);
    }

    #[test]
    fn merge_remaps_ids_and_deduplicates_nodes() {
        let mut bdd_a = Bdd::new();
        let (a4_id, _) = bdd_a.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (a3_id, _) = bdd_a.ensure_node(Variable(2), NodeId::TERMINAL_1, a4_id);
        let (a2_id, _) = bdd_a.ensure_node(Variable(2), NodeId::TERMINAL_0, a4_id);
        bdd_a.ensure_node(Variable(1), a2_id, a3_id);

        let bdd_b = Bdd::new();
        let (b3_id, _) = bdd_b.ensure_node(Variable(3), NodeId::TERMINAL_1, NodeId::TERMINAL_0);
        let (b2_id, _) = bdd_b.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (b1_id, _) = bdd_b.ensure_node(Variable(2), b2_id, b3_id);

        let nodes_before = bdd_a.nodes.len();
        let id_map = bdd_a.merge(&bdd_b);

        assert_eq!(id_map[&NodeId::TERMINAL_0], NodeId::TERMINAL_0);
        assert_eq!(id_map[&NodeId::TERMINAL_1], NodeId::TERMINAL_1);
        assert_eq!(id_map[&b2_id], a4_id);

        let merged_b1_id = id_map[&b1_id];
        let merged_b1 = bdd_a.nodes.get(&merged_b1_id).unwrap();
        assert_eq!(merged_b1.variable, Variable(2));
        assert_eq!(merged_b1.low_child, a4_id);
        assert_eq!(merged_b1.high_child, id_map[&b3_id]);

        let merged_b3 = bdd_a.nodes.get(&id_map[&b3_id]).unwrap();
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

        let a1 = extracted.nodes.get(&new_a_root_id).unwrap();
        assert_eq!(a1.variable, Variable(1));

        let a2 = extracted.nodes.get(&a1.low_child).unwrap();
        assert_eq!(a2.variable, Variable(2));
        assert!(a2.low_child.is_zero());

        let a3 = extracted.nodes.get(&a1.high_child).unwrap();
        assert_eq!(a3.variable, Variable(2));
        assert!(a3.low_child.is_one());

        assert_eq!(a2.high_child, a3.high_child);
        let a4 = extracted.nodes.get(&a2.high_child).unwrap();
        assert_eq!(a4.variable, Variable(3));
        assert_eq!(a4.low_child, a2.low_child);
        assert_eq!(a4.high_child, a3.low_child);

        let (extracted_b, new_b_root_id) = bdd.extract(b_root_id);
        assert_eq!(extracted_b.nodes.len(), 5);

        let b1 = extracted_b.nodes.get(&new_b_root_id).unwrap();
        assert_eq!(b1.variable, Variable(2));

        let b2 = extracted_b.nodes.get(&b1.low_child).unwrap();
        assert_eq!(b2.value(), a4.value());

        let b3 = extracted_b.nodes.get(&b1.high_child).unwrap();
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
        let (_, expected) = bdd_ab.apply(a_root_id, b_root_id);

        let mut bdd_a = Bdd::new();
        let (a4_id, _) = bdd_a.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (a3_id, _) = bdd_a.ensure_node(Variable(2), NodeId::TERMINAL_1, a4_id);
        let (a2_id, _) = bdd_a.ensure_node(Variable(2), NodeId::TERMINAL_0, a4_id);
        let (a1_id, _) = bdd_a.ensure_node(Variable(1), a2_id, a3_id);

        let bdd_b = Bdd::new();
        let (b3_id, _) = bdd_b.ensure_node(Variable(3), NodeId::TERMINAL_1, NodeId::TERMINAL_0);
        let (b2_id, _) = bdd_b.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (b1_id, _) = bdd_b.ensure_node(Variable(2), b2_id, b3_id);

        let id_map = bdd_a.merge(&bdd_b);
        let (_, actual) = bdd_a.apply(a1_id, id_map[&b1_id]);

        assert_eq!(actual, expected);
    }
}
