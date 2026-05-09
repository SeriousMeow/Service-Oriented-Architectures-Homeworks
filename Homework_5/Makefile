SHELL := /usr/bin/env bash

.PHONY: up down test-integration test-all clean

up:
	docker compose up -d --build

down:
	docker compose down

test-integration:
	docker compose up --build --abort-on-container-exit --exit-code-from integration-test integration-test

test-all:
	docker compose up --build --abort-on-container-exit --exit-code-from all-test all-test

clean:
	docker compose down -v
