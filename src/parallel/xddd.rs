#[cfg(test)]
pub mod tests {
    use crate::parallel::{Bdd, SuccessorResults, TaskId, Variable};
    use crate::test_utils::load_util::parse_big_file_path;
    use crate::test_utils::parallel::helpers::merge_load;
    use dashmap::DashMap;

    #[test]
    fn test() {
        let bdd_id_a = 3789;
        let bdd_id_b = 2789;

        let file_a = parse_big_file_path(bdd_id_a);
        let file_b = parse_big_file_path(bdd_id_b);
        let (mut bdd_actual, a_root, b_root) = merge_load(&file_a, &file_b);
        let original_size = bdd_actual.nodes.len();

        let generated_tasks = bdd_actual.generate_tasks(a_root, b_root, 10);
        let generated_tasks_2: DashMap<Variable, DashMap<TaskId, SuccessorResults>> =
            DashMap::new();
        let mut tasks_total = 0;
        let mut variables_total = 0;

        for (variable, tasks) in generated_tasks {
            println!("Variable {:?} has {} tasks", variable, tasks.len());
            tasks_total += tasks.len();
            variables_total += 1;
            generated_tasks_2.insert(variable, tasks);
        }
        println!("====================================");
        println!("Total variables = {}", variables_total);
        println!("Total tasks = {}", tasks_total);

        let transformed_task = Bdd::transform_tasks(generated_tasks_2);
        bdd_actual.process_tasks(transformed_task, (a_root, b_root), 10);
        let new_size = bdd_actual.nodes.len();
        println!("Resulting BDD has {} nodes", new_size);
        println!("New nodes = {}", new_size - original_size);
    }
}
