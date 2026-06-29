use async_trait::async_trait;
use std::collections::BTreeMap;
use std::fmt;

// ==================== Error Type ====================

#[derive(Debug)]
pub enum StorageError {
    #[allow(dead_code)]
    BranchOutOfRange(usize),
    Internal(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::BranchOutOfRange(id) => write!(f, "branch id {id} out of range"),
            StorageError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for StorageError {}

pub type Result<T> = std::result::Result<T, StorageError>;

const EMPTY: usize = 0;

#[async_trait]
pub trait Storage: Send + Sync {
    // ── Radix-style: node management ──
    async fn new_node(&mut self, prefix: Vec<u8>, record: usize) -> Result<usize>;
    async fn update_node(
        &mut self,
        id: usize,
        prefix: Option<Vec<u8>>,
        record: Option<usize>,
    ) -> Result<()>;
    async fn add_child(&mut self, parent_id: usize, child_id: usize) -> Result<()>;
    async fn get_node(&self, id: usize) -> Result<(Vec<u8>, usize)>;
    async fn get_children(&self, id: usize) -> Result<Vec<usize>>;
    async fn set_root(&mut self, shard: usize, root_id: usize) -> Result<()>;
    async fn get_root(&self, shard: usize) -> Result<usize>;

    // ── Automaton-style: state machine ──
    async fn add_state(&mut self, label: &str) -> Result<usize>;
    async fn set_transition(&mut self, from: usize, label: &str, to: usize) -> Result<()>;
    async fn get_transitions(&self, from: usize) -> Result<Vec<(String, usize)>>;
    async fn set_failure(&mut self, state: usize, fail: usize) -> Result<()>;
    async fn get_failure(&self, state: usize) -> Result<usize>;
    async fn set_output(&mut self, state: usize, pattern_idx: usize) -> Result<()>;
    async fn get_output(&self, state: usize) -> Result<Option<usize>>;
    async fn add_root_input(&mut self, state: usize) -> Result<()>;
    async fn get_root_inputs(&self) -> Result<Vec<usize>>;
    async fn get_label(&self, state: usize) -> Result<String>;
    async fn num_states(&self) -> Result<usize>;
}

// ==================== In-Memory Storage (Radix + Automaton) ====================

pub struct InMemoryStorage {
    // ── Radix data ──
    nodes: Vec<(Vec<u8>, usize)>,
    children: Vec<Vec<usize>>,
    roots: Vec<usize>,

    // ── Automaton data ──
    labels: Vec<String>,
    transitions: Vec<BTreeMap<String, usize>>,
    failures: Vec<usize>,
    outputs: BTreeMap<usize, usize>,
    root_inputs: Vec<usize>,
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self {
            // Radix sentinel tại index 0
            nodes: vec![(vec![], 0)],
            children: vec![vec![]],
            roots: vec![],

            // Automaton root state tại index 0 (dùng chung sentinel với radix)
            labels: vec![String::new()],
            transitions: vec![BTreeMap::new()],
            failures: vec![0],
            outputs: BTreeMap::new(),
            root_inputs: Vec::new(),
        }
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    // ==================== Radix Methods ====================

    async fn new_node(&mut self, prefix: Vec<u8>, record: usize) -> Result<usize> {
        let id = self.nodes.len();
        self.nodes.push((prefix, record));
        self.children.push(Vec::new());
        Ok(id)
    }

    async fn update_node(
        &mut self,
        id: usize,
        prefix: Option<Vec<u8>>,
        record: Option<usize>,
    ) -> Result<()> {
        if let Some(p) = prefix {
            self.nodes[id].0 = p;
        }
        if let Some(r) = record {
            self.nodes[id].1 = r;
        }
        Ok(())
    }

    async fn add_child(&mut self, parent_id: usize, child_id: usize) -> Result<()> {
        self.children[parent_id].push(child_id);
        Ok(())
    }

    async fn get_node(&self, id: usize) -> Result<(Vec<u8>, usize)> {
        if id >= self.nodes.len() {
            return Err(StorageError::BranchOutOfRange(id));
        }
        Ok(self.nodes[id].clone())
    }

    async fn get_children(&self, id: usize) -> Result<Vec<usize>> {
        if id >= self.children.len() {
            return Ok(vec![]);
        }
        Ok(self.children[id].clone())
    }

    async fn set_root(&mut self, shard: usize, root_id: usize) -> Result<()> {
        if shard >= self.roots.len() {
            self.roots.resize(shard + 1, 0);
        }
        self.roots[shard] = root_id;
        Ok(())
    }

    async fn get_root(&self, shard: usize) -> Result<usize> {
        Ok(self.roots.get(shard).copied().unwrap_or(EMPTY))
    }

    // ==================== Automaton Methods ====================

    async fn add_state(&mut self, label: &str) -> Result<usize> {
        let id = self.labels.len();
        self.labels.push(label.to_string());
        self.transitions.push(BTreeMap::new());
        self.failures.push(0);
        Ok(id)
    }

    async fn set_transition(&mut self, from: usize, label: &str, to: usize) -> Result<()> {
        self.transitions[from].insert(label.to_string(), to);
        Ok(())
    }

    async fn get_transitions(&self, from: usize) -> Result<Vec<(String, usize)>> {
        Ok(self.transitions[from].clone().into_iter().collect())
    }

    async fn set_failure(&mut self, state: usize, fail: usize) -> Result<()> {
        self.failures[state] = fail;
        Ok(())
    }

    async fn get_failure(&self, state: usize) -> Result<usize> {
        Ok(self.failures[state])
    }

    async fn set_output(&mut self, state: usize, pattern_idx: usize) -> Result<()> {
        self.outputs.insert(state, pattern_idx);
        Ok(())
    }

    async fn get_output(&self, state: usize) -> Result<Option<usize>> {
        Ok(self.outputs.get(&state).copied())
    }

    async fn add_root_input(&mut self, state: usize) -> Result<()> {
        self.root_inputs.push(state);
        Ok(())
    }

    async fn get_root_inputs(&self) -> Result<Vec<usize>> {
        Ok(self.root_inputs.clone())
    }

    async fn get_label(&self, state: usize) -> Result<String> {
        if state >= self.labels.len() {
            return Err(StorageError::BranchOutOfRange(state));
        }
        Ok(self.labels[state].clone())
    }

    async fn num_states(&self) -> Result<usize> {
        Ok(self.transitions.len())
    }
}

// =========================================================================
//  Redis Storage
//  (chỉ build khi feature "redis" được bật)
// =========================================================================

#[cfg(feature = "redis")]
pub mod redis {
    //! Redis-backed Storage implementation (Radix + Automaton).
    //!
    //! Dùng `MultiplexedConnection` (async) bên trong, bridge qua sync bằng
    //! `tokio::runtime::Handle::block_on`. Nhờ đó nhận được connection từ
    //! `Resolver::cache()` mà không cần tạo sync connection riêng.
    //!
    //! ## Cấu trúc key
    //!
    //! | Key                        | Kiểu  | Mục đích                        |
    //! |----------------------------|-------|---------------------------------|
    //! | `{prefix}:branch`          | List  | prefix của từng node            |
    //! | `{prefix}:record`          | List  | record của từng node            |
    //! | `{prefix}:forward:{id}`    | List  | children list của node          |
    //! | `{prefix}:endpoint`        | Hash  | root ID cho mỗi shard           |
    //! | `{prefix}:label`           | List  | label của từng state            |
    //! | `{prefix}:trans:{id}`      | Hash  | transitions của state           |
    //! | `{prefix}:failure`         | List  | failure link của state          |
    //! | `{prefix}:output`          | Hash  | output (pattern_idx) của state  |
    //! | `{prefix}:root_inputs`     | List  | danh sách root input states     |

    use std::sync::Arc;

    use redis::aio::MultiplexedConnection;
    use tokio::sync::Mutex;

    use super::{Result, Storage, StorageError};

    // ==================== KeyBuilder ====================

    type KeyFormatter = Arc<dyn Fn(&str) -> String + Send + Sync>;

    /// Cấu hình key cho Redis storage.
    ///
    /// Mặc định format: `{prefix}:{name}` và `{prefix}:{name}:{id}`.
    /// Có thể dùng `with_formatter` để custom hoàn toàn.
    pub struct KeyBuilder {
        prefix: String,
        formatter: Option<KeyFormatter>,
    }

    impl KeyBuilder {
        pub fn new(prefix: &str) -> Self {
            Self {
                prefix: prefix.to_string(),
                formatter: None,
            }
        }

        /// Dùng custom formatter thay vì default `{prefix}:{name}`.
        pub fn with_formatter(prefix: &str, f: KeyFormatter) -> Self {
            Self {
                prefix: prefix.to_string(),
                formatter: Some(f),
            }
        }

        /// `key("branch")` → `"{prefix}:branch"`
        pub fn key(&self, name: &str) -> String {
            match &self.formatter {
                Some(f) => f(name),
                None => format!("{}:{}", self.prefix, name),
            }
        }

        /// `indexed("forward", 5)` → `"{prefix}:forward:5"`
        pub fn indexed(&self, name: &str, idx: usize) -> String {
            self.key(&format!("{name}:{idx}"))
        }
    }

    /// Helper shorthand: `cmd("LLEN")` → `redis::cmd("LLEN")`
    fn cmd(name: &str) -> redis::Cmd {
        redis::cmd(name)
    }

    // ==================== RedisStorage ====================

    pub struct RedisStorage {
        conn: Arc<Mutex<MultiplexedConnection>>,
        kb: KeyBuilder,
    }

    impl RedisStorage {
        /// Helper: lock the mutex, unwrap on poison.
        async fn lock(&self) -> tokio::sync::MutexGuard<'_, MultiplexedConnection> {
            self.conn.lock().await
        }

        /// Tạo storage từ `redis::Client` (async).
        pub async fn new(client: redis::Client, prefix: &str) -> Result<Self> {
            let conn = client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            let s = Self {
                conn: Arc::new(Mutex::new(conn)),
                kb: KeyBuilder::new(prefix),
            };
            s.init().await?;
            Ok(s)
        }

        /// Tạo storage từ `MultiplexedConnection` có sẵn (vd từ `Resolver::cache()`).
        pub async fn from_multiplexed(conn: MultiplexedConnection, prefix: &str) -> Result<Self> {
            let s = Self {
                conn: Arc::new(Mutex::new(conn)),
                kb: KeyBuilder::new(prefix),
            };
            s.init().await?;
            Ok(s)
        }

        /// Tạo storage với `KeyBuilder` tuỳ chỉnh + client.
        pub async fn with_key_builder(client: redis::Client, kb: KeyBuilder) -> Result<Self> {
            let conn = client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            let s = Self {
                conn: Arc::new(Mutex::new(conn)),
                kb,
            };
            s.init().await?;
            Ok(s)
        }

        /// Tạo storage với `MultiplexedConnection` + `KeyBuilder` custom.
        pub async fn from_multiplexed_with_key_builder(
            conn: MultiplexedConnection,
            kb: KeyBuilder,
        ) -> Result<Self> {
            let s = Self {
                conn: Arc::new(Mutex::new(conn)),
                kb,
            };
            s.init().await?;
            Ok(s)
        }

        async fn init(&self) -> Result<()> {
            let mut conn = self.lock().await;

            let exists: bool = cmd("EXISTS")
                .arg(self.kb.key("branch"))
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            if !exists {
                redis::pipe()
                    .rpush(self.kb.key("branch"), b"" as &[u8])
                    .rpush(self.kb.key("record"), 0i64)
                    .rpush(self.kb.key("label"), "")
                    .rpush(self.kb.key("failure"), 0i64)
                    .exec_async(&mut *conn)
                    .await
                    .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;
            }

            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Storage for RedisStorage {
        // ==================== Radix Methods ====================

        async fn new_node(&mut self, prefix: Vec<u8>, record: usize) -> Result<usize> {
            let mut conn = self.lock().await;

            let id: usize = cmd("LLEN")
                .arg(self.kb.key("branch"))
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            redis::pipe()
                .rpush(self.kb.key("branch"), &prefix[..])
                .rpush(self.kb.key("record"), record as i64)
                .exec_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(id)
        }

        async fn update_node(
            &mut self,
            id: usize,
            prefix: Option<Vec<u8>>,
            record: Option<usize>,
        ) -> Result<()> {
            let mut conn = self.lock().await;

            let mut pipe = redis::pipe();
            if let Some(p) = prefix {
                pipe.lset(self.kb.key("branch"), id as isize, &p[..]);
            }
            if let Some(r) = record {
                pipe.lset(self.kb.key("record"), id as isize, r as i64);
            }

            pipe.exec_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(())
        }

        async fn add_child(&mut self, parent_id: usize, child_id: usize) -> Result<()> {
            let mut conn = self.lock().await;

            cmd("RPUSH")
                .arg(self.kb.indexed("forward", parent_id))
                .arg(child_id as i64)
                .query_async::<()>(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(())
        }

        async fn get_node(&self, id: usize) -> Result<(Vec<u8>, usize)> {
            let mut conn = self.lock().await;

            let prefix: Vec<u8> = cmd("LINDEX")
                .arg(self.kb.key("branch"))
                .arg(id as isize)
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            let rec: i64 = cmd("LINDEX")
                .arg(self.kb.key("record"))
                .arg(id as isize)
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok((prefix, rec as usize))
        }

        async fn get_children(&self, id: usize) -> Result<Vec<usize>> {
            let mut conn = self.lock().await;

            let children: Vec<i64> = cmd("LRANGE")
                .arg(self.kb.indexed("forward", id))
                .arg(0i64)
                .arg(-1i64)
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(children.into_iter().map(|x| x as usize).collect())
        }

        async fn set_root(&mut self, shard: usize, root_id: usize) -> Result<()> {
            let mut conn = self.lock().await;

            cmd("HSET")
                .arg(self.kb.key("endpoint"))
                .arg(shard as i64)
                .arg(root_id as i64)
                .query_async::<()>(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(())
        }

        async fn get_root(&self, shard: usize) -> Result<usize> {
            let mut conn = self.lock().await;

            let root: Option<i64> = cmd("HGET")
                .arg(self.kb.key("endpoint"))
                .arg(shard as i64)
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(root.unwrap_or(0) as usize)
        }

        // ==================== Automaton Methods ====================

        async fn add_state(&mut self, label: &str) -> Result<usize> {
            let mut conn = self.lock().await;

            let id: usize = cmd("LLEN")
                .arg(self.kb.key("label"))
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            redis::pipe()
                .rpush(self.kb.key("label"), label)
                .rpush(self.kb.key("failure"), 0i64)
                .exec_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(id)
        }

        async fn set_transition(&mut self, from: usize, label: &str, to: usize) -> Result<()> {
            let mut conn = self.lock().await;

            cmd("HSET")
                .arg(self.kb.indexed("trans", from))
                .arg(label)
                .arg(to as i64)
                .query_async::<()>(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(())
        }

        async fn get_transitions(&self, from: usize) -> Result<Vec<(String, usize)>> {
            let mut conn = self.lock().await;

            let pairs: Vec<(String, String)> = cmd("HGETALL")
                .arg(self.kb.indexed("trans", from))
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(pairs
                .into_iter()
                .map(|(k, v)| (k, v.parse::<usize>().unwrap_or(0)))
                .collect())
        }

        async fn set_failure(&mut self, state: usize, fail: usize) -> Result<()> {
            let mut conn = self.lock().await;

            cmd("LSET")
                .arg(self.kb.key("failure"))
                .arg(state as isize)
                .arg(fail as i64)
                .query_async::<()>(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(())
        }

        async fn get_failure(&self, state: usize) -> Result<usize> {
            let mut conn = self.lock().await;

            let val: Option<i64> = cmd("LINDEX")
                .arg(self.kb.key("failure"))
                .arg(state as isize)
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(val.unwrap_or(0) as usize)
        }

        async fn set_output(&mut self, state: usize, pattern_idx: usize) -> Result<()> {
            let mut conn = self.lock().await;

            cmd("HSET")
                .arg(self.kb.key("output"))
                .arg(state as i64)
                .arg(pattern_idx as i64)
                .query_async::<()>(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(())
        }

        async fn get_output(&self, state: usize) -> Result<Option<usize>> {
            let mut conn = self.lock().await;

            let val: Option<i64> = cmd("HGET")
                .arg(self.kb.key("output"))
                .arg(state as i64)
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(val.map(|v| v as usize))
        }

        async fn add_root_input(&mut self, state: usize) -> Result<()> {
            let mut conn = self.lock().await;

            cmd("RPUSH")
                .arg(self.kb.key("root_inputs"))
                .arg(state as i64)
                .query_async::<()>(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(())
        }

        async fn get_root_inputs(&self) -> Result<Vec<usize>> {
            let mut conn = self.lock().await;

            let vals: Vec<i64> = cmd("LRANGE")
                .arg(self.kb.key("root_inputs"))
                .arg(0i64)
                .arg(-1i64)
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(vals.into_iter().map(|v| v as usize).collect())
        }

        async fn get_label(&self, state: usize) -> Result<String> {
            let mut conn = self.lock().await;

            let val: Option<Vec<u8>> = cmd("LINDEX")
                .arg(self.kb.key("label"))
                .arg(state as isize)
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            match val {
                Some(bytes) => {
                    String::from_utf8(bytes).map_err(|e| StorageError::Internal(e.to_string()))
                }
                None => Ok(String::new()),
            }
        }

        async fn num_states(&self) -> Result<usize> {
            let mut conn = self.lock().await;

            let n: usize = cmd("LLEN")
                .arg(self.kb.key("label"))
                .query_async(&mut *conn)
                .await
                .map_err(|e: redis::RedisError| StorageError::Internal(e.to_string()))?;

            Ok(n)
        }
    }

    // ── Tests ──────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use std::sync::atomic::{AtomicU16, Ordering};

        use super::*;
        use crate::storage::Storage;

        static COUNTER: AtomicU16 = AtomicU16::new(0);

        /// Tạo RedisStorage mới với prefix unique (cần tokio runtime).
        async fn new_test_storage() -> RedisStorage {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let client = redis::Client::open("redis://127.0.0.1:6379/15")
                .expect("redis connection failed — is redis-server running?");
            RedisStorage::new(client, &format!("test:merged:{n}"))
                .await
                .expect("init failed")
        }

        // ── Radix-style tests ──

        #[tokio::test]
        async fn test_new_node_and_get_node() {
            let mut s = new_test_storage().await;
            let id = s.new_node(b"hello".to_vec(), 42).await.unwrap();
            assert_ne!(id, 0, "id should not be the sentinel");

            let (prefix, record) = s.get_node(id).await.unwrap();
            assert_eq!(prefix, b"hello");
            assert_eq!(record, 42);
        }

        #[tokio::test]
        async fn test_update_node() {
            let mut s = new_test_storage().await;
            let id = s.new_node(b"init".to_vec(), 1).await.unwrap();

            s.update_node(id, Some(b"updated".to_vec()), Some(99))
                .await
                .unwrap();

            let (prefix, record) = s.get_node(id).await.unwrap();
            assert_eq!(prefix, b"updated");
            assert_eq!(record, 99);
        }

        #[tokio::test]
        async fn test_add_child_and_get_children() {
            let mut s = new_test_storage().await;
            let parent = s.new_node(b"parent".to_vec(), 0).await.unwrap();
            let child1 = s.new_node(b"child1".to_vec(), 1).await.unwrap();
            let child2 = s.new_node(b"child2".to_vec(), 2).await.unwrap();

            s.add_child(parent, child1).await.unwrap();
            s.add_child(parent, child2).await.unwrap();

            let children = s.get_children(parent).await.unwrap();
            assert_eq!(children, vec![child1, child2]);
        }

        #[tokio::test]
        async fn test_root() {
            let mut s = new_test_storage().await;

            assert_eq!(s.get_root(3).await.unwrap(), 0, "fresh shard returns 0");

            s.set_root(3, 42).await.unwrap();
            assert_eq!(s.get_root(3).await.unwrap(), 42);

            s.set_root(3, 99).await.unwrap();
            assert_eq!(s.get_root(3).await.unwrap(), 99);
        }

        #[tokio::test]
        async fn test_consecutive_ids() {
            let mut s = new_test_storage().await;
            let a = s.new_node(b"a".to_vec(), 10).await.unwrap();
            let b = s.new_node(b"b".to_vec(), 20).await.unwrap();
            let c = s.new_node(b"c".to_vec(), 30).await.unwrap();

            assert_eq!(a, 1);
            assert_eq!(b, 2);
            assert_eq!(c, 3);
        }

        // ── Automaton-style tests ──

        #[tokio::test]
        async fn test_add_state() {
            let mut s = new_test_storage().await;
            let id = s.add_state("a").await.unwrap();
            assert_eq!(id, 1, "first real state gets ID 1");
            assert_eq!(s.num_states().await.unwrap(), 2);
        }

        #[tokio::test]
        async fn test_label() {
            let mut s = new_test_storage().await;
            let id = s.add_state("hello").await.unwrap();
            assert_eq!(s.get_label(id).await.unwrap(), "hello");
            assert_eq!(s.get_label(0).await.unwrap(), "");
        }

        #[tokio::test]
        async fn test_transitions() {
            let mut s = new_test_storage().await;
            let s1 = s.add_state("a").await.unwrap();
            let s2 = s.add_state("b").await.unwrap();
            s.set_transition(0, "x", s1).await.unwrap();
            s.set_transition(s1, "y", s2).await.unwrap();

            let t0 = s.get_transitions(0).await.unwrap();
            assert!(t0.contains(&("x".into(), s1)));

            let t1 = s.get_transitions(s1).await.unwrap();
            assert!(t1.contains(&("y".into(), s2)));
        }

        #[tokio::test]
        async fn test_failure() {
            let mut s = new_test_storage().await;
            let id = s.add_state("test").await.unwrap();
            assert_eq!(s.get_failure(id).await.unwrap(), 0);
            s.set_failure(id, 42).await.unwrap();
            assert_eq!(s.get_failure(id).await.unwrap(), 42);
        }

        #[tokio::test]
        async fn test_output() {
            let mut s = new_test_storage().await;
            let id = s.add_state("term").await.unwrap();
            assert_eq!(s.get_output(id).await.unwrap(), None);
            s.set_output(id, 7).await.unwrap();
            assert_eq!(s.get_output(id).await.unwrap(), Some(7));
        }

        #[tokio::test]
        async fn test_root_inputs() {
            let mut s = new_test_storage().await;
            let s1 = s.add_state("s1").await.unwrap();
            let s2 = s.add_state("s2").await.unwrap();
            s.add_root_input(s1).await.unwrap();
            s.add_root_input(s2).await.unwrap();

            let inputs = s.get_root_inputs().await.unwrap();
            assert_eq!(inputs, vec![s1, s2]);
        }
    }
}
