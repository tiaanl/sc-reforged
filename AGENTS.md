# sc-reforged agent notes

## Project overview
- Rust (edition 2024) game recreation using `winit` + `wgpu`.
- Requires the original game data directory at runtime.

## Common commands
- Build: `cargo build`
- Run (requires data path): `cargo run -- "C:\Program Files\Sinister Games\Shadow Company - Left for Dead\Data"`
- Run with campaign override: `cargo run -- "<data_path>" <campaign_name>`
- Run without egui (optional): `cargo run --no-default-features -- "<data_path>"`
- Tests (if any): `cargo test`
- Format: `cargo fmt`
- Lint: `cargo clippy`

## Repo layout
- `src/main.rs`: application entry point and main loop.
- `src/engine/`: renderer, input, and scene abstractions.
- `src/game/`: data loading, file system, scenes.
- `docs/`: design notes and file format references.

## Runtime notes
- CLI args are parsed with `clap`; the data path is required.
- Default campaign name is `"training"` if not provided.

## Code style notes
- Use code comments when generating code, and do not mention the conversation that generated the code.
- All public functions must have doc comments.
- Private functions should have doc comments if they are more than 1-2 lines of code.
- Code formatting and linting should always be done if allowed.

## Testing guidance
- Tests are highly recommended, especially for logic.
- Visual/graphics behavior may be exempt, but core logic should be covered.
