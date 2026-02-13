.PHONY: dev dev-rust dev-python

dev:
	./scripts/dev.sh

dev-rust:
	BACKEND_RUNTIME=rust ./scripts/dev.sh

dev-python:
	BACKEND_RUNTIME=python ./scripts/dev.sh
