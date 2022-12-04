CREATE TABLE labels (
    id   SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE todo_labels (
    id       SERIAL PRIMARY KEY,
    -- Point: 
    --   DEFERRABLE INITIALLY DEFERRED を宣言することで、
    --   トランザクションのコミット時まで「外部キー制約の適用」を遅延させる
    todo_id  INTEGER NOT NULL REFERENCES todos (id) DEFERRABLE INITIALLY DEFERRED,
    label_id INTEGER NOT NULL REFERENCES labels (id) DEFERRABLE INITIALLY DEFERRED
);
