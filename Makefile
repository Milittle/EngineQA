.PHONY: dev dev-rust dev-python package-rust-backend package-python-backend

dev:
	./scripts/dev.sh

dev-rust:
	BACKEND_RUNTIME=rust ./scripts/dev.sh

dev-python:
	BACKEND_RUNTIME=python ./scripts/dev.sh

package-rust-backend:
	./scripts/package/build-rust-backend.sh

package-python-backend:
	./scripts/package/build-python-backend.sh
