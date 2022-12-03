use anyhow::Context;
use validator::Validate;
use std::{
    collections::{HashMap},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
enum RepositoryError {
    #[error("NotFound, id is {0}")]
    NotFound(i32),
}

// Clone, Send, Sync, 'static の多重継承
// axum でこのレポジトリ機能を共有(?)するために layer という機能を使う。layer を利用するためにこれらを継承する必要がある
// ここでの「共有」は単一プロセスの中でシングルトン的に扱いたい、という意味合いと勝手に解釈した
pub trait TodoRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    fn create(&self, payload: CreateTodo) -> Todo;
    fn find(&self, id: i32) -> Option<Todo>;
    fn all(&self) -> Vec<Todo>;
    fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo>;
    fn delete(&self, id: i32) -> anyhow::Result<()>;
}

// 以下の Todo に関連する構造体は derive Clone しないと axum の「共有状態」として利用できなくなる(?)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Todo {
    id: i32,
    text: String,
    completed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct CreateTodo {
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[validate(length(max = 100, message = "Over text length"))]
    text: String,
}

#[cfg(test)]
impl CreateTodo {
    pub fn new(text: String) -> Self {
        Self { text: text }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct UpdateTodo {
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[validate(length(max = 100, message = "over text length"))]
    text: Option<String>,
    completed: Option<bool>,
}

impl Todo {
    pub fn new(id: i32, text: String) -> Self {
        Self {
            id,
            text,
            completed: false,
        }
    }
}

type TodoDatas = HashMap<i32, Todo>;

#[derive(Debug, Clone)]
pub struct TodoRepositoryForMemory {
    // 複数スレッドからのアクセスを想定し Arc<RwLock<>> でスレッドセーフにする
    // 不変参照の場合は複数スレッドで共有できるが、可変参照の場合はスレッドを1つに制限する
    store: Arc<RwLock<TodoDatas>>,
}

impl TodoRepositoryForMemory {
    pub fn new() -> Self {
        TodoRepositoryForMemory {
            store: Arc::default(),
        }
    }

    fn write_store_ref(&self) -> RwLockWriteGuard<TodoDatas> {
        self.store.write().unwrap()
    }

    fn read_store_ref(&self) -> RwLockReadGuard<TodoDatas> {
        self.store.read().unwrap()
    }
}

impl TodoRepository for TodoRepositoryForMemory {
    fn create(&self, payload: CreateTodo) -> Todo {
        let mut store = self.write_store_ref();
        let id = (store.len() + 1) as i32;
        let todo = Todo::new(id, payload.text.clone());
        store.insert(id, todo.clone());
        todo
    }

    fn find(&self, id: i32) -> Option<Todo> {
        let store = self.read_store_ref();
        store.get(&id).map(|todo| todo.clone())
    }

    // Note: find() の実装に Box を使うパターン. clone の回数が増えるならヒープの利用を検討する
    // fn find(&self, id: i32) -> Option<Box<Todo>> {
    //     let store = self.read_store_ref();
    //     let todo = store.get(&id);
    //     let todo = Box::new(todo.clone());
    //     Some(Todo)
    // }

    fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
        let mut store = self.write_store_ref();
        let todo = store
            .get(&id)
            .context(RepositoryError::NotFound(id))?;
        let text = payload.text.unwrap_or(todo.text.clone());
        let completed = payload.completed.unwrap_or(todo.completed);
        let todo = Todo {
            id,
            text,
            completed,
        };
        store.insert(id, todo.clone());
        Ok(todo)
    }

    fn all(&self) -> Vec<Todo> {
        let store = self.read_store_ref();
        Vec::from_iter(store.values().map(|todo| todo.clone()))
    }

    fn delete(&self, id: i32) -> anyhow::Result<()> {
        let mut store = self.write_store_ref();
        store.remove(&id).ok_or(RepositoryError::NotFound(id))?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn todo_crud_scenario() {
        let text = "todo text".to_string();
        let id = 1;
        let expected = Todo::new(id, text.clone());

        let repo = TodoRepositoryForMemory::new();
        // create
        let todo = repo.create(CreateTodo {text});
        assert_eq!(expected, todo);

        // find
        let todo = repo.find(todo.id).unwrap();
        assert_eq!(expected, todo);

        // all
        let todos = repo.all();
        assert_eq!(vec![expected], todos);

        // update
        let text = "update todo".to_string();
        let todo = repo.update(
            1,
            UpdateTodo {
                text: Some(text.clone()),
                completed: Some(true),
            }
        ).expect("failed update todo");
        assert_eq!(
            Todo {
                id,
                text,
                completed: true,
            },
            todo
        );

        // delete
        let res = repo.delete(id);
        assert!(res.is_ok())
    }
}