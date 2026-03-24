# Agent Instructions

## Development Workflow

* This is a Rust project, always use `cargo clippy` and `cargo test` and fix issues.
* Use `./build.sh` to build the WASM build. Always update it when iterating.
* Always commit the code when the app builds and tests and linting is green.

## Agent Behavior

* If there is any ambiguity in the interpretation of a user request, ask the user. Do not assume anything if you are not sure or need more details.
* Never guess. Think through issues, read documentation and existing code.
* If documentation is missing, try installing it or ask the user for help.

## Code Quality

* Always try writing generic, modular, maintainable code.
* Prioritize good architecture and clarity over raw performance.

* Write small functions that ideally fit on a single screen.
* Avoid deep nesting, use more helper functions instead.

* Avoid data duplication, all constants like numeric values and strings and use the constant variable.
* Avoid code duplication, factor out a common function instead
* Before writing any new non-trivial code, check whether similar functionality already exists.
* Always try to write code that can be cleanly reused in a different context.

* Write testable code with mockable inputs, write tests for everything except e.g. UI code.
* In tests, use an adversary mindset. Test typical edge cases and try covering all code paths.
