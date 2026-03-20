use std::cmp::Ordering;

pub fn binary_search<T, K, F>(arr: &[T], target: &K, comparator: F) -> Option<usize>
where
    F: Fn(&K, &T) -> Ordering,
{
    let mut left = 0;
    let mut right = arr.len();

    while left < right {
        let mid = left + (right - left) / 2;

        match comparator(target, &arr[mid]) {
            Ordering::Less => right = mid,
            Ordering::Equal => return Some(mid),
            Ordering::Greater => left = mid + 1,
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_search_with_comparator() {
        let arr = vec![2, 5, 7, 8, 11, 12];

        // Ascending order comparator
        let ascending_comparator = |a: &i32, b: &i32| a.cmp(b);
        assert_eq!(binary_search(&arr, &7, ascending_comparator), Some(2));
        assert_eq!(binary_search(&arr, &1, ascending_comparator), None);

        // Descending order comparator
        let descending_comparator = |a: &i32, b: &i32| b.cmp(a);
        let descending_arr = vec![12, 11, 8, 7, 5, 2];
        assert_eq!(
            binary_search(&descending_arr, &7, descending_comparator),
            Some(3)
        );
        assert_eq!(
            binary_search(&descending_arr, &1, descending_comparator),
            None
        );

        let empty_arr: Vec<i32> = vec![];
        assert_eq!(binary_search(&empty_arr, &5, ascending_comparator), None);
    }

    #[test]
    fn test_binary_search_with_struct_comparator() {
        struct Person {
            name: String,
            age: i32,
        }

        let people = vec![
            Person {
                name: "Alice".to_string(),
                age: 30,
            },
            Person {
                name: "Bob".to_string(),
                age: 25,
            },
            Person {
                name: "Charlie".to_string(),
                age: 35,
            },
        ];

        let age_comparator = |a: &Person, b: &Person| a.age.cmp(&b.age);
        let target = Person {
            name: "".to_string(),
            age: 25,
        }; // Name doesn't matter for this search

        assert_eq!(binary_search(&people, &target, age_comparator), Some(1));

        let target = Person {
            name: "".to_string(),
            age: 40,
        };
        assert_eq!(binary_search(&people, &target, age_comparator), None);
    }
}
