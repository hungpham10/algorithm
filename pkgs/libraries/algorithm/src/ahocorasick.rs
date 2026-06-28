use std::collections::{BTreeMap, VecDeque};

use crate::storage::{self, Storage};

type MappingBox = Box<dyn Fn(&String, &BTreeMap<String, usize>) -> Option<usize> + Send + Sync>;
type CompareBox = Box<dyn Fn(&String, &String) -> bool + Send + Sync>;
type CollectBox = Box<dyn Fn(&String) + Send + Sync>;
type SplitBox = Box<dyn Fn(&String) -> Vec<String> + Send + Sync>;

type MappingFn =
    &'static (dyn Fn(&String, &BTreeMap<String, usize>) -> Option<usize> + Send + Sync);
type CompareFn = &'static (dyn Fn(&String, &String) -> bool + Send + Sync);
type CollectFn = &'static (dyn Fn(&String) + Send + Sync);
type SplitFn = &'static (dyn Fn(&String) -> Vec<String> + Send + Sync);

pub struct AhoCorasick {
    // @NOTE: callbacks
    mapping_fn: MappingBox,
    compare_fn: CompareBox,
    collect_fn: CollectBox,
    split_fn: SplitBox,

    // @NOTE: automaton storage (abstracted)
    automaton: Box<dyn Storage>,

    // @NOTE: pattern registry (pre-optimization)
    pattern_mapping: BTreeMap<String, usize>,
    patterns: Vec<String>,

    // @NOTE: flags
    is_optimized: bool,
}

struct Node {
    next: BTreeMap<String, usize>,
    back: usize,
    index: usize,
    label: String,
}

impl Default for AhoCorasick {
    fn default() -> Self {
        Self::new()
    }
}

impl AhoCorasick {
    pub fn new() -> Self {
        Self::with_storage(storage::InMemoryStorage::default())
    }

    pub fn new_with_callbacks(
        mapping_fn: MappingFn,
        compare_fn: CompareFn,
        collect_fn: CollectFn,
        split_fn: SplitFn,
    ) -> Self {
        Self::with_storage_and_callbacks(
            storage::InMemoryStorage::default(),
            mapping_fn,
            compare_fn,
            collect_fn,
            split_fn,
        )
    }

    /// Creates with a custom storage backend + default callbacks.
    ///
    /// ```ignore
    /// use algorithm::storage::InMemoryStorage;
    /// let ac = AhoCorasick::with_storage(InMemoryStorage::default());
    /// ```
    pub fn with_storage<S: storage::Storage + 'static>(storage: S) -> Self {
        Self::with_storage_and_callbacks(
            storage,
            &|block: &String, mapping: &BTreeMap<String, usize>| -> Option<usize> {
                mapping.get(block).cloned()
            },
            &|left: &String, right: &String| -> bool { left == right },
            &|_: &String| {},
            &|pattern: &String| -> Vec<String> {
                pattern
                    .split("")
                    .filter(|block| !block.is_empty())
                    .map(|block| block.to_string())
                    .collect()
            },
        )
    }

    /// Creates with a custom storage backend + custom callbacks.
    pub fn with_storage_and_callbacks<S: storage::Storage + 'static>(
        storage: S,
        mapping_fn: MappingFn,
        compare_fn: CompareFn,
        collect_fn: CollectFn,
        split_fn: SplitFn,
    ) -> Self {
        Self {
            mapping_fn: Box::new(mapping_fn),
            compare_fn: Box::new(compare_fn),
            collect_fn: Box::new(collect_fn),
            split_fn: Box::new(split_fn),

            automaton: Box::new(storage),

            pattern_mapping: BTreeMap::new(),
            patterns: Vec::new(),

            is_optimized: false,
        }
    }

    pub fn add(&mut self, pattern: String) {
        if !pattern.is_empty() && !self.pattern_mapping.contains_key(&pattern) {
            // @NOTE: configure new state machine
            self.pattern_mapping
                .insert(pattern.clone(), self.pattern_mapping.len());

            // @NOTE: add context
            self.patterns.push(pattern.clone());

            // @NOTE: reset optimized flag
            self.is_optimized = false;
        }
    }

    pub async fn optimize(&mut self) {
        let mut nodes = Vec::<Node>::new();
        let mut queue = VecDeque::<usize>::new();
        let mut state: usize = 1;

        // @NOTE: add root first
        nodes.push(Node {
            next: BTreeMap::new(),
            back: 0,
            index: 0,
            label: String::from(""),
        });

        for i in 0..self.patterns.len() {
            let pattern = &self.patterns[i];
            let mut current_state = 0_usize;
            let mut next_state = current_state;

            for block in (self.split_fn)(pattern) {
                let possible_next_state = nodes[current_state].next.get(&block);

                if let Some(possible_next_state) = possible_next_state {
                    next_state = *possible_next_state;
                } else {
                    // @NOTE: if next state not found, build it

                    let index = self.automaton.num_states().await.expect("valid automaton");
                    let next_block = block.clone();

                    if current_state > 0 {
                        // @NOTE: we are in flow, just build this flow only
                        self.automaton
                            .set_transition(nodes[current_state].index, &next_block, index)
                            .await
                            .expect("write transition");
                    }

                    self.automaton.add_state(&block).await.expect("add state");

                    if current_state == 0 {
                        // @NOTE: this is the open state, new flow has been created
                        self.automaton
                            .add_root_input(index)
                            .await
                            .expect("add root input");
                    }

                    // @NOTE: save this node into our database
                    nodes.push(Node {
                        next: BTreeMap::new(),
                        back: current_state,
                        label: block,
                        index,
                    });

                    // @NOTE: save reference between this node and next node
                    nodes[current_state].next.insert(next_block.clone(), index);
                    next_state = state;
                    state += 1;
                }

                current_state = next_state;
            }

            // @NOTE: we go to then end of the pattern, mark this as output
            self.automaton
                .set_output(next_state, i)
                .await
                .expect("set output");
        }

        // @NOTE: build failure mapping

        queue.push_back(0);

        while !queue.is_empty() {
            let i = queue.pop_front().unwrap();
            let label = &nodes[i].label;
            let mut failure_of_previous = self
                .automaton
                .get_failure(nodes[i].back)
                .await
                .expect("get failure");
            let mut break_at_last = false;

            if nodes[i].back != 0 {
                loop {
                    let transitions = self
                        .automaton
                        .get_transitions(failure_of_previous)
                        .await
                        .expect("get transitions");

                    let failure_state = transitions
                        .iter()
                        .find(|(l, _)| l == label)
                        .map(|(_, s)| *s);

                    match failure_state {
                        Some(failure_state) => {
                            self.automaton
                                .set_failure(i, failure_state)
                                .await
                                .expect("set failure");
                            break;
                        }
                        None => {
                            if break_at_last {
                                break;
                            }

                            let try_failure = self
                                .automaton
                                .get_failure(failure_of_previous)
                                .await
                                .expect("get failure");

                            if try_failure == failure_of_previous {
                                break_at_last = true;
                            }

                            failure_of_previous = try_failure;
                        }
                    }
                }
            }

            for next_state in nodes[i].next.values() {
                queue.push_back(*next_state);
            }
        }

        self.is_optimized = true;
    }

    pub async fn similar(&self, sample: &String) -> bool {
        let blocks = (self.split_fn)(sample);
        let mut state = 0_usize;
        let mut i = 0_usize;

        if !self.is_optimized {
            return false;
        }

        let root_inputs = self.automaton.get_root_inputs().await.expect("valid automaton");

        while i < blocks.len() {
            let mut next_state = 0_usize;
            let block = &blocks[i];

            if state == 0 {
                // @NOTE: first state, find matching initial string
                for first_id in &root_inputs {
                    let label = self.automaton.get_label(*first_id).await.expect("valid label");
                    if (self.compare_fn)(block, &label) {
                        state = *first_id;
                        break;
                    }
                }

                // @NOTE: skip transition block – root input was resolved above.
                //        Vào iteration tiếp theo state != 0, transition sẽ chạy.
                //        Nếu state vẫn là 0, ta chỉ việc tăng i và thử block kế.
            } else {
                // @NOTE: move to next state from current state

                let transitions = self
                    .automaton
                    .get_transitions(state)
                    .await
                    .expect("get transitions");
                let mapping: BTreeMap<String, usize> = transitions.into_iter().collect();

                match (self.mapping_fn)(block, &mapping) {
                    Some(possible_next_state) => {
                        next_state = possible_next_state;
                    }
                    None => {
                        for (template, possible_next_state) in &mapping {
                            if (self.compare_fn)(block, template) {
                                next_state = *possible_next_state;
                                break;
                            }
                        }
                    }
                }

                if next_state != 0 {
                    // @NOTE: collect variables for this possible flow
                    (self.collect_fn)(block);
                }

                if next_state == 0 {
                    // @NOTE: not found the next state, use failure mapping
                    state = self.automaton.get_failure(state).await.expect("get failure");
                    continue;
                } else {
                    state = next_state;
                }
            }

            if self
                .automaton
                .get_output(state)
                .await
                .expect("get output")
                .is_some()
            {
                return true;
            }

            i += 1;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_ahocorasick() {
        let mut ahocorasick = AhoCorasick::new_with_callbacks(
            &|block: &String, mapping: &BTreeMap<String, usize>| -> Option<usize> {
                mapping.get(block).cloned()
            },
            &|left: &String, right: &String| -> bool { left == right },
            &|_block: &String| {},
            &|pattern: &String| -> Vec<String> {
                pattern
                    .split("")
                    .filter(|block| !block.is_empty())
                    .map(|block| block.to_string())
                    .collect()
            },
        );

        ahocorasick.add("he".to_string());
        ahocorasick.add("she".to_string());
        ahocorasick.add("his".to_string());
        ahocorasick.add("hers".to_string());
        ahocorasick.optimize().await;

        assert!(!ahocorasick.similar(&"us".to_string()).await);
        assert!(!ahocorasick.similar(&"x".to_string()).await);

        assert!(ahocorasick.similar(&"she".to_string()).await);
        assert!(ahocorasick.similar(&"he".to_string()).await);
        assert!(ahocorasick.similar(&"his".to_string()).await);
        assert!(ahocorasick.similar(&"hers".to_string()).await);
        assert!(ahocorasick.similar(&"hello".to_string()).await);
    }

    #[tokio::test]
    async fn test_ahocorasick_with_complex_pattern() {
        let mut ahocorasick = AhoCorasick::new_with_callbacks(
            &|block: &String, mapping: &BTreeMap<String, usize>| -> Option<usize> {
                mapping.get(block).cloned()
            },
            &|left: &String, right: &String| -> bool { left == right },
            &|_block: &String| {},
            &|pattern: &String| -> Vec<String> {
                pattern
                    .split("")
                    .filter(|block| !block.is_empty())
                    .map(|block| block.to_string())
                    .collect()
            },
        );

        ahocorasick.add("CG".to_string());
        ahocorasick.add("TGC".to_string());
        ahocorasick.add("CGT".to_string());
        ahocorasick.add("GCC".to_string());
        ahocorasick.add("GTGC".to_string());
        ahocorasick.add("TCGT".to_string());
        ahocorasick.optimize().await;

        assert!(ahocorasick.similar(&"CG".to_string()).await);
        assert!(ahocorasick.similar(&"TGC".to_string()).await);
        assert!(ahocorasick.similar(&"CGT".to_string()).await);
        assert!(ahocorasick.similar(&"GCC".to_string()).await);
        assert!(ahocorasick.similar(&"GTGC".to_string()).await);
        assert!(ahocorasick.similar(&"TCGT".to_string()).await);

        assert!(ahocorasick.similar(&"ACGT".to_string()).await);
        assert!(ahocorasick.similar(&"GTCG".to_string()).await);
        assert!(ahocorasick.similar(&"TACG".to_string()).await);

        assert!(!ahocorasick.similar(&"AAA".to_string()).await);
        assert!(!ahocorasick.similar(&"GGG".to_string()).await);
        assert!(!ahocorasick.similar(&"CCC".to_string()).await);
        assert!(!ahocorasick.similar(&"TTT".to_string()).await);
    }

    #[tokio::test]
    async fn test_ahocorasick_with_vietnammese() {
        let mut ahocorasick = AhoCorasick::new_with_callbacks(
            &|block: &String, mapping: &BTreeMap<String, usize>| -> Option<usize> {
                mapping.get(block).cloned()
            },
            &|left: &String, right: &String| -> bool { left == right },
            &|_block: &String| {},
            &|pattern: &String| -> Vec<String> {
                pattern
                    .split("")
                    .filter(|block| !block.is_empty())
                    .map(|block| block.to_string())
                    .collect()
            },
        );

        ahocorasick.add("ư".to_string());
        ahocorasick.add("ới".to_string());
        ahocorasick.optimize().await;

        assert!(ahocorasick.similar(&("ư".to_string())).await);
        assert!(!ahocorasick.similar(&("ơi".to_string())).await);
        assert!(ahocorasick.similar(&("ưới".to_string())).await);
    }

    #[tokio::test]
    async fn test_ahocorasick_with_empty_pattern() {
        let mut ahocorasick = AhoCorasick::new_with_callbacks(
            &|block: &String, mapping: &BTreeMap<String, usize>| -> Option<usize> {
                mapping.get(block).cloned()
            },
            &|left: &String, right: &String| -> bool { left == right },
            &|_block: &String| {},
            &|pattern: &String| -> Vec<String> {
                pattern
                    .split("")
                    .filter(|block| !block.is_empty())
                    .map(|block| block.to_string())
                    .collect()
            },
        );

        ahocorasick.add(String::from(""));
        ahocorasick.optimize().await;

        assert!(!ahocorasick.similar(&"something".to_string()).await);
    }

    #[tokio::test]
    async fn test_ahocorasick_with_duplicate_pattern() {
        let mut ahocorasick = AhoCorasick::new_with_callbacks(
            &|block: &String, mapping: &BTreeMap<String, usize>| -> Option<usize> {
                mapping.get(block).cloned()
            },
            &|left: &String, right: &String| -> bool { left == right },
            &|_block: &String| {},
            &|pattern: &String| -> Vec<String> {
                pattern
                    .split("")
                    .filter(|block| !block.is_empty())
                    .map(|block| block.to_string())
                    .collect()
            },
        );

        ahocorasick.add("he".to_string());
        ahocorasick.add("he".to_string());
        ahocorasick.add("she".to_string());
        ahocorasick.optimize().await;

        assert!(!ahocorasick.similar(&"us".to_string()).await);
        assert!(!ahocorasick.similar(&"x".to_string()).await);

        assert!(ahocorasick.similar(&"she".to_string()).await);
        assert!(ahocorasick.similar(&"he".to_string()).await);
    }

    #[tokio::test]
    async fn test_ahocorasick_with_special_characters() {
        let mut ahocorasick = AhoCorasick::new_with_callbacks(
            &|block: &String, mapping: &BTreeMap<String, usize>| -> Option<usize> {
                mapping.get(block).cloned()
            },
            &|left: &String, right: &String| -> bool { left == right },
            &|_block: &String| {},
            &|pattern: &String| -> Vec<String> {
                pattern
                    .split("")
                    .filter(|block| !block.is_empty())
                    .map(|block| block.to_string())
                    .collect()
            },
        );

        ahocorasick.add("h,e".to_string());
        ahocorasick.add("s h e".to_string());
        ahocorasick.optimize().await;

        assert!(ahocorasick.similar(&"h,e".to_string()).await);
        assert!(ahocorasick.similar(&"s h e".to_string()).await);
    }

    #[tokio::test]
    async fn test_ahocorasick_with_no_pattern() {
        let ahocorasick = AhoCorasick::new_with_callbacks(
            &|block: &String, mapping: &BTreeMap<String, usize>| -> Option<usize> {
                mapping.get(block).cloned()
            },
            &|left: &String, right: &String| -> bool { left == right },
            &|_block: &String| {},
            &|pattern: &String| -> Vec<String> {
                pattern
                    .split("")
                    .filter(|block| !block.is_empty())
                    .map(|block| block.to_string())
                    .collect()
            },
        );

        // không add pattern nào, chỉ optimize rỗng
        // ahocorasick.optimize();  // không gọi optimize

        assert!(!ahocorasick.similar(&"something".to_string()).await);
    }

    #[tokio::test]
    async fn test_ahocorasick_with_partial_match() {
        let mut ahocorasick = AhoCorasick::new_with_callbacks(
            &|block: &String, mapping: &BTreeMap<String, usize>| -> Option<usize> {
                mapping.get(block).cloned()
            },
            &|left: &String, right: &String| -> bool { left == right },
            &|_block: &String| {},
            &|pattern: &String| -> Vec<String> {
                pattern
                    .split("")
                    .filter(|block| !block.is_empty())
                    .map(|block| block.to_string())
                    .collect()
            },
        );

        ahocorasick.add("abc".to_string());
        ahocorasick.add("def".to_string());
        ahocorasick.optimize().await;

        assert!(!ahocorasick.similar(&"ab".to_string()).await);
        assert!(ahocorasick.similar(&"abc".to_string()).await);
        // @NOTE: Aho-Corasick là substring matching, "abcdef" chứa "abc" → match
        assert!(ahocorasick.similar(&"abcdef".to_string()).await);
        assert!(ahocorasick.similar(&"def".to_string()).await);
        assert!(!ahocorasick.similar(&"de".to_string()).await);
    }

    #[tokio::test]
    async fn test_ahocorasick_performance() {
        let words = vec![
            "informatics",
            "information",
            "informative",
            "informing",
            "informant",
            "informally",
            "informal",
            "informed",
            "informer",
            "informers",
            "informing",
            "inform",
            "info",
            "infographic",
            "infographics",
            "infomercial",
            "infomercials",
            "infotainment",
            "infotainments",
            "infomania",
        ];

        let mut tries = AhoCorasick::new();
        for word in &words {
            tries.add(word.to_string());
        }
        tries.optimize().await;

        let inputs = vec![
            "informatics",
            "information",
            "informative",
            "informing",
            "t",
            "a",
            "b",
            "c",
            "infotainments",
            "infomania",
            "unknown",
            "nothing",
        ];

        let mut outputs = vec![false; inputs.len()];
        let now = Instant::now();
        // sequential
        for i in 0..inputs.len() {
            outputs[i] = tries.similar(&inputs[i].to_string()).await;
        }

        let elapsed = now.elapsed();
        println!(
            "\n\nAhoCorasick – single thread: {}ms\n\n",
            elapsed.as_millis()
        );

        assert!(outputs[0]); // informatics
        assert!(outputs[1]); // information
        assert!(outputs[2]); // informative
        assert!(outputs[3]); // informing
        assert!(!outputs[4]); // t (not a keyword)
        assert!(!outputs[5]); // a
        assert!(!outputs[6]); // b
        assert!(!outputs[7]); // c
        assert!(outputs[8]); // infotainments
        assert!(outputs[9]); // infomania
        assert!(!outputs[10]); // unknown
        assert!(!outputs[11]); // nothing
    }

    #[tokio::test]
    async fn test_parallel_searching() {
        let mut ahocorasick = AhoCorasick::new_with_callbacks(
            &|block: &String, mapping: &BTreeMap<String, usize>| -> Option<usize> {
                mapping.get(block).cloned()
            },
            &|left: &String, right: &String| -> bool { left == right },
            &|_block: &String| {},
            &|pattern: &String| -> Vec<String> {
                pattern
                    .split("")
                    .filter(|block| !block.is_empty())
                    .map(|block| block.to_string())
                    .collect()
            },
        );

        ahocorasick.add("he".to_string());
        ahocorasick.add("she".to_string());
        ahocorasick.add("his".to_string());
        ahocorasick.add("hers".to_string());
        ahocorasick.optimize().await;

        let samples: Vec<String> = vec![
            "us".to_string(),
            "she".to_string(),
            "he".to_string(),
            "his".to_string(),
            "hers".to_string(),
            "hello".to_string(),
            "x".to_string(),
        ];

        let now = Instant::now();
        // sequential (similar is async, so rayon won't work)
        let mut results = vec![false; samples.len()];
        for i in 0..samples.len() {
            results[i] = ahocorasick.similar(&samples[i]).await;
        }

        let elapsed = now.elapsed();
        println!(
            "\n\nAhoCorasick – sequential: {:?}ms\n\n",
            elapsed.as_millis()
        );

        assert!(!results[0]); // us
        assert!(results[1]); // she
        assert!(results[2]); // he
        assert!(results[3]); // his
        assert!(results[4]); // hers
        assert!(results[5]); // hello
        assert!(!results[6]); // x
    }
}
