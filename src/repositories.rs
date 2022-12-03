use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
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
    fn update(&self, id: i32, paylaod: UpdateTodo) -> anyhow::Result<Todo>;
    fn delete(&self, id: i32) -> anyhow::Result<()>;
}

// 以下の Todo に関連する構造体は derive Clone しないと axum の「共有状態」として利用できなくなる(?)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Todo {
    id: i32,
    text: String,
    complated: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CreateTodo {
    text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct UpdateTodo {
    text: Option<String>,
    completed: Option<bool>,
}

impl Todo {
    pub fn new(id: i32, text: String) -> Self {
        Self {
            id,
            text,
            complated: false,
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
}

impl TodoRepository for TodoRepositoryForMemory {
    fn create(&self, payload: CreateTodo) -> Todo {
        todo!()
    }

    fn find(&self, id: i32) -> Option<Todo> {
        todo!()
    }

    fn update(&self, id: i32, paylaod: UpdateTodo) -> anyhow::Result<Todo> {
        todo!()
    }

    fn all(&self) -> Vec<Todo> {
        todo!()
    }

    fn delete(&self, id: i32) -> anyhow::Result<()> {
        todo!()
    }
}
