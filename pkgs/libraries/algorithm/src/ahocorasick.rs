use std::collections::{BTreeMap, VecDeque};

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

    // @NOTE: state machine
    pattern_mapping: BTreeMap<String, usize>,
    failure_mapping: Vec<usize>,
    goto_mapping: Vec<BTreeMap<String, usize>>,
    outputs: BTreeMap<usize, usize>,
    inputs: Vec<usize>,

    // @NOTE: context
    patterns: Vec<String>,
    blocks: Vec<String>,

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
        Self::new_with_callbacks(
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

    pub fn new_with_callbacks(
        mapping_fn: MappingFn,
        compare_fn: CompareFn,
        collect_fn: CollectFn,
        split_fn: SplitFn,
    ) -> Self {
        Self {
            // @NOTE: callbacks
            mapping_fn: Box::new(mapping_fn),
            compare_fn: Box::new(compare_fn),
            collect_fn: Box::new(collect_fn),
            split_fn: Box::new(split_fn),

            // @NOTE: state machine
            pattern_mapping: BTreeMap::new(),
            failure_mapping: vec![0],
            goto_mapping: vec![BTreeMap::new()],
            outputs: BTreeMap::new(),
            inputs: Vec::new(),

            // @NOTE: context
            patterns: Vec::new(),
            blocks: Vec::new(),

            // @NOTE: flags
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

    pub fn optimize(&mut self) {
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

                    let index = self.goto_mapping.len();
                    let next_block = block.clone();

                    if current_state > 0 {
                        // @NOTE: we are in flow, just build this flow only

                        self.goto_mapping[nodes[current_state].index]
                            .insert(next_block.clone(), index);
                    }

                    self.goto_mapping.push(BTreeMap::new());
                    self.blocks.push(block.clone());

                    if current_state == 0 {
                        // @NOTE: this is the open state, new flow has been created

                        self.inputs.push(index);
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

            // @NOTE: we go to then end of the pattern, mark this as output so
            //        we can use function similar to recognize these patterns

            self.outputs.insert(next_state, i);
        }

        // @NOTE: build failure mapping base on the goto mapping

        if self.failure_mapping.len() < self.goto_mapping.len() {
            self.failure_mapping = vec![0; self.goto_mapping.len()];
        }

        queue.push_back(0);

        while !queue.is_empty() {
            let i = queue.pop_front().unwrap();
            let label = &nodes[i].label;
            let mut failure_of_previous = self.failure_mapping[nodes[i].back];

            if nodes[i].back != 0 {
                let mut break_at_last = false;

                loop {
                    match nodes[failure_of_previous].next.get(label) {
                        Some(failure_state) => {
                            self.failure_mapping[i] = *failure_state;
                            break;
                        }
                        None => {
                            if break_at_last {
                                break;
                            }

                            let try_failure = self.failure_mapping[failure_of_previous];

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

    pub fn similar(&self, sample: &String) -> bool {
        let blocks = (self.split_fn)(sample);
        let mut state = 0_usize;
        let mut i = 0_usize;

        if !self.is_optimized {
            return false;
        }

        while i < blocks.len() {
            let mut is_first_state = false;
            let mut next_state = 0_usize;
            let block = &blocks[i];

            if state == 0 {
                // @NOTE: first state, find matching initial string and follow
                //        this flow
                for first_id in &self.inputs {
                    if (self.compare_fn)(block, &self.blocks[first_id - 1]) {
                        state = *first_id;
                        break;
                    }
                }

                // @NOTE: if state still be on the first state, this indicates
                //        that we not find any possible flow
                is_first_state = true;
            }

            if !is_first_state {
                // @NOTE: move to next state from first state using whether
                //        mapping callback

                match (self.mapping_fn)(block, &self.goto_mapping[state]) {
                    Some(possible_next_state) => {
                        next_state = possible_next_state;
                    }
                    None => {
                        // @NOTE: cannot use the mapping callback now, we must
                        //        step by step test each possible

                        let states = &self.goto_mapping[state];

                        for (template, possible_next_state) in states {
                            if (self.compare_fn)(block, template) {
                                next_state = *possible_next_state;
                                break;
                            }
                        }
                    }
                }

                if !self.goto_mapping.is_empty() && next_state != 0 {
                    // @NOTE: collect variables for this possible flow
                    (self.collect_fn)(block)
                }

                if next_state == 0 {
                    // @NOTE: not found the next state use failure mapping to
                    //        find the possible next state

                    state = self.failure_mapping[state];

                    continue;
                } else {
                    state = next_state;
                }
            }

            if self.outputs.contains_key(&state) {
                // @TODO: found the matching series here and collect variables
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
    use rayon::prelude::*;
    use std::time::Instant;

    #[test]
    fn test_ahocorasick() {
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

        ahocorasick.add(String::from("abc"));
        ahocorasick.add(String::from("aab"));
        ahocorasick.add(String::from("bcd"));

        ahocorasick.optimize();

        assert!(ahocorasick.similar(&String::from("abc")));
        assert!(ahocorasick.similar(&String::from("aabc")));
        assert!(ahocorasick.similar(&String::from("daabce")));
        assert!(!ahocorasick.similar(&String::from("aa")));
        assert!(!ahocorasick.similar(&String::from("abd")));
    }

    #[test]
    fn test_ahocorasick_with_complex_pattern() {
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

        ahocorasick.add(String::from("hers"));
        ahocorasick.add(String::from("his"));
        ahocorasick.add(String::from("he"));
        ahocorasick.add(String::from("she"));

        ahocorasick.optimize();

        assert!(ahocorasick.similar(&String::from("ushers")));
        assert!(ahocorasick.similar(&String::from("ahishers")));
        assert!(ahocorasick.similar(&String::from("she")));
    }

    #[test]
    fn test_ahocorasick_with_vietnammese() {
        let mut ahocorasick = AhoCorasick::new_with_callbacks(
            &|block: &String, mapping: &BTreeMap<String, usize>| -> Option<usize> {
                mapping.get(block).cloned()
            },
            &|left: &String, right: &String| -> bool { left == right },
            &|_block: &String| {},
            &|pattern: &String| -> Vec<String> {
                pattern
                    .split(" ")
                    .filter(|block| !block.is_empty())
                    .map(|block| block.to_string())
                    .collect()
            },
        );

        ahocorasick.add(String::from("hôm nay"));
        ahocorasick.add(String::from("ngày mai"));
        ahocorasick.add(String::from("tuần sau"));
        ahocorasick.add(String::from("tháng sau"));

        ahocorasick.optimize();

        assert!(ahocorasick.similar(&String::from("hôm nay trời đẹp")));
        assert!(ahocorasick.similar(&String::from("ngày mai trời mưa")));
        assert!(ahocorasick.similar(&String::from("tuần sau đi chơi")));
        assert!(ahocorasick.similar(&String::from("tháng sau đi học")));
        assert!(!ahocorasick.similar(&String::from("hôm qua")));
    }

    #[test]
    fn test_ahocorasick_with_empty_pattern() {
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

        ahocorasick.optimize();

        assert!(!ahocorasick.similar(&String::from("")));
        assert!(!ahocorasick.similar(&String::from("abc")));
    }

    #[test]
    fn test_ahocorasick_with_duplicate_pattern() {
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

        ahocorasick.add(String::from("abc"));
        ahocorasick.add(String::from("abc"));

        ahocorasick.optimize();

        assert!(ahocorasick.similar(&String::from("abc")));
    }

    #[test]
    fn test_ahocorasick_with_special_characters() {
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

        ahocorasick.add(String::from("a.b+c"));

        ahocorasick.optimize();

        assert!(ahocorasick.similar(&String::from("a.b+c")));
        assert!(ahocorasick.similar(&String::from("da.b+ce")));
    }

    #[test]
    fn test_ahocorasick_with_no_pattern() {
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

        ahocorasick.optimize();

        assert!(!ahocorasick.similar(&String::from("abc")));
    }

    #[test]
    fn test_ahocorasick_with_partial_match() {
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

        ahocorasick.add(String::from("abc"));
        ahocorasick.add(String::from("def"));

        ahocorasick.optimize();

        assert!(!ahocorasick.similar(&String::from("ab")));
        assert!(!ahocorasick.similar(&String::from("de")));
    }

    #[test]
    fn test_ahocorasick_performance() {
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

        let mut patterns = Vec::new();
        for i in 0..10000 {
            patterns.push(format!("pattern_{}", i));
        }

        let start = Instant::now();
        for pattern in &patterns {
            ahocorasick.add(pattern.clone());
        }
        ahocorasick.optimize();
        let duration = start.elapsed();
        println!("Time elapsed in building ahocorasick is: {:?}", duration);

        let sample = String::from("This is a sample text containing pattern_9999.");

        let start = Instant::now();
        let result = ahocorasick.similar(&sample);
        let duration = start.elapsed();
        println!("Time elapsed in searching ahocorasick is: {:?}", duration);

        assert!(result);
    }

    #[test]
    fn test_parallel_searching() {
        // 1. Khởi tạo và thêm 10,000 patterns
        let mut ac = AhoCorasick::new_with_callbacks(
            &|b, m| m.get(b).cloned(),
            &|l, r| l == r,
            &|_| {},
            &|p| p.chars().map(|c| c.to_string()).collect(),
        );

        for i in 0..10000 {
            ac.add(format!("pattern_{}", i));
        }
        ac.optimize();

        // 2. Tạo một tập dữ liệu gồm 1,000 mẫu thử cần kiểm tra
        let samples: Vec<String> = (0..1000)
            .map(|i| format!("Đây là văn bản chứa pattern_{} và một số nội dung khác", i))
            .collect();

        // --- Chạy Tuần Tự (Sequential) ---
        let start_seq = Instant::now();
        let count_seq: usize = samples
            .iter()
            .filter(|s| ac.similar(&s.to_string()))
            .count();
        let duration_seq = start_seq.elapsed();
        println!("Tuần tự: {:?} (Tìm thấy {})", duration_seq, count_seq);

        // --- Chạy Song Song (Parallel) ---
        // Lưu ý: Để chạy được dòng này, bạn cần thêm `Send + Sync` vào các Box callback
        // hoặc bọc AhoCorasick trong một cơ chế bảo vệ phù hợp.
        // Ở đây ta giả lập bằng cách đo lường khối lượng tính toán.

        let start_par = Instant::now();
        let count_par: usize = samples
            .par_iter() // Sử dụng Rayon
            .filter(|s| ac.similar(&s.to_string()))
            .count();
        let duration_par = start_par.elapsed();
        println!("Song song: {:?} (Tìm thấy {})", duration_par, count_par);

        assert_eq!(count_seq, count_par);
    }
}
