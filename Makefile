.PHONY: build
build: ## สร้าง library และ binaries / Build library and binaries
	cargo build

.PHONY: build-release
build-release: ## สร้าง release build / Build optimized release binaries
	cargo build --release

.PHONY: test
test: ## รันทดสอบทั้งหมด / Run all tests
	cargo test

.PHONY: demo-cli
demo-cli: ## รัน CLI demo พร้อมรูปตัวอย่าง / Run CLI demo with sample images
	./scripts/demo-cli.sh
	open certificate-cli.pdf

.PHONY: demo-rest
demo-rest: ## รัน REST demo ทั้ง 3 strategies / Run REST demo for all 3 strategies
	./scripts/demo-rest.sh

.PHONY: clean
clean: ## ลบไฟล์ PDF ที่สร้างจาก demo / Remove generated demo PDFs
	rm -f certificate-cli.pdf certificate-multipart.pdf certificate-base64.pdf certificate-url.pdf certificate.pdf

.PHONY: help
help:
	@grep -E '^[a-zA-Z_-]+:.*##' Makefile | awk 'BEGIN {FS = ":.*##"}; {printf "  %-15s %s\n", $$1, $$2}'
