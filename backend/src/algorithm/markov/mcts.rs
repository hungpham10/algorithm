use rand::distributions::WeightedIndex;
use rand::prelude::*;

use anyhow::{anyhow, Result};

#[derive(Clone)]
pub struct Node {
    children: Vec<usize>,
    nvisits: u32,
    value: f64,
}

impl Node {
    fn ucb(&self, tvisits: u32, exploration: f64) -> f64 {
        if self.nvisits == 0 {
            f64::INFINITY
        } else {
            self.value / self.nvisits as f64
                + exploration * (2.0 * (tvisits as f64).ln() / self.nvisits as f64).sqrt()
        }
    }
}

pub trait Handler {
    fn terminated(&self, tree: &[Node], route: &[usize], depth: usize) -> bool;
    fn expand(&self, tree: &[Node], route: &[usize], leaf: &usize) -> Vec<Node>;
    fn evaluate(&self, node: &Node) -> f64;
    fn rollout(&self, node: &Node) -> f64;
}

pub struct MonteCarloTreeSearch {
    handler: Box<dyn Handler>,
    tree: Vec<Node>,
    iroot: usize,
    exploration: f64,
    discount: f64,
    combat: bool,
}

impl MonteCarloTreeSearch {
    pub fn new(handler: Box<dyn Handler>, combat: bool, exploration: f64, discount: f64) -> Self {
        MonteCarloTreeSearch {
            tree: vec![Node {
                children: Vec::new(),
                nvisits: 0,
                value: 0.0,
            }],
            iroot: 0,
            exploration,
            discount,
            combat,
            handler,
        }
    }

    pub fn shift(&mut self, inode: usize) -> Result<()> {
        if !self.tree[self.iroot].children.contains(&inode) {
            return Err(anyhow!(
                "Provided inode {} is not a child of the current root {}",
                inode,
                self.iroot
            ));
        }

        let mut tree = Vec::new();
        let mut nodes_to_process = vec![(inode, 0)];

        while let Some((old_index, new_index)) = nodes_to_process.pop() {
            // Clone the node
            let mut node = self.tree[old_index].clone();

            // Store the old children and clear the children vector
            let old_children = node.children.clone();
            node.children.clear();

            // Add the cloned node to the new tree
            tree.push(node);

            // Add children to the queue and update the node's children indices
            for &child_index in &old_children {
                let new_child_index = tree.len();
                tree[new_index].children.push(new_child_index);
                nodes_to_process.push((child_index, new_child_index));
            }
        }

        self.tree = tree;
        self.iroot = 0;

        Ok(())
    }

    pub fn actions(&mut self, depth: usize) -> Result<Vec<usize>> {
        loop {
            let mut selected = self.select(self.iroot);
            let leaf = match selected.last() {
                Some(&id) => Ok(id),
                None => Err(anyhow!(
                    "select empty action, possible wrong somewhere else"
                )),
            }?;

            if selected.len() == depth {
                break;
            } else {
                let expanded = self.expand(&selected, leaf);
                let (inode, reward) = self.simulate(leaf, &expanded)?;

                selected.push(inode);
                self.backpropagate(&selected, reward);

                if self.handler.terminated(&self.tree, &selected, depth) {
                    return Ok(selected);
                }
            }
        }

        Err(anyhow!("Too deep to explore"))
    }

    fn select(&self, mut current: usize) -> Vec<usize> {
        let mut selected = vec![current];

        while !self.tree[current].children.is_empty() {
            let total_visits = self.tree[current].nvisits;
            let best_child = self.tree[current]
                .children
                .iter()
                .max_by(|&&a, &&b| {
                    let ucb_a = self.tree[a].ucb(total_visits, self.exploration);
                    let ucb_b = self.tree[b].ucb(total_visits, self.exploration);
                    ucb_a
                        .partial_cmp(&ucb_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied()
                .unwrap();
            selected.push(best_child);
            current = best_child;
        }
        selected
    }

    fn expand(&mut self, route: &[usize], leaf: usize) -> Vec<Node> {
        let possible = self.handler.expand(&self.tree, route, &leaf);
        let n = self.tree.len();

        for i in 0..(possible.len()) {
            self.tree[leaf].children.push(i + n);
        }

        self.tree.extend_from_slice(possible.as_slice());
        possible
    }

    fn simulate(&mut self, selected: usize, possible: &[Node]) -> Result<(usize, f64)> {
        let mut rng = rand::thread_rng();
        let distribution = WeightedIndex::new(
            &possible
                .iter()
                .map(|node| self.handler.evaluate(node))
                .collect::<Vec<f64>>(),
        )?;
        let chosen = distribution.sample(&mut rng);
        let reward = self.handler.rollout(&possible[chosen]);
        let inode = self.tree[selected].children[chosen];

        self.tree[inode].value = reward;
        self.tree[inode].nvisits += 1;

        Ok((inode, reward))
    }

    fn backpropagate(&mut self, selected: &[usize], rollout_value: f64) {
        let mut depth = 0;
        let mut reward = rollout_value;
        let factor = self.discount;

        for &node_index in selected.iter().rev() {
            if depth > 0 {
                self.tree[node_index].nvisits += 1;
                self.tree[node_index].value += reward * factor.powi(depth);
            }

            if self.combat {
                reward = -reward;
            }
            depth += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rand::distributions::WeightedIndex;
    use rand::prelude::*;

    // Mock Handler for testing
    struct MockHandler;

    impl Handler for MockHandler {
        fn terminated(&self, _tree: &[Node], route: &[usize], depth: usize) -> bool {
            route.len() >= depth
        }

        fn expand(&self, tree: &[Node], _route: &[usize], leaf: &usize) -> Vec<Node> {
            // Create two child nodes for each leaf with predictable values
            vec![
                Node {
                    children: Vec::new(),
                    nvisits: 0,
                    value: 0.0,
                },
                Node {
                    children: Vec::new(),
                    nvisits: 0,
                    value: 0.0,
                },
            ]
        }

        fn evaluate(&self, _node: &Node) -> f64 {
            // Return fixed evaluation scores for predictability
            1.0
        }

        fn rollout(&self, _node: &Node) -> f64 {
            // Return a fixed rollout value for predictability
            0.5
        }
    }

    fn setup_mcts() -> MonteCarloTreeSearch {
        MonteCarloTreeSearch::new(Box::new(MockHandler), false, 1.0, 0.99)
    }

    #[test]
    fn test_shift_valid_child() {
        let mut mcts = setup_mcts();

        // Expand the root to create children
        let route = vec![0];
        let expanded = mcts.expand(&route, 0);
        let new_nodes_start = mcts.tree.len() - expanded.len();
        mcts.tree[0].children = (new_nodes_start..mcts.tree.len()).collect();

        // Simulate a node to set some statistics
        let (new_node, reward) = mcts.simulate(0, &expanded).unwrap();
        mcts.backpropagate(&vec![0, new_node], reward);

        // Store original statistics of the first child
        let first_child_index = mcts.tree[0].children[0];
        let original_nvisits = mcts.tree[first_child_index].nvisits;
        let original_value = mcts.tree[first_child_index].value;

        // Shift to the first child
        mcts.shift(first_child_index).unwrap();

        // Verify new root
        assert_eq!(mcts.iroot, 0, "New root should be at index 0");
        assert_eq!(
            mcts.tree[0].nvisits, original_nvisits,
            "Root nvisits should be preserved"
        );
        assert_eq!(
            mcts.tree[0].value, original_value,
            "Root value should be preserved"
        );
        assert!(
            mcts.tree.len() <= 1 || !mcts.tree[0].children.is_empty(),
            "Children should be preserved if they existed"
        );
    }

    #[test]
    fn test_shift_invalid_child() {
        let mut mcts = setup_mcts();
        let result = mcts.shift(999); // Non-existent child
        assert!(result.is_err(), "Shifting to invalid child should fail");
        assert_eq!(
            result.unwrap_err().to_string(),
            "Provided inode 999 is not a child of the current root 0",
            "Error message should indicate invalid child"
        );
    }

    #[test]
    fn test_shift_empty_subtree() {
        let mut mcts = setup_mcts();

        // Expand to create a child
        let route = vec![0];
        let expanded = mcts.expand(&route, 0);
        let new_nodes_start = mcts.tree.len() - expanded.len();
        mcts.tree[0].children = (new_nodes_start..mcts.tree.len()).collect();

        let first_child_index = mcts.tree[0].children[0];

        // Shift to a leaf node (no children)
        mcts.shift(first_child_index).unwrap();

        assert_eq!(mcts.tree.len(), 1, "Tree should contain only the new root");
        assert_eq!(mcts.iroot, 0, "Root should be at index 0");
        assert!(
            mcts.tree[0].children.is_empty(),
            "New root should have no children"
        );
    }

    #[test]
    fn test_select() {
        let mut mcts = setup_mcts();

        // Expand root to create children
        let route = vec![0];
        let expanded = mcts.expand(&route, 0);
        let new_nodes_start = mcts.tree.len() - expanded.len();
        mcts.tree[0].children = (new_nodes_start..mcts.tree.len()).collect();

        // Simulate to set some statistics
        let (new_node, reward) = mcts.simulate(0, &expanded).unwrap();
        mcts.backpropagate(&vec![0, new_node], reward);

        let selected = mcts.select(0);
        assert_eq!(selected[0], 0, "Selection should start at root");
        assert!(
            mcts.tree[0].children.contains(&selected[1]),
            "Selected node should be a child of root"
        );
    }

    #[test]
    fn test_expand_and_simulate() {
        let mut mcts = setup_mcts();
        let route = vec![0];
        let expanded = mcts.expand(&route, 0);
        assert_eq!(expanded.len(), 2, "Expand should create two children");

        let new_nodes_start = mcts.tree.len() - expanded.len();
        mcts.tree[0].children = (new_nodes_start..mcts.tree.len()).collect();

        let (new_node, reward) = mcts.simulate(0, &expanded).unwrap();
        assert_eq!(reward, 0.5, "Simulate should return fixed rollout value");
        assert_eq!(
            mcts.tree[new_node].nvisits, 1,
            "Simulated node should have one visit"
        );
        assert_eq!(
            mcts.tree[new_node].value, 0.5,
            "Simulated node should have correct value"
        );
    }

    #[test]
    fn test_backpropagate() {
        let mut mcts = setup_mcts();
        let route = vec![0];
        let expanded = mcts.expand(&route, 0);
        let new_nodes_start = mcts.tree.len() - expanded.len();
        mcts.tree[0].children = (new_nodes_start..mcts.tree.len()).collect();

        let (new_node, reward) = mcts.simulate(0, &expanded).unwrap();
        mcts.backpropagate(&vec![0, new_node], reward);

        assert_eq!(mcts.tree[0].nvisits, 1, "Root should have one visit");
        assert_eq!(mcts.tree[new_node].nvisits, 1, "Leaf should have one visit");
        assert_eq!(
            mcts.tree[0].value,
            reward * mcts.discount,
            "Root value should be discounted"
        );
        assert_eq!(
            mcts.tree[new_node].value, reward,
            "Leaf value should match reward"
        );
    }

    #[test]
    fn test_backpropagate_combat_mode() {
        let mut mcts = MonteCarloTreeSearch::new(Box::new(MockHandler), true, 1.0, 0.99);
        let route = vec![0];
        let expanded = mcts.expand(&route, 0);
        let new_nodes_start = mcts.tree.len() - expanded.len();
        mcts.tree[0].children = (new_nodes_start..mcts.tree.len()).collect();

        // Expand another level to test alternating rewards
        let route2 = vec![0, new_nodes_start];
        let expanded2 = mcts.expand(&route2, new_nodes_start);
        let new_nodes_start2 = mcts.tree.len() - expanded2.len();
        mcts.tree[new_nodes_start].children = (new_nodes_start2..mcts.tree.len()).collect();

        let (leaf_node, reward) = mcts.simulate(new_nodes_start, &expanded2).unwrap();
        mcts.backpropagate(&vec![0, new_nodes_start, leaf_node], reward);

        // In combat mode: leaf gets +reward, parent gets -reward*discount, grandparent gets +reward*discount^2
        assert_eq!(
            mcts.tree[leaf_node].value, reward,
            "Leaf should have positive reward"
        );
        assert_eq!(
            mcts.tree[new_nodes_start].value,
            -reward * mcts.discount,
            "Parent should have negated discounted reward"
        );
        assert_eq!(
            mcts.tree[0].value,
            reward * mcts.discount.powi(2),
            "Grandparent should have positive double-discounted reward"
        );
    }

    #[test]
    fn test_actions() {
        let mut mcts = setup_mcts();
        let depth = 2;
        let result = mcts.actions(depth);

        assert!(result.is_ok(), "Actions should succeed for valid depth");
        let actions = result.unwrap();
        assert_eq!(actions.len(), depth, "Actions should reach specified depth");
        assert_eq!(actions[0], 0, "Actions should start at root");
    }
}
