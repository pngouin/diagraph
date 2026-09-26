.PHONY: build test frontend fuzz

build: frontend
	cargo build

test: frontend
	cargo test

frontend:
	cd frontend && npm install && npm run build

fuzz:
	cd frontend && npm run fuzz
