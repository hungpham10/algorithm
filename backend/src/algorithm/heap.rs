#[derive(Clone)]
pub struct Heap<T: Clone> {
    items: Vec<T>,
    all: usize,
    branch: usize,
    comparator: fn(&T, &T) -> i64,
}

impl<T: Clone> Heap<T> {
    pub fn size(&self) -> usize { self.all }

    pub fn new(comparator: fn(&T, &T) -> i64) -> Self {
        Self {
            items: vec![],
            all: 0,
            branch: 0,
            comparator,
        }
    }

    pub fn get(&self) -> Option<&T> {
        if self.all == 0 {
            return None;
        }

        return self.items.get(0);
    }

    pub fn get_mut(&mut self) -> Option<&T> {
        if self.all == 0 {
            return None;
        }

        return self.items.get(0);
    }

    pub fn pop(&mut self) -> bool {
        return self.refresh(true);
    }

    pub fn push(&mut self, value: T) -> bool {
        if self.all == self.items.len() {
            self.items.push(value);
        } else {
            self.items[self.all] = value;
        }

        self.all = self.all + 1;
        self.branch = self.all / 2;

        return self.refresh(false);
    }

    fn refresh(&mut self, sorting: bool) -> bool {
        if self.branch > 0 && sorting {
            return false;
        }

        loop {
            if self.branch > 0 {
                self.branch = self.branch - 1;
            } else if sorting {
                self.all = self.all - 1;

                if self.all > 0 {
                    self.swap(0, self.all);
                }
            } else {
                break;
            }

            let mut copy_branch = self.branch;
            let mut left_leaf: usize;
            let mut right_leaf: usize;

            loop {
                left_leaf = 2 * copy_branch + 1;
                right_leaf = left_leaf + 1;

                if right_leaf < self.all {
                    let left = self.items.get(left_leaf).unwrap();
                    let right = self.items.get(right_leaf).unwrap();

                    if (self.comparator)(left, right) > 0 {
                        copy_branch = left_leaf;
                    } else {
                        copy_branch = right_leaf;
                    }
                } else {
                    break;
                }
            }

            if right_leaf == self.all {
                copy_branch = left_leaf;
            }

            loop {
                if copy_branch == self.branch {
                    break;
                }
                let top = self.items.get(self.branch).unwrap();
                let bot = self.items.get(copy_branch).unwrap();

                if (self.comparator)(top, bot) <= 0 {
                    break;
                }

                copy_branch = (copy_branch + copy_branch % 2) / 2 - 1;
            }

            let cache = copy_branch;

            while copy_branch != self.branch {
                copy_branch = (copy_branch + copy_branch % 2) / 2 - 1;
                self.swap(copy_branch, cache);
            }

            if self.branch == 0 && sorting {
                break;
            }
        }

        return true;
    }

    fn swap(&mut self, left_index: usize, right_index: usize) {
        self.items.swap(left_index, right_index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_heap() {
        let mut heap = Heap::new(|l: &i64, r: &i64| -> i64 { l - r });

        for i in 0..100 {
            assert_eq!(heap.push(i), true);
        }

        for i in (0..100).rev() {
            assert_eq!(heap.get().unwrap_or(&-1), &i);
            assert_eq!(heap.pop(), true);
        }
    }
}
