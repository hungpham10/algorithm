#![allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::variables::{Variables, RuleError, DEFAULT_SCOPE};
    use tokio_test::block_on;

    /// Helper to initialize Variables with given time-series capacity and buffer size.
    fn init_vars(timeseries: usize, buffer: usize) -> Variables {
        Variables::new(timeseries, buffer)
    }

    /// Happy-path: create a variable, update it once, and verify last value.
    #[tokio::test]
    async fn test_create_and_update_happy_path() {
        let mut vars = init_vars(3, 2);
        vars.create(&"temp".to_string()).unwrap();
        let len = vars
            .update(&DEFAULT_SCOPE.to_string(), &"temp".to_string(), 25.0)
            .await
            .unwrap();
        assert_eq!(len, 1);
        assert_eq!(vars.last("temp").unwrap(), 25.0);
    }

    /// Edge case: creating the same variable twice yields an error.
    #[test]
    fn test_create_duplicate_variable() {
        let mut vars = init_vars(3, 2);
        vars.create(&"temp".to_string()).unwrap();
        let res = vars.create(&"temp".to_string());
        assert!(res.is_err(), "Expected error when creating duplicate variable");
    }

    /// Edge case: updating a non-existent variable returns an error.
    #[tokio::test]
    async fn test_update_nonexistent_variable() {
        let mut vars = init_vars(3, 2);
        let res = vars
            .update(&DEFAULT_SCOPE.to_string(), &"unknown".to_string(), 1.0)
            .await;
        assert!(res.is_err(), "Expected error when updating non-existent variable");
    }

    /// Edge case: get_by_expr with an invalid numeric format returns an error.
    #[test]
    fn test_get_by_expr_invalid_format() {
        let mut vars = init_vars(3, 2);
        vars.create(&"expr".to_string()).unwrap();
        let res = vars.get_by_expr("expr", "not_a_number");
        assert!(res.is_err(), "Expected error for invalid expression format");
    }

    /// Edge case: get_by_index with an out-of-bounds index returns an error.
    #[test]
    fn test_get_by_index_out_of_bounds() {
        let mut vars = init_vars(3, 2);
        vars.create(&"idx".to_string()).unwrap();
        let res = vars.get_by_index("idx", 10);
        assert!(res.is_err(), "Expected error for index out of bounds");
    }

    /// Edge case: querying length of a non-existent variable returns an error.
    #[test]
    fn test_len_nonexistent_variable() {
        let vars = init_vars(3, 2);
        let res = vars.len("unknown");
        assert!(res.is_err(), "Expected error when querying length of unknown variable");
    }

    /// Flushing all variables without an S3 client should early-return OK.
    #[tokio::test]
    async fn test_flush_all_no_s3() {
        let vars = init_vars(3, 2);
        let res = vars.flush_all().await;
        assert!(res.is_ok(), "Expected flush_all() to succeed with no S3 configured");
    }

    /// Time-series buffer should drop oldest entries when capacity is exceeded.
    #[tokio::test]
    async fn test_timeseries_capped() {
        let mut vars = init_vars(2, 1);
        vars.create(&"x".to_string()).unwrap();
        block_on(async {
            vars.update(&DEFAULT_SCOPE.to_string(), &"x".to_string(), 1.0)
                .await
                .unwrap();
            vars.update(&DEFAULT_SCOPE.to_string(), &"x".to_string(), 2.0)
                .await
                .unwrap();
            vars.update(&DEFAULT_SCOPE.to_string(), &"x".to_string(), 3.0)
                .await
                .unwrap();
        });
        // Only the two most recent values (3.0, 2.0) should remain.
        assert_eq!(vars.len("x").unwrap(), 2);
        assert_eq!(vars.get_by_index("x", 0).unwrap(), 3.0);
        assert_eq!(vars.get_by_index("x", 1).unwrap(), 2.0);
    }

    /// Scoping and flushing a specific scope without S3 should succeed.
    #[tokio::test]
    async fn test_flush_scope_no_s3() {
        let mut vars = init_vars(3, 2);
        vars.create(&"a".to_string()).unwrap();
        vars.create(&"b".to_string()).unwrap();
        vars.scope("myscope", &vec!["a".to_string()]).unwrap();
        vars.update(&"myscope".to_string(), &"a".to_string(), 42.0)
            .await
            .unwrap();
        let res = vars.flush("myscope").await;
        assert!(res.is_ok(), "Expected scoped flush() to succeed with no S3 configured");
    }
}