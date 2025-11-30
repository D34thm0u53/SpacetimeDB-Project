---
applyTo: '**.rs'
---
Coding standards, domain knowledge, and preferences that AI should follow.

# Function formatting
- Functions should be formatted with a single blank line before the function definition.
- Function names should be in `snake_case`.
- Function parameters should be named descriptively, using `snake_case`.
- Function parameters should be limited to 3-4 parameters. If more are needed, consider using a struct or tuple.
- Function bodies should be indented with 4 spaces.
- Use `match` statements for control flow when appropriate, especially for enums.

# Comments
- Use comments to explain complex logic or important decisions.
- Use `///` for documentation comments that describe the purpose and usage of functions, structs, and enums.
- Use `//` for inline comments to clarify specific lines of code.
- Avoid unnecessary comments that restate what the code does.
- Provide a brief summary of the function's purpose at the top of the function body, when the function is complex.

# Variable naming
- Variable names should be in `snake_case`.
- Use descriptive names that convey the purpose of the variable.
- Avoid single-letter variable names except for loop indices.

# Wrappers
- Use wrapper functions for repetitive tasks to improve code readability and maintainability.
- Ensure wrapper functions have clear and descriptive names that indicate their purpose.
- Wrapper functions should handle error checking and logging consistently.
- Wrapper functions are denoted by a `wrap_` prefix in their names.

# Error handling
- Use `Result<T, E>` for functions that can fail, with appropriate error types.
- Use `?` operator for propagating errors when appropriate.
- Log errors with sufficient context to aid in debugging.
- Avoid using `unwrap()` or `expect()` unless absolutely certain the value is valid, even then try and avoid use.

