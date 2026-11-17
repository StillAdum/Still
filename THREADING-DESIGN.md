# Threading Design for Still

## Problem
Still's refcounted runtime uses non-atomic reference counts. This is fast but unsafe across threads:
- Two threads incrementing/decrementing the same refcount can race
- A value freed on one thread while another thread holds a reference is UB

## Design Options

### Option A: Atomic refcounts (simple, slow)
- Change `still_hdr.rc` from `int64_t` to `atomic_int64_t`
- Use `atomic_fetch_add`/`atomic_fetch_sub` for retain/release
- Cost: ~10-50x slower on hot paths (atomic operations are expensive)
- Pro: Simple, no API changes
- Con: Defeats "runs like C" goal

### Option B: Values moved between threads + per-thread arenas (recommended)
- Thread-local ownership: each value has a single owner thread
- Moving a value to another thread transfers ownership (atomic handoff)
- Per-thread arenas for allocation (no contention)
- Shared values require explicit `shared[T]` type with atomic refcounts
- Pro: Fast single-threaded (non-atomic), safe cross-thread
- Con: Requires new type system concept (`shared[T]`)

### Option C: Region-based (most performant)
- Values are allocated in regions owned by a thread
- Moving a value moves its region
- Shared regions use atomic refcounts
- Pro: Best performance, predictable memory
- Con: Complex, changes ownership model

## Recommended Approach: Option B

### New Type: `shared[T]`
- `shared[T]` is a reference-counted value that can be shared across threads
- Uses atomic refcounts (like Arc in Rust)
- Only `shared[T]` types can cross thread boundaries
- Regular types (`str`, `[T]`, structs) are thread-local

### Thread API
```
func thread_spawn(f: func() -> none) -> thread
func thread_join(t: thread) -> none
func thread_channel[T]() -> (sender[T], receiver[T])
```

### Memory Model
- Thread-local values: non-atomic refcounts (fast)
- Shared values: atomic refcounts (Arc-like)
- Moving a thread-local value to another thread: deep copy or ownership transfer
- Channels: move values between threads (ownership transfer)

### Implementation Phases
1. Design doc (this document) ✅
2. Atomic refcount runtime (still_rt_atomic.h)
3. `shared[T]` type in checker
4. Thread spawn/join in runtime
5. Channel implementation
6. Conformance tests

### Performance Impact
- Single-threaded code: no change (non-atomic refcounts)
- Multi-threaded code: atomic refcounts only for `shared[T]`
- Channel operations: O(1) with ownership transfer (no deep copy)

### Open Questions
- Should `shared[T]` be covariant? (probably not, for safety)
- How to handle closures capturing shared values?
- Thread pool vs. raw threads?
- Async/await on top of threads?