# sc-reforged agent notes

## Project overview
- Rust (edition 2024) game recreation.
- Requires the original game data directory at runtime.

## Common commands
- Build: `cargo build`
- Run (requires data path): `cargo run -- "C:\Program Files\Sinister Games\Shadow Company - Left for Dead\Data"`
- Run with campaign override: `cargo run -- "<data_path>" <campaign_name>`
- Tests (if any): `cargo test`
- Format: `cargo fmt`
- Lint: `cargo clippy`

## Runtime notes
- CLI args are parsed with `clap`; the data path is required.
- Default campaign name is `"training"` if not provided.

## Code style notes
- Use code comments when generating code. Do not mention the conversation or internal implementation details; include only what users of the code need to know.
- All public functions must have doc comments.
- Private functions should have doc comments if they are more than 1-2 lines of code.
- Code formatting and linting should always be done if allowed.

## Testing guidance
- Tests are highly recommended, especially for logic.
- Visual/graphics behavior may be exempt, but core logic should be covered.
