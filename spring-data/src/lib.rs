//! spring-data — Spring Data 风格的 Repository 抽象
//!
//! 提供:
//! - [`Repository`] trait：定义标准 CRUD 操作
//! - [`InMemoryRepository<T>`]：基于 `HashMap` + 自动递增 u64 主键的内存实现

use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

// ─────────────────────────────────────────────
//  Repository trait
// ─────────────────────────────────────────────

/// Spring Data 风格的 Repository 接口。
/// 所有操作均通过实现此 trait 的具体类型暴露给用户。
pub trait Repository<T>: Send + Sync {
    /// 保存一条新记录，返回自动生成的 u64 主键。
    fn save(&self, entity: T) -> u64;
    /// 按 id 更新，返回是否存在该记录。
    fn update(&self, id: u64, entity: T) -> bool;
    /// 按主键查询（不可变引用通过回调方式返回结果）。
    /// 注意：线程安全实现通常会在持有读锁期间调用闭包。
    fn find_by_id<R, F: FnOnce(Option<&T>) -> R>(&self, id: u64, f: F) -> R;
    /// 遍历所有记录，通过回调暴露 (id, &T)。
    /// 注意：线程安全实现通常会在持有读锁期间调用闭包。
    fn for_each<F: FnMut(u64, &T)>(&self, f: F);
    /// 将所有记录克隆出来，以 Vec<(u64, T)> 形式返回（需要 T: Clone）。
    fn find_all_cloned(&self) -> Vec<(u64, T)>
    where
        T: Clone;
    /// 按主键删除，返回是否存在。
    fn delete_by_id(&self, id: u64) -> bool;
    /// 清空所有记录。
    fn delete_all(&self);
    /// 记录总数。
    fn count(&self) -> usize;
    /// 判断主键是否存在。
    fn exists_by_id(&self, id: u64) -> bool;
}

// ─────────────────────────────────────────────
//  InMemoryRepository<T>
// ─────────────────────────────────────────────

/// 基于 `RwLock<HashMap<u64, T>>` 的内存 Repository 实现。
///
/// - 用 `AtomicU64` 分配自增主键。
/// - 读路径用读锁，写路径用写锁。
/// - `find_by_id/for_each` 仍保留借用式 callback API（闭包执行期间持有读锁）。
pub struct InMemoryRepository<T> {
    store: RwLock<HashMap<u64, T>>,
    next_id: AtomicU64,
}

impl<T> InMemoryRepository<T> {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }
}

impl<T> Default for InMemoryRepository<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + Sync> Repository<T> for InMemoryRepository<T> {
    fn save(&self, entity: T) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.store.write().unwrap().insert(id, entity);
        id
    }

    fn update(&self, id: u64, entity: T) -> bool {
        let mut store = self.store.write().unwrap();
        if let std::collections::hash_map::Entry::Occupied(mut e) = store.entry(id) {
            e.insert(entity);
            true
        } else {
            false
        }
    }

    fn find_by_id<R, F: FnOnce(Option<&T>) -> R>(&self, id: u64, f: F) -> R {
        let store = self.store.read().unwrap();
        f(store.get(&id))
    }

    fn for_each<F: FnMut(u64, &T)>(&self, mut f: F) {
        let store = self.store.read().unwrap();
        let mut pairs: Vec<(u64, &T)> = store.iter().map(|(&k, v)| (k, v)).collect();
        pairs.sort_by_key(|(k, _)| *k);
        for (id, val) in pairs {
            f(id, val);
        }
    }

    fn find_all_cloned(&self) -> Vec<(u64, T)>
    where
        T: Clone,
    {
        let store = self.store.read().unwrap();
        let mut pairs: Vec<(u64, T)> = store.iter().map(|(&k, v)| (k, v.clone())).collect();
        pairs.sort_by_key(|(k, _)| *k);
        pairs
    }

    fn delete_by_id(&self, id: u64) -> bool {
        self.store.write().unwrap().remove(&id).is_some()
    }

    fn delete_all(&self) {
        self.store.write().unwrap().clear();
        self.next_id.store(1, Ordering::Relaxed);
    }

    fn count(&self) -> usize {
        self.store.read().unwrap().len()
    }

    fn exists_by_id(&self, id: u64) -> bool {
        self.store.read().unwrap().contains_key(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct User {
        name: String,
        age: u32,
    }

    fn user(name: &str, age: u32) -> User {
        User {
            name: name.to_string(),
            age,
        }
    }

    #[test]
    fn test_save_and_find() {
        let repo: InMemoryRepository<User> = InMemoryRepository::new();
        let id = repo.save(user("Alice", 30));
        assert_eq!(id, 1);
        repo.find_by_id(id, |u| {
            assert_eq!(u.unwrap().name, "Alice");
        });
    }

    #[test]
    fn test_count_and_exists() {
        let repo: InMemoryRepository<User> = InMemoryRepository::new();
        assert_eq!(repo.count(), 0);
        let id = repo.save(user("Bob", 25));
        assert_eq!(repo.count(), 1);
        assert!(repo.exists_by_id(id));
        assert!(!repo.exists_by_id(99));
    }

    #[test]
    fn test_update() {
        let repo: InMemoryRepository<User> = InMemoryRepository::new();
        let id = repo.save(user("Carol", 20));
        assert!(repo.update(id, user("Carol Updated", 21)));
        repo.find_by_id(id, |u| {
            assert_eq!(u.unwrap().name, "Carol Updated");
        });
        assert!(!repo.update(99, user("Ghost", 0)));
    }

    #[test]
    fn test_delete() {
        let repo: InMemoryRepository<User> = InMemoryRepository::new();
        let id = repo.save(user("Dave", 40));
        assert!(repo.delete_by_id(id));
        assert_eq!(repo.count(), 0);
        assert!(!repo.delete_by_id(id));
    }

    #[test]
    fn test_find_all_cloned() {
        let repo: InMemoryRepository<User> = InMemoryRepository::new();
        repo.save(user("Eve", 22));
        repo.save(user("Frank", 33));
        let all = repo.find_all_cloned();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].1.name, "Eve");
        assert_eq!(all[1].1.name, "Frank");
    }

    #[test]
    fn test_delete_all() {
        let repo: InMemoryRepository<User> = InMemoryRepository::new();
        repo.save(user("G", 1));
        repo.save(user("H", 2));
        repo.delete_all();
        assert_eq!(repo.count(), 0);
        // next id resets to 1
        let id = repo.save(user("I", 3));
        assert_eq!(id, 1);
    }
}
