.PHONY: build app run test clean install

build:
	swift build

app:
	Scripts/build-app.sh release

run: app
	open dist/Vigil.app

test:
	swift test

install: app
	rm -rf /Applications/Vigil.app
	cp -R dist/Vigil.app /Applications/Vigil.app
	@echo "Installed to /Applications/Vigil.app"

clean:
	rm -rf .build dist
