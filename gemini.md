# Todoism Architecture & Design Principles

## 1. Core Philosophy
*   **Domain-Driven Design (DDD) Lite**: The core logic resides in the `core` crate, separated from the presentation layer (`cli`).
*   **Type Safety**: Leverage Rust's type system (Enums) to represent invalid states as unrepresentable types.
*   **CQRS-ish**: Separate the read model (DTOs for UI) from the write model (Domain Entities for logic).


## 3. Service Layer & DTOs
### `TaskService`
*   Acts as the boundary between the Core and the UI.
*   **Write Operations**: Accept primitive inputs or commands, load Entities, execute methods, and save.
*   **Read Operations**: Return `TaskDto`, NOT Entities.

### `TaskDto` (Read Model)
A flattened, read-only projection of the `Task` entity, optimized for UI rendering.

```rust
// core/src/service/dto.rs

pub struct TaskDto {
    pub id: Uuid,
    pub name: String,
    pub status_label: String, // e.g., "Pending", "Completed"
    pub is_tracking: bool,
    pub total_time_spent: u64, // Calculated on the fly for Pending, fixed for Completed
    pub project: String,
    // ...
}
```

## 4. UI Layer (CLI/TUI)
*   **Passive View**: The TUI should not contain business logic.
*   **No Entity Access**: The UI never touches `Task` directly. It only renders `TaskDto`.
*   **Actions**: User actions (e.g., "Complete Task") are forwarded to `TaskService` methods.

## 5. Development Guidelines
*   **Refactoring**: When changing the state model, always update the DTO mapping.
*   **Testing**: Test state transitions in `Task` unit tests. Test mapping logic in `TaskService`.
