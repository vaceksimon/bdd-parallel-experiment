use crate::parallel::{Bdd, GeneratedTask, Node, NodeId, SuccessorResults, Task, TaskId, Variable};
use crossbeam_queue::SegQueue;
use dashmap::DashMap;
use std::cmp::min;
use std::collections::BTreeSet;
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

struct WorkerPool2 {
    // TODO rename
    queue: SegQueue<GeneratedTask>,
    results: DashMap<TaskId, (NodeId, Node)>,

    // Termination detection state
    is_done: AtomicBool,
    idle_workers: AtomicUsize,

    // Standard library locking primitives used purely for thread sleeping
    _sleep_lock: Mutex<()>,
    _cvar: Condvar,
}

impl Bdd {
    pub fn new() -> Self {
        let terminal_0 = Node::zero();
        let terminal_1 = Node::one();
        let nodes = DashMap::new();
        nodes.insert(NodeId::TERMINAL_0, terminal_0);
        nodes.insert(NodeId::TERMINAL_1, terminal_1);

        Bdd {
            nodes,
            node_table: DashMap::new(),
            _task_cache: DashMap::new(),
        }
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

        let mut pool = Arc::new(WorkerPool2 {
            queue: SegQueue::new(),
            results: DashMap::new(),
            is_done: AtomicBool::new(false),
            idle_workers: AtomicUsize::new(0),
            _sleep_lock: Mutex::new(()),
            _cvar: Condvar::new(),
        });

        for variable in keys.into_iter().rev() {
            let (variable, queue) = tasks.remove(&variable).unwrap();

            // reset state
            pool.is_done.store(false, Ordering::Release);
            pool.idle_workers.store(0, Ordering::Release);
            Arc::get_mut(&mut pool).unwrap().queue = queue;

            thread::scope(|s| {
                for _ in 0..total_workers {
                    s.spawn(|| {
                        let pool = pool.clone();

                        while let Some((a_id, b_id, task_id, successor_results)) = pool.queue.pop()
                        {
                            // TODO not sure if while is correct here
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn generating_tasks() {
        let nodes = DashMap::new();

        let zero = Node::zero();
        let zero_id = NodeId::TERMINAL_0;
        nodes.insert(zero_id, zero);
        let one = Node::one();
        let one_id = NodeId::TERMINAL_1;
        nodes.insert(one_id, one);

        let a4 = Node::new(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let a4_id = NodeId::new();
        nodes.insert(a4_id, a4);

        let a3 = Node::new(Variable(2), NodeId::TERMINAL_1, a4_id);
        let a3_id = NodeId::new();
        nodes.insert(a3_id, a3);
        let a2 = Node::new(Variable(2), NodeId::TERMINAL_0, a4_id);
        let a2_id = NodeId::new();
        nodes.insert(a2_id, a2);

        let a1 = Node::new(Variable(1), a2_id, a3_id);
        let a1_id = NodeId::new();
        nodes.insert(a1_id, a1);

        let b3 = Node::new(Variable(3), NodeId::TERMINAL_1, NodeId::TERMINAL_0);
        let b3_id = NodeId::new();
        nodes.insert(b3_id, b3);

        let _b2 = a4;
        let b2_id = a4_id;
        // nodes.insert(b2_id.as_usize(), b2); // avoid duplicities - node is identical to a4

        let b1 = Node::new(Variable(2), b2_id, b3_id);
        let b1_id = NodeId::new();
        nodes.insert(b1_id, b1);

        let mut bdd = Bdd::new();
        bdd.nodes = nodes;

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
    fn apply() {
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

        let (_, c1) = bdd.apply(a1_id, b1_id);
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
}
