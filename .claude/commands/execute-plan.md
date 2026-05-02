Read `agent-task/PLAN.md` and implement the plan described in it.

Follow these rules throughout implementation:

1. **Read the plan first.** Understand the goal, steps, constraints, and definition of done before writing any code.

2. **Implement step by step.** Work through each step in order. Mark steps complete as you finish them (do not batch completions).

3. **Keep the code compiling.** After each meaningful change, run `cargo build` (or `cargo check` for speed) to verify the project compiles. Do not proceed to the next step if there are compile errors — fix them first.

4. **Document all items in Rust doc format.** Every function, struct, enum, trait, and module must have a `///` doc comment explaining its purpose. Include parameter and return descriptions where non-obvious. Use `# Examples` sections for non-trivial public APIs.

5. **Do not over-engineer.** Implement only what the plan describes. Avoid adding abstractions, error handling, or features beyond what is required.

6. **When done**, confirm the project compiles cleanly with `cargo build` and that all items in the definition of done are satisfied.
