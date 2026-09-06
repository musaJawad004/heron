.PHONY: build app run test clean install

build:
	swift build

app:
	Scripts/build-app.sh release

run: app
	open dist/Heron.app

test:
	swift test

install: app
	rm -rf /Applications/Heron.app
	cp -R dist/Heron.app /Applications/Heron.app
	@echo "Installed to /Applications/Heron.app"

clean:
	rm -rf .build dist
