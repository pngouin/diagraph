.PHONY: build test frontend

build: frontend
	cargo build

test: frontend
	cargo test

frontend:
	cd frontend && npm install && npm run build
