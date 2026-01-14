#### The Elm Architecture (Iced)

Instead of GTK signal callbacks, we use the **Model-View-Update** pattern.

1. **Message**: An enum defining all possible events (e.g., `ScanPressed`, `ScanCompleted`).
2. **Update**: A pure function that takes the current state and a message, returning a new state and potentially a Command.
3. **View**: A pure function that renders the UI based on the current state.

**Why?**
This eliminates the need for `Rc<RefCell<T>>` or weak references for basic UI logic, making the application thread-safe and easier to reason about.

---

### 4. `cd_reader.rs` – CD Detection & Metadata

#### Core Structure

```rust
pub struct CdInfo {
    pub title: String,
    pub artist: String,
    pub tracks: Vec<String>,
    pub disc_id: String,
}

pub struct CdReader;  // Unit struct – no state, just associated functions
```