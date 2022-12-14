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


#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct TodoFromRow {
    id: i32,
    text: String,
    completed: bool,
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
    let mut result: Vec<TodoEntity> = vec![];
    'outer: while let Some(row) = rows.next() {
        let mut todos = result.iter_mut();

        while let Some(todo) = todos.next() {
            // todo:label の N:N 関係を第一正規形展開したものを受けとるので、
            // rows の中で同じ ID を持つ Todo は存在し得る
            // TodoEntity 的には自身 (Todo) に紐づく Label を配列でまとめて保持する定義なので、
            // 同じ Todo ID に属す Label は同じ Entity インスタンスに集約したい、という処理
            if todo.id == row.id {
                // この todo は result の要素を可変参照で見るデータなので、
                // todo に対する破壊的操作は result を更新することに注意
                todo.labels.push(Label {
                    id: row.label_id.unwrap(),
                    name: row.label_name.clone().unwrap(),
                });
                continue 'outer;
            }
        }
        
        // 手前の while を抜けているので、この時点では
        // 今の outer ループで扱っている row の Todo ID は
        // 今の Vec<TodoEntity> の中に存在してない Todo である、と言える
        // なので、このコメント以下でやるべき仕事は新しい TodoEntity を作って push すること。
        // TodoEntity の Todo ID は row が持っているそれ。
        // 
        // 実際に DB に入ってるデータとそのクエリ方法の想定として
        // 交差テーブルを使っての outer join を行うので、
        // row.label_id は Optional 型となることに注意。
        // ラベルの有無に関わらず join していくクエリを書くので、label_id は Optional となる。
        let labels = if row.label_id.is_some() {
            vec![Label {
                id: row.label_id.unwrap(),
                name: row.label_name.clone().unwrap(),
            }]
        } else {
            vec![]
        };

        result.push(TodoEntity {
            id: row.id,
            text: row.text.clone(),
            completed: row.completed,
            labels,
        });
    }
    result
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
    labels: Option<Vec<i32>>,
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
        let tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, TodoFromRow>(
            r#"
            INSERT INTO todos (text, completed)
            VALUES ($1, false)
            RETURNING *
            "#
        ).bind(payload.text.clone())
        .fetch_one(&self.pool)
        .await?;
        
        // この SQL 文は、bind した配列を展開したら例えばこうなる
        // INSERT INTO todo_labels (todo_id, label_id)
        // SELECT 1, id
        // FROM unnest(array[1, 2, 3]) as t(id) 
        sqlx::query(
            r#"
            INSERT INTO todo_labels (todo_id, label_id)
            SELECT $1, id
            FROM unnest($2) as t(id);
            "#
        )
        .bind(row.id)
        .bind(payload.labels)
        .execute(&self.pool)
        .await?;

        tx.commit().await?;

        let todo = self.find(row.id).await?;
        Ok(todo)
    }

    async fn find(&self, id: i32) ->  anyhow::Result<TodoEntity> {
        let items = sqlx::query_as::<_, TodoWithLabelFromRow>(
            r#"
            SELECT todos.*, labels.id label_id, labels.name label_name
            FROM todos
            LEFT OUTER JOIN todo_labels tl on todos.id = tl.todo_id
            LEFT OUTER JOIN labels on labels.id = tl.label_id
            WHERE todos.id=$1
            "#  
        ).
        bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
            _ => RepositoryError::Unexpected(e.to_string()),
        })?;

        let todos = fold_entities(items);
        let todo = todos.first().ok_or(RepositoryError::NotFound(id))?;
        Ok(todo.clone())
    }

    async fn all(&self) -> anyhow::Result<Vec<TodoEntity>> {
        let todos = sqlx::query_as::<_, TodoWithLabelFromRow>(
            r#"
            SELECT todos.*, labels.id as label_id, labels.name as label_name
            FROM todos
                LEFT OUTER JOIN todo_labels tl on todos.id = tl.todo_id
                LEFT OUTER JOIN labels on labels.id = tl.label_id
            ORDER BY todos.id DESC
            "#
        ).fetch_all(&self.pool)
        .await?;

        Ok(fold_entities(todos))
    }

    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<TodoEntity> {
        let tx = self.pool.begin().await?;
        
        let old_todo = self.find(id).await?;
        sqlx::query_as::<_, TodoFromRow>(
            r#"
            UPDATE todos SET text=$1, completed=$2
            WHERE id=$3
            RETURNING *
            "#
        )
        .bind(payload.text.unwrap_or(old_todo.text))
        .bind(payload.completed.unwrap_or(old_todo.completed))
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        // payload が labels を持っているなら交差テーブル todo_labels を更新
        if let Some(labels) = payload.labels {
            // いったん削除
            sqlx::query(
                r#"
                DELETE FROM todo_labels WHERE todo_id = $1
                "#
            )
            .bind(id)
            .execute(&self.pool)
            .await?;

            // 新しい label ids を insert
            sqlx::query(
                r#"
                INSERT INTO todo_labels (todo_id, label_id)
                SELECT $1, id as label_id
                FROM unnest($2) as t(id);
                "#
            )
            .bind(id)
            .bind(labels)
            .execute(&self.pool)
            .await?;
        }

        tx.commit().await?;
        let todo = self.find(id).await?;

        Ok(todo)
    }

    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        let tx = self.pool.begin().await?;

        // 中間テーブルの関係を外す
        sqlx::query(
            r#"
            DELETE FROM todo_labels WHERE todo_id = $1
            "#
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
            _ => RepositoryError::Unexpected(e.to_string()),
        })?;

        // todo の削除
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

        tx.commit().await?;
        
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
        // Note: Label レポジトリのテストデータと同じ名前だと2回目以降のテストが通らない.
        //        このことから、複数スレッド or 複数クライアントを想定したシナリオが漏れている
        //        脆弱なテストと言えるのではないか?
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
            .bind(label_name)
            .fetch_one(&pool)
            .await
            .expect("failed to insert label data.");
            label
        };
        // Memo: この時点では、DB にラベル "test label" が存在する

        let repo = TodoRepositoryForDb::new(pool.clone());
        let todo_text = "[crud_scenario] text";

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
        assert_eq!(*created.labels.first().unwrap(), label_1);

        // find
        let todo = repo.find(created.id).await.expect("[find] returned Err");
        assert_eq!(todo, created);

        // all
        let todos = repo.all().await.expect("[all] returned Err");
        // assert_eq!(todos, vec![todo]);
        let todo = todos.first().unwrap();
        assert_eq!(created, *todo);

        // update
        let update_text = "[crud_scenario] updated text";
        let todo = repo
            .update(
                todo.id,
                UpdateTodo {
                    text: Some(update_text.to_string()),
                    completed: Some(true),
                    labels: Some(vec![]),
                },
            )
            .await
            .expect("[update] returned Err");
        assert_eq!(created.id, todo.id);
        assert_eq!(todo.text, update_text);
        assert!(todo.labels.len() == 0);

        // delete
        let _ = repo
            .delete(todo.id)
            .await
            .expect("[delete] returned Err");
        let res = repo.find(created.id).await;
        assert!(res.is_err());

        let todo_rows = sqlx::query(
            r#"
            SELECT * FROM todos where id = $1
            "#
        )
        .bind(todo.id)
        .fetch_all(&pool)
        .await
        .expect("[delete] todo_labels fetch error");
        assert!(todo_rows.len() == 0);

        let rows = sqlx::query(
            r#"
            SELECT * FROM todo_labels WHERE todo_id = $1
            "#
        )
        .bind(todo.id)
        .fetch_all(&pool)
        .await
        .expect("[delete] todo_labels fect error");
        assert!(rows.len() == 0);
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
                    labels: Some(vec![]),
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