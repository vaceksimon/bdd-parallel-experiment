use crate::parallel::Bdd;
use crate::{Node, NodeId, Variable};
use crossbeam_queue::SegQueue;
use dashmap::DashMap;
use std::cmp::min;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::Duration;

struct WorkerPool {
    queue: SegQueue<(NodeId, NodeId, Variable)>,
    _results: SegQueue<(NodeId, Node)>,

    // Termination detection state
    is_done: AtomicBool,
    idle_workers: AtomicUsize,

    // Standard library locking primitives used purely for thread sleeping
    sleep_lock: Mutex<()>,
    cvar: Condvar,
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
            _node_table: DashMap::new(),
            _task_cache: DashMap::new(),
        }
    }

    pub fn apply(&mut self, a_id: NodeId, b_id: NodeId) -> HashSet<(NodeId, NodeId)> {
        let total_workers = 3;
        let pool = Arc::new(WorkerPool {
            queue: SegQueue::new(),
            _results: SegQueue::new(),
            is_done: AtomicBool::new(false),
            idle_workers: AtomicUsize::new(0),
            sleep_lock: Mutex::new(()),
            cvar: Condvar::new(),
        });
        pool.queue.push((a_id, b_id, Variable::UNDEFINED_VARIABLE));
        let generated_tasks = RwLock::new(HashSet::new()); // for now this is the goal

        thread::scope(|s| {
            for _ in 0..total_workers {
                s.spawn(|| {
                    println!("{:?} just spawned", thread::current().id());
                    let pool = pool.clone();
                    loop {
                        if pool.is_done.load(Ordering::Acquire) {
                            break;
                        }

                        if let Some((a_id, b_id, _)) = pool.queue.pop() {
                            println!("{:?} at work", thread::current().id());
                            if a_id.is_terminal() && b_id.is_terminal() {
                                generated_tasks.write().unwrap().insert((a_id, b_id));
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

                            generated_tasks.write().unwrap().insert((a_id, b_id));
                            pool.queue
                                .push((high_a, high_b, Variable::UNDEFINED_VARIABLE));
                            pool.queue
                                .push((low_a, low_b, Variable::UNDEFINED_VARIABLE));
                            pool.cvar.notify_all(); // we have just 4 workers, so its justifiable to notify all

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
                                .wait_timeout(guard, Duration::from_nanos(10)) // todo change as needed
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
        bdd._node_table = node_table;

        let task_set = bdd.apply(a1_id, b1_id);
        let expected_results = HashSet::from([
            (NodeId(0), NodeId(0)),
            (NodeId(0), NodeId(1)),
            (NodeId(0), NodeId(2)),
            (NodeId(0), NodeId(2)),
            (NodeId(1), NodeId(0)),
            (NodeId(1), NodeId(1)),
            (NodeId(1), NodeId(2)),
            (NodeId(1), NodeId(2)),
            (NodeId(2), NodeId(6)),
            (NodeId(2), NodeId(6)),
            (NodeId(3), NodeId(7)),
            (NodeId(3), NodeId(7)),
            (NodeId(4), NodeId(7)),
            (NodeId(4), NodeId(7)),
            (NodeId(5), NodeId(7)),
        ]);
        assert_eq!(expected_results, task_set);
    }
}
