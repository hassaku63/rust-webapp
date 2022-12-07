use anyhow::Ok;
use axum::async_trait;
use validator::Validate;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use super::{label::Label, RepositoryError};

// Clone, Send, Sync, 'static の多重継承
// axum でこのレポジトリ機能を共有(?)するために layer という機能を使う。layer を利用するためにこれらを継承する必要がある
// ここでの「共有」は単一プロセスの中でシングルトン的に扱いたい、という意味合いと勝手に解釈した
#[async_trait]
pub trait TodoRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<TodoEntity>;
    async fn find(&self, id: i32) -> anyhow::Result<TodoEntity>;
    async fn all(&self) -> anyhow::Result<Vec<TodoEntity>>;
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<TodoEntity>;
    async fn delete(&self, id: i32) -> anyhow::Result<()>;
}

// 以下の Todo に関連する構造体は derive Clone しないと axum の「共有状態」として利用できなくなる(?)
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct TodoWithLabelFromRow {
    id: i32,
    text: String,
    completed: bool,
    label_id: Option<i32>,
    label_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TodoEntity {
    pub id: i32,
    pub text: String,
    pub completed: bool,
    pub labels: Vec<Label>,
}

fn fold_entities(rows: Vec<TodoWithLabelFromRow>) -> Vec<TodoEntity> {
    let mut rows = rows.iter();
    let mut accum: Vec<TodoEntity> = vec![];
    'outer: while let Some(row) = rows.next() {
        let mut todos = accum.iter_mut();

        while let Some(todo) = todos.next() {
            // id が一致 = Todo に紐づくラベルが複数存在している
            if todo.id == row.id {
                todo.labels.push(Label {
                    id: row.label_id.unwrap(),
                    name: row.label_name.clone().unwrap(),
                });
                continue 'outer;
            }
        }
        
        // Todo の id に一致しない場合のみ到達、TodoEntity を作成
        let labels = if row.label_id.is_some() {
            vec![Label {
                id: row.label_id.unwrap(),
                name: row.label_name.clone().unwrap(),
            }]
        } else {
            vec![]
        };

        accum.push(TodoEntity {
            id: row.id,
            text: row.text.clone(),
            completed: row.completed,
            labels,
        });
    }
    accum
}

fn fold_entity(row: TodoWithLabelFromRow) -> TodoEntity {
    let todo_entities = fold_entities(vec![row]);
    let todo = todo_entities.first().expect("expect 1 todo");

    todo.clone()
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct CreateTodo {
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[validate(length(max = 100, message = "Over text length"))]
    text: String,
    labels: Vec<i32>,
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
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<TodoEntity> {
        let todo = sqlx::query_as::<_, TodoWithLabelFromRow>(
            r#"
            INSERT INTO todos (text, completed)
            VALUES ($1, false)
            RETURNING *;
            "#
        ).bind(payload.text.clone())
        .fetch_one(&self.pool)
        .await?;

        Ok(fold_entity(todo))
    }

    async fn find(&self, id: i32) ->  anyhow::Result<TodoEntity> {
        let todo = sqlx::query_as::<_, TodoWithLabelFromRow>(
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

        Ok(fold_entity(todo))
    }

    async fn all(&self) -> anyhow::Result<Vec<TodoEntity>> {
        let todos = sqlx::query_as::<_, TodoWithLabelFromRow>(
            r#"
            SELECT * FROM todos
            ORDER BY id DESC;
            "#
        ).fetch_all(&self.pool)
        .await?;

        Ok(fold_entities(todos))
    }

    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<TodoEntity> {
        // bind() で参照する payload のプロパティは Option<T> なので
        // unwrap_or() を使うことで nil なら old_todo の値を渡すことで実質的に対象カラムを更新しないように書ける
        let old_todo = self.find(id).await?;
        let todo = sqlx::query_as::<_, TodoWithLabelFromRow>(
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

        Ok(fold_entity(todo))
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
mod test {
    use super::*;
    use dotenv::dotenv;
    use sqlx::PgPool;
    use std::env;

    #[cfg(feature = "database-test")]
    #[tokio::test]
    async fn crud_scenario() {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("undefined [DATABASE_URL]");
        let pool = PgPool::connect(&database_url)
            .await
            .expect(&format!("failed to connect database: [{}]", database_url));
        
        // label data prepare
        let label_name = String::from("test label");
        let optional_label = sqlx::query_as::<_, Label>(
            r#"
            SELECT * FROM labels WHERE name = $1
            "#
        )
        .bind(label_name.clone())
        .fetch_optional(&pool)
        .await
        .expect("failed to prepare label data.");

        let label_1 = if let Some(label) = optional_label {
            // DB にラベルが入ってるならそれを使う
            label
        } else {
            // DB に label_name と同名のラベルが存在しないなら作成
            let label = sqlx::query_as::<_, Label>(
                r#"
                INSERT INTO labels ( name )
                VALUES ( $1 )
                RETURNING *
                "#
            )
            .bind(label_name.clone())
            .fetch_one(&pool)
            .await
            .expect("failed to insert label data.");
            label
        };
        // Memo: この時点では、DB にラベル "test label" が存在する

        let repo = TodoRepositoryForDb::new(pool.clone());
        let todo_text = "[crud_scenario] text";

        // cleanup todo data
        // let todos = repo.all().await.expect("cannot get all todos");
        // for item in todos.iter() {
        //     repo.delete(item.id).await.expect(&format!("failed delete todo: {}", item.id));
        // }

        // create
        let created = repo
            .create(CreateTodo::new(
                todo_text.to_string(),
                vec![label_1.id],
            ))
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

    impl TodoEntity {
        pub fn new(id: i32, text: String) -> Self {
            Self {
                id,
                text,
                completed: false,
                labels: vec![],
            }
        }
    }

    #[cfg(test)]
    impl CreateTodo {
        pub fn new(text: String, labels: Vec<i32>) -> Self {
            Self {
                text: text,
                labels,
            }
        }
    }

    type TodoDatas = HashMap<i32, TodoEntity>;

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
        async fn create(&self, payload: CreateTodo) -> anyhow::Result<TodoEntity> {
            let mut store = self.write_store_ref();
            let id = (store.len() + 1) as i32;
            let todo = TodoEntity::new(id, payload.text.clone());
            store.insert(id, todo.clone());
            Ok(todo)
        }

        async fn find(&self, id: i32) -> anyhow::Result<TodoEntity> {
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

        async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<TodoEntity> {
            let mut store = self.write_store_ref();
            let todo = store
                .get(&id)
                .context(RepositoryError::NotFound(id))?;
            let text = payload.text.unwrap_or(todo.text.clone());
            let completed = payload.completed.unwrap_or(todo.completed);
            let todo = TodoEntity {
                id,
                text,
                completed,
                labels: vec![],
            };
            store.insert(id, todo.clone()).unwrap();
            Ok(todo)
        }

        async fn all(&self) -> anyhow::Result<Vec<TodoEntity>> {
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

        #[test]
        fn fold_entities_test() {
            let label_1 = Label {
                id: 1,
                name: String::from("label 1"),
            };
            let label_2 = Label {
                id: 2,
                name: String::from("label 2"),
            };
            let rows = vec![
                TodoWithLabelFromRow {
                    id: 1,
                    text: String::from("todo 1"),
                    completed: false,
                    label_id: Some(label_1.id),
                    label_name: Some(label_1.name.clone()),
                },
                TodoWithLabelFromRow {
                    id: 1,
                    text: String::from("todo 1"),
                    completed: false,
                    label_id: Some(label_2.id),
                    label_name: Some(label_2.name.clone()),
                },
                TodoWithLabelFromRow {
                    id: 2,
                    text: String::from("todo 2"),
                    completed: false,
                    label_id: Some(label_1.id),
                    label_name: Some(label_1.name.clone()),
                },
            ];
    
            let res = fold_entities(rows);
            assert_eq!(
                res,
                vec![
                    TodoEntity {
                        id: 1,
                        text: String::from("todo 1"),
                        completed: false,
                        labels: vec![label_1.clone(), label_2.clone()],
                    },
                    TodoEntity {
                        id: 2,
                        text: String::from("todo 2"),
                        completed: false,
                        labels: vec![label_1.clone()],
                    },
                ]
            )
        }

        #[tokio::test]
        async fn todo_crud_scenario() {
            let text = "todo text".to_string();
            let id = 1;
            let expected = TodoEntity::new(id, text.clone());

            // create
            // todo!("ラベルデータの追加");
            let labels = vec![];
            let repo = TodoRepositoryForMemory::new();
            let todo = repo
                .create(CreateTodo::new(text, labels))
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
                TodoEntity {
                    id,
                    text,
                    completed: true,
                    labels: vec![],
                },
                todo
            );

            // delete
            let res = repo.delete(id).await;
            assert!(res.is_ok())
        }
    }
}