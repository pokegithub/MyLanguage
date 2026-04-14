# CoVibe Language Specification
## Part 5: Concurrency Model and Runtime Semantics

**Version:** 1.0
**Date:** 2026-04-14
**Status:** Final

---

## Table of Contents

1. [Introduction](#introduction)
2. [Task Model (Green Threads)](#task-model-green-threads)
3. [Spawn Semantics](#spawn-semantics)
4. [Channel Types and Operations](#channel-types-and-operations)
5. [Select Statement](#select-statement)
6. [Async/Await Desugaring](#asyncawait-desugaring)
7. [Structured Concurrency](#structured-concurrency)
8. [Scheduler Contract](#scheduler-contract)
9. [Atomic Types and Memory Ordering](#atomic-types-and-memory-ordering)
10. [Thread-Local Storage](#thread-local-storage)
11. [Parallel Iteration](#parallel-iteration)
12. [Data Race Freedom Guarantee](#data-race-freedom-guarantee)
13. [Actor Model Module](#actor-model-module)

---

## 1. Introduction

CoVibe provides a comprehensive concurrency model that combines the simplicity of Go's goroutines and channels with Rust's memory safety guarantees and the expressiveness of async/await syntax. The concurrency system is designed to be:

- **Safe by default**: Data races are prevented at compile time through the ownership system
- **Lightweight**: Tasks are green threads scheduled across a pool of OS threads with minimal overhead
- **Ergonomic**: Simple spawn syntax, clean async/await sugar, and structured concurrency primitives
- **Scalable**: Work-stealing scheduler efficiently distributes tasks across available cores
- **Composable**: Multiple concurrency patterns (tasks, async/await, channels, parallel iteration) work seamlessly together

### 1.1 Core Concurrency Primitives

CoVibe provides four primary concurrency abstractions:

1. **Tasks**: Lightweight green threads managed by an M:N scheduler
2. **Channels**: Type-safe message passing between tasks
3. **Async/Await**: High-level syntax that desugars to the task model
4. **Parallel Iteration**: Data parallelism for collection processing

All primitives are built on the same underlying runtime and respect the same memory safety guarantees.

### 1.2 Memory Safety in Concurrent Code

The ownership and borrow checking system extends naturally to concurrent code:

- **Send Trait**: Types that can be safely transferred between tasks
- **Sync Trait**: Types that can be safely shared (via references) between tasks
- **Compile-time Data Race Prevention**: The borrow checker ensures no two tasks can mutate the same data simultaneously without synchronization
- **Channel Ownership Transfer**: Sending a value through a channel transfers ownership, preventing aliasing

---

## 2. Task Model (Green Threads)

CoVibe uses an M:N threading model where M user-level tasks (green threads or fibers) are multiplexed onto N OS threads. Tasks are lightweight and can be created by the millions without exhausting system resources.

### 2.1 Task Characteristics

A task in CoVibe has the following properties:

- **Independent Stack**: Each task has its own stack (default 4KB, growable up to a limit)
- **Local State**: Tasks have their own execution context (instruction pointer, stack pointer, registers)
- **Scheduled Cooperatively**: Tasks yield at specific yield points (await, channel operations, explicit yield)
- **Heap Allocated**: Task metadata is stored on the heap, but the task's local variables live on its stack
- **Non-Preemptive by Default**: Tasks run until they yield (except for CPU time limit enforcement)

### 2.2 Task States

A task transitions through the following states during its lifetime:

```
Created → Ready → Running → (Blocked | Completed)
          ↑         ↓
          └─────────┘
```

- **Created**: Task has been allocated but not yet scheduled
- **Ready**: Task is in a scheduler queue waiting to run
- **Running**: Task is currently executing on an OS thread
- **Blocked**: Task is waiting for I/O, a channel operation, or synchronization
- **Completed**: Task has finished execution

### 2.3 Task Representation

Conceptually, a task is represented as:

```covibe
struct Task {
    id: TaskId,                    // Unique task identifier
    stack: Stack,                  // Task's execution stack
    state: TaskState,              // Current state
    context: Context,              // Saved registers (when not running)
    parent: Option<TaskId>,        // For structured concurrency
    result: Option<Result<T, E>>, // Task's return value
}
```

### 2.4 Task Local Variables

Task-local state (function parameters, local variables) lives on the task's stack and is accessible only to that task. Moving or borrowing task-local state follows the same ownership rules as non-concurrent code.

### 2.5 Task Cleanup

When a task completes:
1. Its return value is stored in the task metadata
2. Its stack is deallocated
3. All RAII destructors run in reverse order of construction
4. Parent task is notified (in structured concurrency)
5. Task metadata is retained until all JoinHandles are dropped

---

## 3. Spawn Semantics

The `spawn` keyword creates a new task and schedules it for execution.

### 3.1 Basic Spawn Syntax

```covibe
spawn expression
```

The `expression` must be a closure or function call. The spawn expression returns a `JoinHandle<T>` where `T` is the return type of the task.

**Example:**

```covibe
let handle = spawn {
    compute_heavy_task()
}

let result = handle.join()  // Wait for task to complete and get result
```

### 3.2 Spawn with Arguments

```covibe
fn process(data: String, count: Int) -> Int {
    // ... processing logic
    data.len() * count
}

let data = "hello".to_owned()
let handle = spawn process(data, 42)
```

### 3.3 Ownership Transfer in Spawn

When spawning a task, ownership of captured variables is transferred to the new task:

```covibe
let message = "Hello from task".to_owned()

spawn {
    println(message)  // message is moved into the task
}

// println(message)  // ERROR: message was moved
```

### 3.4 Shared State via Channels

To share state between tasks, use channels or shared synchronization primitives:

```covibe
let (tx, rx) = channel::<String>()

spawn {
    tx.send("result from task")
}

let msg = rx.recv()
println(msg)
```

### 3.5 JoinHandle Type

`JoinHandle<T>` is the handle returned by spawn:

```covibe
struct JoinHandle<T> {
    task_id: TaskId,
    // Internal fields
}

impl<T> JoinHandle<T> {
    // Block until task completes and return its result
    fn join(self) -> T

    // Try to get result without blocking (returns None if not ready)
    fn try_join(&mut self) -> Option<T>

    // Detach from task (task continues running but result is discarded)
    fn detach(self)
}
```

### 3.6 Spawn Constraints

The closure or function passed to spawn must satisfy:
- All captured values must implement `Send` (can be safely sent to another task)
- The return type `T` must implement `Send`

```covibe
struct NotSend {
    ptr: *const Int  // Raw pointers are not Send
}

let x = NotSend { ptr: &42 as *const Int }

spawn {
    println(x)  // ERROR: NotSend does not implement Send
}
```

### 3.7 Task Panics

If a task panics:
- The panic does not propagate to other tasks
- The panic is captured and stored in the JoinHandle
- Calling `join()` on a panicked task propagates the panic to the caller
- Detached tasks that panic print an error and terminate silently

```covibe
let handle = spawn {
    panic("task failed")
}

// handle.join()  // This would panic with "task failed"
```

---

## 4. Channel Types and Operations

Channels provide type-safe, synchronized message passing between tasks. CoVibe supports both unbuffered (synchronous) and buffered (asynchronous) channels.

### 4.1 Channel Types

```covibe
struct Sender<T> {
    // Internal implementation
}

struct Receiver<T> {
    // Internal implementation
}
```

Channels are created with the `channel` function:

```covibe
// Unbuffered channel (synchronous)
let (tx, rx) = channel::<Int>()

// Buffered channel with capacity N
let (tx, rx) = channel_buffered::<String>(capacity: 10)
```

### 4.2 Send Operation

```covibe
impl<T> Sender<T> {
    // Send a value, blocking if channel is full
    fn send(self, value: T) -> Result<(), SendError<T>>

    // Try to send without blocking
    fn try_send(self, value: T) -> Result<(), TrySendError<T>>
}
```

**Blocking behavior:**
- Unbuffered: `send()` blocks until a receiver calls `recv()`
- Buffered: `send()` blocks only if buffer is full

**Ownership transfer:**
Sending transfers ownership of the value to the channel:

```covibe
let (tx, rx) = channel::<String>()
let msg = "hello".to_owned()

tx.send(msg)  // msg is moved
// println(msg)  // ERROR: msg was moved
```

### 4.3 Receive Operation

```covibe
impl<T> Receiver<T> {
    // Receive a value, blocking until one is available
    fn recv(self) -> Result<T, RecvError>

    // Try to receive without blocking
    fn try_recv(self) -> Result<T, TryRecvError>

    // Receive with timeout
    fn recv_timeout(self, timeout: Duration) -> Result<T, RecvTimeoutError>
}
```

**Blocking behavior:**
`recv()` blocks until a sender sends a value or all senders are dropped.

```covibe
let (tx, rx) = channel::<Int>()

spawn {
    tx.send(42)
}

let value = rx.recv().unwrap()  // Blocks until value arrives
println(value)  // 42
```

### 4.4 Channel Closure

Channels automatically close when all senders or receivers are dropped:

- If all `Sender<T>` are dropped, `recv()` returns `Err(RecvError::Closed)`
- If all `Receiver<T>` are dropped, `send()` returns `Err(SendError(value))`

```covibe
let (tx, rx) = channel::<Int>()

drop(tx)  // Close the sender side

match rx.recv() {
    Ok(val) => println("Received: {val}"),
    Err(RecvError::Closed) => println("Channel closed"),
}
```

### 4.5 Cloning Senders and Receivers

`Sender<T>` can be cloned to allow multiple producers:

```covibe
let (tx, rx) = channel::<Int>()
let tx2 = tx.clone()

spawn {
    tx.send(1)
}

spawn {
    tx2.send(2)
}

println(rx.recv().unwrap())  // Could be 1 or 2
println(rx.recv().unwrap())  // The other value
```

`Receiver<T>` is not cloneable to ensure single-consumer semantics. For multiple consumers, use multiple channels or a broadcast channel variant.

### 4.6 Channel Iteration

`Receiver<T>` implements `Iterator`, allowing consumption of all values:

```covibe
let (tx, rx) = channel::<Int>()

spawn {
    for i in 0..10 {
        tx.send(i)
    }
}

for value in rx {
    println(value)  // Prints 0, 1, 2, ..., 9
}
```

The iteration ends when all senders are dropped.

### 4.7 Buffered vs Unbuffered Channels

**Unbuffered (Synchronous):**
```covibe
let (tx, rx) = channel::<Int>()
// send() blocks until recv() is called (rendezvous)
```

**Buffered (Asynchronous):**
```covibe
let (tx, rx) = channel_buffered::<Int>(10)
// send() blocks only when buffer is full
```

**Zero-size Buffer:**
A channel with capacity 0 is equivalent to an unbuffered channel.

### 4.8 Channel Send/Sync Traits

- `Sender<T>` implements `Send` if `T: Send`
- `Sender<T>` implements `Sync` if `T: Send`
- `Receiver<T>` implements `Send` if `T: Send`
- `Receiver<T>` does not implement `Sync` (single consumer)

---

## 5. Select Statement

The `select` statement allows waiting on multiple channel operations simultaneously, proceeding with whichever completes first.

### 5.1 Select Syntax

```covibe
select {
    value = receiver1.recv() => {
        // Handle value from receiver1
    },
    value = receiver2.recv() => {
        // Handle value from receiver2
    },
    sender.send(data) => {
        // Send succeeded
    },
    default => {
        // No operation ready (non-blocking select)
    },
}
```

### 5.2 Select Semantics

- **Non-deterministic Choice**: If multiple operations are ready, one is chosen randomly
- **Blocking**: Select blocks until at least one operation can proceed (unless `default` is present)
- **Fair Scheduling**: Over time, all ready operations have equal probability of being chosen
- **Ownership**: Selected values have ownership transferred as with normal send/recv

### 5.3 Select with Default (Non-blocking)

```covibe
select {
    msg = rx.recv() => {
        println("Received: {msg}")
    },
    default => {
        println("No message available")
    },
}
```

### 5.4 Select with Timeout

```covibe
select {
    msg = rx.recv() => {
        println("Received: {msg}")
    },
    timeout(Duration::from_secs(5)) => {
        println("Timeout after 5 seconds")
    },
}
```

### 5.5 Select Example: Fan-In Pattern

```covibe
fn fan_in(rx1: Receiver<Int>, rx2: Receiver<Int>) -> Receiver<Int> {
    let (tx, rx) = channel()

    spawn {
        loop {
            select {
                val = rx1.recv() => {
                    match val {
                        Ok(v) => tx.send(v),
                        Err(_) => break,
                    }
                },
                val = rx2.recv() => {
                    match val {
                        Ok(v) => tx.send(v),
                        Err(_) => break,
                    }
                },
            }
        }
    }

    rx
}
```

### 5.6 Select Example: Timeout Pattern

```covibe
fn fetch_with_timeout(url: String, max_duration: Duration) -> Result<String, Error> {
    let (tx, rx) = channel()

    spawn {
        let result = http_fetch(url)
        tx.send(result)
    }

    select {
        result = rx.recv() => result.unwrap(),
        timeout(max_duration) => Err(Error::Timeout),
    }
}
```

### 5.7 Select Constraints

- All send operations in a select must have the same channel type for a given sender
- All receive operations must bind to a variable of the appropriate type
- The `default` branch cannot coexist with a `timeout` branch

---

## 6. Async/Await Desugaring

CoVibe provides `async`/`await` syntax as syntactic sugar over the task model. Async functions desugar to regular functions that spawn tasks and use channels for coordination.

### 6.1 Async Function Declaration

```covibe
async fn fetch_user(id: UserId) -> User {
    let response = http_get("/users/{id}").await
    parse_user(response)
}
```

### 6.2 Desugaring Rules

An async function:
```covibe
async fn foo(x: T) -> U {
    body
}
```

Desugars to:
```covibe
fn foo(x: T) -> impl Future<Output = U> {
    Future::from_task(spawn {
        body
    })
}
```

Where `Future<Output = U>` is a trait:
```covibe
trait Future {
    type Output

    fn poll(self, waker: Waker) -> Poll<Self::Output>
}

enum Poll<T> {
    Ready(T),
    Pending,
}
```

### 6.3 Await Expression

The `await` expression:
```covibe
let result = future_expr.await
```

Desugars to:
```covibe
let result = {
    let mut future = future_expr
    loop {
        match future.poll(current_waker()) {
            Poll::Ready(value) => break value,
            Poll::Pending => {
                yield_task()  // Yield control to scheduler
            }
        }
    }
}
```

### 6.4 Future Trait Implementation

The `Future` trait is implemented by the runtime:

```covibe
struct TaskFuture<T> {
    handle: JoinHandle<T>,
}

impl<T> Future for TaskFuture<T> {
    type Output = T

    fn poll(self, waker: Waker) -> Poll<T> {
        match self.handle.try_join() {
            Some(result) => Poll::Ready(result),
            None => {
                // Register waker to be notified when task completes
                register_waker(self.handle.task_id, waker)
                Poll::Pending
            }
        }
    }
}
```

### 6.5 Async Blocks

Anonymous async blocks create futures inline:

```covibe
let future = async {
    let x = compute().await
    let y = process(x).await
    x + y
}

let result = future.await
```

### 6.6 Async Closures

```covibe
let fetch = async |id: Int| {
    fetch_user(id).await
}

let user = fetch(42).await
```

### 6.7 Async Trait Methods

Traits can have async methods:

```covibe
trait Repository {
    async fn save(self, item: Item) -> Result<(), Error>
    async fn load(self, id: ItemId) -> Result<Item, Error>
}
```

### 6.8 Parallel Async Execution

Futures don't execute until awaited. To run multiple futures concurrently:

```covibe
// Sequential (one after another)
let user = fetch_user(1).await
let posts = fetch_posts(user.id).await

// Parallel (both run concurrently)
let (user, posts) = join(
    fetch_user(1),
    fetch_posts(1)
).await
```

The `join` function is defined as:

```covibe
async fn join<T, U>(f1: impl Future<Output = T>, f2: impl Future<Output = U>) -> (T, U) {
    let h1 = spawn { f1.await }
    let h2 = spawn { f2.await }
    (h1.join(), h2.join())
}
```

### 6.9 Async Main

The program entry point can be async:

```covibe
async fn main() {
    let result = fetch_data().await
    println(result)
}
```

This desugars to:
```covibe
fn main() {
    runtime::block_on(async {
        let result = fetch_data().await
        println(result)
    })
}
```

---

## 7. Structured Concurrency

Structured concurrency ensures that spawned tasks have bounded lifetimes and are properly cleaned up. CoVibe provides scopes that automatically join all spawned tasks when the scope exits.

### 7.1 Task Scope

```covibe
fn scope<F, R>(f: F) -> R
where
    F: FnOnce(&Scope) -> R
```

Example:

```covibe
let results = scope(|s| {
    let h1 = s.spawn({ compute_task_1() })
    let h2 = s.spawn({ compute_task_2() })
    let h3 = s.spawn({ compute_task_3() })

    (h1.join(), h2.join(), h3.join())
})  // All tasks are guaranteed to complete before scope exits
```

### 7.2 Scope Semantics

- All tasks spawned within a scope must complete before the scope exits
- If a scope exits normally, all tasks are joined
- If a scope panics, all tasks are cancelled (or allowed to finish, depending on policy)
- Scopes can be nested

### 7.3 Scope with Shared References

The scope allows tasks to borrow local variables:

```covibe
let data = vec![1, 2, 3, 4, 5]

scope(|s| {
    for chunk in data.chunks(2) {
        s.spawn({
            process_chunk(chunk)  // Borrows from data
        })
    }
})  // All tasks complete, data is still valid
```

This is safe because:
1. The scope ensures all tasks complete before returning
2. The borrow checker ensures `data` outlives the scope

### 7.4 Cancellation

Scopes support cancellation tokens:

```covibe
let cancel_token = CancellationToken::new()

scope(|s| {
    s.spawn({
        loop {
            if cancel_token.is_cancelled() {
                break
            }
            do_work()
        }
    })

    // After some condition
    cancel_token.cancel()
})
```

### 7.5 Error Handling in Scopes

If a task in a scope panics:

```covibe
let result = std::panic::catch_unwind(|| {
    scope(|s| {
        s.spawn({ panic("task failed") })
    })
})

match result {
    Ok(_) => println("Scope completed"),
    Err(e) => println("Scope panicked: {e}"),
}
```

### 7.6 Scope Example: Parallel Map

```covibe
fn parallel_map<T, U, F>(items: &[T], f: F) -> Vec<U>
where
    T: Sync,
    U: Send,
    F: Fn(&T) -> U + Sync,
{
    let results = Arc::new(Mutex::new(Vec::with_capacity(items.len())))

    scope(|s| {
        for (i, item) in items.iter().enumerate() {
            let results = results.clone()
            s.spawn({
                let result = f(item)
                results.lock().push((i, result))
            })
        }
    })

    let mut results = Arc::try_unwrap(results).unwrap().into_inner()
    results.sort_by_key(|(i, _)| *i)
    results.into_iter().map(|(_, v)| v).collect()
}
```

---

## 8. Scheduler Contract

The CoVibe runtime uses an M:N scheduler with work-stealing to efficiently distribute tasks across OS threads.

### 8.1 Scheduler Architecture

- **Worker Threads**: N OS threads (typically equal to number of CPU cores)
- **Per-Worker Queues**: Each worker has a local deque of ready tasks
- **Global Queue**: Overflow queue for load balancing
- **Work Stealing**: Idle workers steal tasks from busy workers

### 8.2 Scheduler Guarantees

1. **Progress**: Every ready task will eventually be scheduled (no starvation)
2. **Fairness**: No task is systematically prioritized over others (except explicit priorities)
3. **Work Conservation**: No worker is idle while tasks are ready
4. **Locality**: Tasks tend to run on the same worker (cache-friendly)

### 8.3 Yield Points

Tasks yield control to the scheduler at:
- `await` expressions
- Channel `send`/`recv` operations
- Explicit `yield_now()` calls
- Blocking I/O operations (internally converted to async)
- Synchronization primitives (mutex lock, etc.)
- Configurable CPU time limit (default: 10ms)

### 8.4 Task Priority (Optional)

Tasks can be spawned with priority hints:

```covibe
spawn_with_priority(Priority::High, {
    critical_task()
})
```

Priorities are hints only; the scheduler may ignore them for fairness.

### 8.5 Worker Thread Configuration

```covibe
runtime::Builder::new()
    .worker_threads(8)              // Number of worker threads
    .max_blocking_threads(512)      // For blocking operations
    .thread_stack_size(2 * 1024 * 1024)
    .build()
```

### 8.6 Blocking Operations

Blocking operations (e.g., synchronous file I/O) should be wrapped:

```covibe
spawn_blocking({
    // This runs on a separate thread pool
    std::fs::read_to_string("/path/to/file")
})
```

This prevents blocking the scheduler's worker threads.

### 8.7 Scheduler Metrics

The runtime exposes metrics:

```covibe
let metrics = runtime::metrics()
println("Active tasks: {}", metrics.active_tasks)
println("Idle workers: {}", metrics.idle_workers)
println("Steal attempts: {}", metrics.steal_attempts)
```

### 8.8 Custom Schedulers

Advanced users can provide custom schedulers implementing the `Scheduler` trait:

```covibe
trait Scheduler {
    fn schedule(self, task: Task)
    fn run(self)
    fn shutdown(self)
}
```

---

## 9. Atomic Types and Memory Ordering

CoVibe provides atomic types for lock-free concurrent programming with explicit memory ordering control.

### 9.1 Atomic Types

```covibe
struct Atomic<T> {
    // Internal representation
}

// Standard atomic types
type AtomicBool = Atomic<Bool>
type AtomicInt = Atomic<Int>
type AtomicUInt = Atomic<UInt>
type AtomicPtr<T> = Atomic<*mut T>
```

### 9.2 Memory Ordering

```covibe
enum Ordering {
    Relaxed,   // No ordering constraints
    Acquire,   // Acquire semantics (for loads)
    Release,   // Release semantics (for stores)
    AcqRel,    // Both acquire and release
    SeqCst,    // Sequentially consistent (strongest)
}
```

### 9.3 Atomic Operations

```covibe
impl<T> Atomic<T> {
    fn new(value: T) -> Self

    // Load with specified ordering
    fn load(self, ordering: Ordering) -> T

    // Store with specified ordering
    fn store(self, value: T, ordering: Ordering)

    // Swap (exchange)
    fn swap(self, value: T, ordering: Ordering) -> T

    // Compare-and-swap
    fn compare_exchange(
        self,
        current: T,
        new: T,
        success: Ordering,
        failure: Ordering
    ) -> Result<T, T>

    // Weak compare-and-swap (may spuriously fail)
    fn compare_exchange_weak(
        self,
        current: T,
        new: T,
        success: Ordering,
        failure: Ordering
    ) -> Result<T, T>

    // Fetch-and-add (for numeric types)
    fn fetch_add(self, value: T, ordering: Ordering) -> T

    // Fetch-and-sub (for numeric types)
    fn fetch_sub(self, value: T, ordering: Ordering) -> T

    // Fetch-and-bitwise-and
    fn fetch_and(self, value: T, ordering: Ordering) -> T

    // Fetch-and-bitwise-or
    fn fetch_or(self, value: T, ordering: Ordering) -> T

    // Fetch-and-bitwise-xor
    fn fetch_xor(self, value: T, ordering: Ordering) -> T
}
```

### 9.4 Memory Ordering Semantics

**Relaxed:**
- No synchronization or ordering constraints
- Guarantees atomicity only
- Cheapest, suitable for counters

```covibe
let counter = AtomicInt::new(0)
counter.fetch_add(1, Ordering::Relaxed)
```

**Acquire (for loads):**
- All subsequent memory operations cannot be reordered before this load
- Synchronizes with a Release store

**Release (for stores):**
- All previous memory operations cannot be reordered after this store
- Synchronizes with an Acquire load

**AcqRel:**
- Combines Acquire and Release for read-modify-write operations

**SeqCst:**
- Total global ordering of all SeqCst operations
- Strongest and most expensive

### 9.5 Atomic Example: Spin Lock

```covibe
struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    fn new() -> Self {
        SpinLock { locked: AtomicBool::new(false) }
    }

    fn lock(self) {
        while self.locked.swap(true, Ordering::Acquire) {
            // Spin until we acquire the lock
            while self.locked.load(Ordering::Relaxed) {
                std::hint::spin_loop()
            }
        }
    }

    fn unlock(self) {
        self.locked.store(false, Ordering::Release)
    }
}
```

### 9.6 Atomic Example: Message Passing

```covibe
struct Message {
    data: UInt,
    ready: AtomicBool,
}

// Producer
fn produce(msg: &mut Message) {
    msg.data = 42
    msg.ready.store(true, Ordering::Release)  // Release ensures data is visible
}

// Consumer
fn consume(msg: &Message) -> UInt {
    while !msg.ready.load(Ordering::Acquire) {  // Acquire ensures we see data
        std::hint::spin_loop()
    }
    msg.data
}
```

### 9.7 Fences

Explicit memory fences for advanced use cases:

```covibe
fn fence(ordering: Ordering)
fn compiler_fence(ordering: Ordering)
```

```covibe
std::sync::atomic::fence(Ordering::SeqCst)
```

---

## 10. Thread-Local Storage

Thread-local storage (TLS) allows each OS thread to have its own instance of a variable.

### 10.1 Thread-Local Declaration

```covibe
@thread_local
static COUNTER: Int = 0

fn increment() {
    COUNTER += 1
    println("Counter: {COUNTER}")
}

spawn {
    increment()  // Each task has its own COUNTER
    increment()
}

spawn {
    increment()  // Independent from the other task's COUNTER
}
```

### 10.2 Thread-Local Semantics

- Each OS thread gets its own copy of the variable
- Tasks running on the same OS thread share the same TLS value
- TLS is initialized lazily when first accessed on each thread
- TLS is destroyed when the thread exits

### 10.3 Thread-Local with Initialization

```covibe
@thread_local
static mut BUFFER: Vec<u8> = Vec::new()

fn get_buffer() -> &'static mut Vec<u8> {
    unsafe { &mut BUFFER }
}
```

### 10.4 Task-Local Storage

For per-task (rather than per-OS-thread) storage:

```covibe
struct TaskLocal<T> {
    // Internal implementation
}

static TASK_ID: TaskLocal<Int> = TaskLocal::new(|| generate_id())

fn get_task_id() -> Int {
    TASK_ID.get()
}
```

### 10.5 Thread-Local Constraints

- Thread-local variables must have a `'static` lifetime
- Mutable thread-locals require `unsafe` access
- Thread-locals cannot be accessed across threads

---

## 11. Parallel Iteration

CoVibe provides parallel versions of common iteration operations for data parallelism.

### 11.1 Parallel For

```covibe
for item in collection.par_iter() {
    process(item)
}
```

This automatically:
1. Splits the collection into chunks
2. Spawns tasks to process each chunk
3. Waits for all tasks to complete

### 11.2 Parallel Map

```covibe
let results = collection.par_iter()
    .map(|x| expensive_computation(x))
    .collect()
```

### 11.3 Parallel Filter

```covibe
let filtered = collection.par_iter()
    .filter(|x| predicate(x))
    .collect()
```

### 11.4 Parallel Reduce

```covibe
let sum = collection.par_iter()
    .map(|x| x * x)
    .reduce(|| 0, |a, b| a + b)
```

### 11.5 ParallelIterator Trait

```covibe
trait ParallelIterator {
    type Item

    fn par_iter(self) -> impl ParallelIterator<Item = Self::Item>
    fn map<F, U>(self, f: F) -> impl ParallelIterator<Item = U>
    where F: Fn(Self::Item) -> U + Sync

    fn filter<F>(self, f: F) -> impl ParallelIterator<Item = Self::Item>
    where F: Fn(&Self::Item) -> bool + Sync

    fn reduce<F, ID>(self, identity: ID, f: F) -> Self::Item
    where
        F: Fn(Self::Item, Self::Item) -> Self::Item + Sync,
        ID: Fn() -> Self::Item + Sync

    fn collect<C>(self) -> C
    where C: FromParallelIterator<Self::Item>
}
```

### 11.6 Chunk Size Control

```covibe
collection.par_iter()
    .with_chunk_size(1000)
    .map(|x| process(x))
    .collect()
```

### 11.7 Parallel Iteration Example

```covibe
fn parallel_sum_of_squares(numbers: &[f64]) -> f64 {
    numbers.par_iter()
        .map(|&x| x * x)
        .reduce(|| 0.0, |a, b| a + b)
}
```

### 11.8 Sequential vs Parallel Performance

Parallel iteration has overhead. Use when:
- Processing time per item is significant
- Collection is large (thousands+ of items)
- Operation is CPU-bound (not I/O bound)

For small collections or cheap operations, sequential iteration is faster.

---

## 12. Data Race Freedom Guarantee

CoVibe's ownership system guarantees data race freedom at compile time through the Send and Sync traits.

### 12.1 Data Race Definition

A data race occurs when:
1. Two or more tasks access the same memory location
2. At least one access is a write
3. The accesses are not synchronized

CoVibe prevents all data races in safe code.

### 12.2 Send Trait

```covibe
trait Send {}
```

A type `T` implements `Send` if it can be safely transferred to another task.

**Auto-implemented for:**
- All primitive types (Int, Bool, etc.)
- Types composed entirely of Send types
- Owned pointers (Box<T> where T: Send)

**Not Send:**
- Raw pointers (*const T, *mut T)
- `Rc<T>` (non-atomic reference counting)
- Types with thread-affinity (e.g., OpenGL contexts)

### 12.3 Sync Trait

```covibe
trait Sync {}
```

A type `T` implements `Sync` if `&T` (a shared reference to `T`) can be safely shared between tasks.

**Equivalently:** `T: Sync` if and only if `&T: Send`

**Auto-implemented for:**
- Immutable types
- Types with internal synchronization (Mutex<T>, AtomicInt)

**Not Sync:**
- `Cell<T>`, `RefCell<T>` (interior mutability without synchronization)
- Types that are not thread-safe

### 12.4 Send + Sync Enforcement

The compiler automatically enforces Send/Sync:

```covibe
let x = Rc::new(42)  // Rc is not Send

spawn {
    println(x)  // ERROR: Rc<Int> does not implement Send
}
```

Correct version:

```covibe
let x = Arc::new(42)  // Arc is Send + Sync

spawn {
    println(x)  // OK: Arc<Int> implements Send
}
```

### 12.5 Interior Mutability

Types with interior mutability must ensure thread safety:

**Not thread-safe:**
```covibe
struct Counter {
    count: Cell<Int>,  // Cell is not Sync
}

let counter = Counter { count: Cell::new(0) }

spawn {
    counter.count.set(counter.count.get() + 1)  // ERROR: Counter is not Sync
}
```

**Thread-safe:**
```covibe
struct Counter {
    count: AtomicInt,  // AtomicInt is Sync
}

let counter = Counter { count: AtomicInt::new(0) }

spawn {
    counter.count.fetch_add(1, Ordering::Relaxed)  // OK
}
```

### 12.6 Mutex and RwLock

```covibe
struct Mutex<T> {
    // Internal implementation
}

impl<T: Send> Mutex<T> {
    fn new(value: T) -> Self
    fn lock(self) -> MutexGuard<T>
    fn try_lock(self) -> Option<MutexGuard<T>>
}

// Mutex<T> is Sync if T is Send
unsafe impl<T: Send> Sync for Mutex<T> {}
```

Example:

```covibe
let counter = Arc::new(Mutex::new(0))

for _ in 0..10 {
    let counter = counter.clone()
    spawn {
        let mut guard = counter.lock()
        *guard += 1
    }
}
```

### 12.7 RwLock (Reader-Writer Lock)

```covibe
struct RwLock<T> {
    // Internal implementation
}

impl<T: Send> RwLock<T> {
    fn new(value: T) -> Self
    fn read(self) -> RwLockReadGuard<T>
    fn write(self) -> RwLockWriteGuard<T>
}

// Multiple readers OR one writer
let data = Arc::new(RwLock::new(vec![1, 2, 3]))

spawn {
    let guard = data.read()
    println(guard[0])  // Multiple readers OK
}

spawn {
    let mut guard = data.write()
    guard.push(4)  // Exclusive write
}
```

### 12.8 Compile-Time Verification

All data race prevention is verified at compile time with zero runtime overhead:

```covibe
let mut data = vec![1, 2, 3]

spawn {
    data.push(4)  // ERROR: cannot move `data` while borrowed
}

println(data[0])  // This would be a data race
```

The compiler reports:
```
error[E0502]: cannot borrow `data` as immutable because it is also borrowed as mutable
```

---

## 13. Actor Model Module

CoVibe provides an optional actor model abstraction in the standard library for message-based concurrency patterns.

### 13.1 Actor Trait

```covibe
trait Actor {
    type Message

    fn receive(self, msg: Self::Message)
}
```

### 13.2 Actor Spawning

```covibe
fn spawn_actor<A: Actor + Send + 'static>(actor: A) -> ActorRef<A::Message> {
    let (tx, rx) = channel()

    spawn {
        for msg in rx {
            actor.receive(msg)
        }
    }

    ActorRef { sender: tx }
}
```

### 13.3 ActorRef (Actor Reference)

```covibe
struct ActorRef<M> {
    sender: Sender<M>,
}

impl<M> ActorRef<M> {
    fn send(self, msg: M) -> Result<(), SendError<M>> {
        self.sender.send(msg)
    }

    fn clone(self) -> Self {
        ActorRef { sender: self.sender.clone() }
    }
}
```

### 13.4 Actor Example: Counter

```covibe
enum CounterMsg {
    Increment,
    Decrement,
    Get(Sender<Int>),
}

struct CounterActor {
    count: Int,
}

impl Actor for CounterActor {
    type Message = CounterMsg

    fn receive(self, msg: CounterMsg) {
        match msg {
            CounterMsg::Increment => {
                self.count += 1
            },
            CounterMsg::Decrement => {
                self.count -= 1
            },
            CounterMsg::Get(reply_to) => {
                reply_to.send(self.count)
            },
        }
    }
}

// Usage
let actor = spawn_actor(CounterActor { count: 0 })

actor.send(CounterMsg::Increment)
actor.send(CounterMsg::Increment)

let (tx, rx) = channel()
actor.send(CounterMsg::Get(tx))
let count = rx.recv().unwrap()
println("Count: {count}")  // 2
```

### 13.5 Actor Supervision

```covibe
struct Supervisor<A: Actor> {
    strategy: SupervisionStrategy,
    actor_factory: fn() -> A,
}

enum SupervisionStrategy {
    Restart,      // Restart actor on panic
    Stop,         // Stop actor on panic
    Escalate,     // Propagate panic to supervisor's supervisor
}

impl<A: Actor + Send + 'static> Supervisor<A> {
    fn spawn(self) -> ActorRef<A::Message> {
        // Implementation with panic handling and restart logic
    }
}
```

### 13.6 Actor Lifecycle

```covibe
trait Actor {
    type Message

    fn receive(self, msg: Self::Message)

    // Optional lifecycle hooks
    fn pre_start(self) {}
    fn post_stop(self) {}
    fn pre_restart(self, reason: Box<dyn Error>) {}
    fn post_restart(self) {}
}
```

### 13.7 Actor Registry

```covibe
static ACTOR_REGISTRY: ActorRegistry = ActorRegistry::new()

fn register_actor<M>(name: &str, actor_ref: ActorRef<M>) {
    ACTOR_REGISTRY.register(name, actor_ref)
}

fn lookup_actor<M>(name: &str) -> Option<ActorRef<M>> {
    ACTOR_REGISTRY.lookup(name)
}
```

### 13.8 Actor Example: Chat Room

```covibe
enum ChatMsg {
    Join(String, ActorRef<ClientMsg>),
    Leave(String),
    Broadcast(String, String),  // (sender_name, message)
}

enum ClientMsg {
    Message(String, String),  // (sender_name, message)
}

struct ChatRoom {
    clients: HashMap<String, ActorRef<ClientMsg>>,
}

impl Actor for ChatRoom {
    type Message = ChatMsg

    fn receive(self, msg: ChatMsg) {
        match msg {
            ChatMsg::Join(name, client_ref) => {
                self.clients.insert(name.clone(), client_ref)
                println("{name} joined the chat")
            },
            ChatMsg::Leave(name) => {
                self.clients.remove(&name)
                println("{name} left the chat")
            },
            ChatMsg::Broadcast(sender, text) => {
                for (name, client) in self.clients.iter() {
                    if name != &sender {
                        client.send(ClientMsg::Message(sender.clone(), text.clone()))
                    }
                }
            },
        }
    }
}
```

### 13.9 Actor Model vs Direct Channels

**Use actors when:**
- State needs to be encapsulated
- Complex message handling logic
- Supervision and fault tolerance needed

**Use channels when:**
- Simple producer-consumer patterns
- Pipeline architectures
- Lower overhead needed

---

## Summary

CoVibe's concurrency model provides:

1. **Lightweight Tasks**: Green threads with minimal overhead
2. **Type-Safe Message Passing**: Channels with ownership transfer
3. **Structured Concurrency**: Scopes ensure proper cleanup
4. **Async/Await**: Ergonomic high-level syntax
5. **Data Race Freedom**: Compile-time verification via Send/Sync
6. **Flexible Synchronization**: Atomics, mutexes, and channels
7. **Actor Model**: Optional message-based abstraction

All concurrency primitives compose cleanly and are built on the same memory-safe ownership foundation.

---

**END OF PART 5 OF 100. Awaiting confirmation to proceed to Part 6.**
