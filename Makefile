build:
	docker compose build

db:
	docker compose up

dev:
	sqlx db create
	sqlx migrate run
	cargo watch -x run

test:
	cargo test

test-todo:
	cargo test -- repositories::todo::test::crud_scenario

# standalone test
test-s:
	cargo test --no-default-features