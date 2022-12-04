use anyhow::Ok;
use axum::async_trait;
use validator::Validate;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use thiserror::Error;

#[derive(Debug, Error)]
enum RepositoryError {
    #[error("Unexpected Error: [{0}]")]
    Unexpected(String),
    #[error("NotFound, id is {0}")]
    NotFound(i32),
}

// Clone, Send, Sync, 'static の多重継承
// axum でこのレポジトリ機能を共有(?)するために layer という機能を使う。layer を利用するためにこれらを継承する必要がある
// ここでの「共有」は単一プロセスの中でシングルトン的に扱いたい、という意味合いと勝手に解釈した
#[async_trait]
pub trait TodoRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo>;
    async fn find(&self, id: i32) -> anyhow::Result<Todo>;
    async fn all(&self) -> anyhow::Result<Vec<Todo>>;
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo>;
    async fn delete(&self, id: i32) -> anyhow::Result<()>;
}

// 以下の Todo に関連する構造体は derive Clone しないと axum の「共有状態」として利用できなくなる(?)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, FromRow)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct UpdateTodo {
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[validate(length(max = 100, message = "over text length"))]
    text: Option<String>,
    completed: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct TodoRepositoryForDb {
    pool: PgPool
}

impl TodoRepositoryForDb {
    pub fn new (pool: PgPool) -> Self {
        TodoRepositoryForDb { pool }
    }
}

#[async_trait]
impl TodoRepository for TodoRepositoryForDb {
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo> {
        let todo = sqlx::query_as::<_, Todo>(
            r#"
            INSERT INTO todos (text, completed)
            VALUES ($1, false)
            RETURNING *;
            "#
        ).bind(payload.text.clone())
        .fetch_one(&self.pool)
        .await?;

        Ok(todo)
    }

    async fn find(&self, id: i32) ->  anyhow::Result<Todo> {
        let todo = sqlx::query_as::<_, Todo>(
            r#"
            SELECT id, text, completed
            FROM todos
            WHERE id = $1;
            "#
        ).bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
            _ => RepositoryError::Unexpected(e.to_string()),
        })?;

        Ok(todo)
    }

    async fn all(&self) -> anyhow::Result<Vec<Todo>> {
        let todos = sqlx::query_as::<_, Todo>(
            r#"
            SELECT * FROM todos
            ORDER BY id DESC;
            "#
        ).fetch_all(&self.pool)
        .await?;

        Ok(todos)
    }

    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
        // bind() で参照する payload のプロパティは Option<T> なので
        // unwrap_or() を使うことで nil なら old_todo の値を渡すことで実質的に対象カラムを更新しないように書ける
        let old_todo = self.find(id).await?;
        let todo = sqlx::query_as::<_, Todo>(
            r#"
            UPDATE todos SET text=$1, completed=$2
            WHERE id=$3
            RETURNING *
            "#
        ).bind(payload.text.unwrap_or(old_todo.text))
        .bind(payload.completed.unwrap_or(old_todo.completed))
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(todo)
    }

    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM todos WHERE id = $1
            "#
        ).bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
            _ => RepositoryError::Unexpected(e.to_string())
        })?;

        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "database-test")]
mod test {
    use super::*;
    use dotenv::dotenv;
    use sqlx::PgPool;
    use std::env;

    #[tokio::test]
    async fn crud_scenario() {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("undefined [DATABASE_URL]");
        let pool = PgPool::connect(&database_url)
            .await
            .expect(&format!("failed to connect database: [{}]", database_url));
        
        // なぜ Clone する必要が？
        let repo = TodoRepositoryForDb::new(pool.clone());
        let todo_text = "[crud_scenario] text";

        // create
        let created = repo
            .create(CreateTodo::new(todo_text.to_string()))
            .await
            .expect("[create] returned Err");
        assert_eq!(created.text, todo_text);
        assert!(!created.completed);

        // find
        let todo = repo.find(created.id).await.expect("[find] returned Err");
        assert_eq!(todo, created);

        // all
        let todos = repo.all().await.expect("[all] returned Err");
        assert_eq!(todos, vec![todo]);

        // update
        let update_text = "[crud_scenario] updated text";
        let todo = repo
            .update(
                created.id,
                UpdateTodo {
                    text: Some(update_text.to_string()),
                    completed: Some(true),
            })
            .await
            .expect("[update] returned Err");
        assert_eq!(created.id, todo.id);
        assert_eq!(todo.text, update_text);

        // delete
        let _ = repo.delete(todo.id).await.expect("[delete] returned Err");
        let res = repo.find(created.id).await;
        assert!(res.is_err());

        let todo_rows = sqlx::query(
            r#"
            SELECT * FROM todos where id = $1
            "#
        ).bind(todo.id)
        .fetch_all(&pool)
        .await
        .expect("[delete] todo_labels fetch error");
        assert!(todo_rows.len() == 0);
    }
}

#[cfg(test)]
pub mod test_utils {
    use anyhow::Context;
    use axum::async_trait;
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}
    };
    use super::*;

    impl Todo {
        pub fn new(id: i32, text: String) -> Self {
            Self {
                id,
                text,
                completed: false,
            }
        }
    }

    #[cfg(test)]
    impl CreateTodo {
        pub fn new(text: String) -> Self {
            Self { text: text }
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

    #[async_trait]
    impl TodoRepository for TodoRepositoryForMemory {
        async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo> {
            let mut store = self.write_store_ref();
            let id = (store.len() + 1) as i32;
            let todo = Todo::new(id, payload.text.clone());
            store.insert(id, todo.clone());
            Ok(todo)
        }

        async fn find(&self, id: i32) -> anyhow::Result<Todo> {
            let store = self.read_store_ref();
            let todo = store
                .get(&id)
                .map(|todo| todo.clone())
                .ok_or(RepositoryError::NotFound(id))?;
            Ok(todo)
        }

        // Note: find() の実装に Box を使うパターン. clone の回数が増えるならヒープの利用を検討する
        // fn find(&self, id: i32) -> Option<Box<Todo>> {
        //     let store = self.read_store_ref();
        //     let todo = store.get(&id);
        //     let todo = Box::new(todo.clone());
        //     Some(Todo)
        // }

        async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
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
            store.insert(id, todo.clone()).unwrap();
            Ok(todo)
        }

        async fn all(&self) -> anyhow::Result<Vec<Todo>> {
            let store = self.read_store_ref();
            let todos = Vec::from_iter(store.values().map(|todo| todo.clone()));
            Ok(todos)
        }

        async fn delete(&self, id: i32) -> anyhow::Result<()> {
            let mut store = self.write_store_ref();
            store.remove(&id).ok_or(RepositoryError::NotFound(id))?;
            Ok(())
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[tokio::test]
        async fn todo_crud_scenario() {
            let text = "todo text".to_string();
            let id = 1;
            let expected = Todo::new(id, text.clone());

            let repo = TodoRepositoryForMemory::new();
            // create
            let todo = repo
                .create(CreateTodo {text})
                .await
                .expect("failed create todo");
            assert_eq!(expected, todo);

            // find
            let todo = repo.find(todo.id).await.unwrap();
            assert_eq!(expected, todo);

            // all
            let todos = repo.all().await.expect("fialed get all todo");
            assert_eq!(vec![expected], todos);

            // update
            let text = "update todo".to_string();
            let todo = repo.update(
                1,
                UpdateTodo {
                    text: Some(text.clone()),
                    completed: Some(true),
                }
            ).await.expect("failed update todo");
            assert_eq!(
                Todo {
                    id,
                    text,
                    completed: true,
                },
                todo
            );

            // delete
            let res = repo.delete(id).await;
            assert!(res.is_ok())
        }
    }
}