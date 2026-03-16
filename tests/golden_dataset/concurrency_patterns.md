# Concurrency Patterns in Systems Programming

## Overview

Concurrency allows a program to make progress on multiple tasks simultaneously. In systems programming, concurrency is essential for I/O-bound services that must handle thousands of simultaneous connections. Getting concurrency right requires choosing the right model and rigorously avoiding shared mutable state.

## Threading Models

### OS Threads

Operating system threads are scheduled by the kernel and execute truly in parallel on multi-core hardware. They are expensive to create and carry several megabytes of stack overhead. Thread pools amortize creation cost. Threads communicate through shared memory protected by mutexes, condition variables, or read-write locks.

### Async/Await

Async runtimes multiplex many async tasks over a small pool of OS threads. When a task awaits I/O, the runtime suspends it and runs another ready task on the same thread. This model is efficient for I/O-bound workloads because threads are never blocked waiting. Tokio and async-std are the dominant Rust async runtimes.

## Synchronization Primitives

Mutexes protect critical sections by allowing only one thread access at a time. A poisoned mutex, acquired when the holding thread panics, must be recovered explicitly in Rust. Read-write locks allow multiple concurrent readers or one exclusive writer, improving throughput for read-heavy workloads.

### Channels

Channels decouple producers from consumers without shared memory. MPSC (multi-producer, single-consumer) channels are the standard pattern: many tasks send work items; one task processes them. Bounded channels provide backpressure, preventing producers from overwhelming consumers.

## Common Pitfalls

Deadlocks occur when two threads each hold a lock the other needs. Livelock happens when threads continuously react to each other without progressing. Data races — unsynchronized access to shared memory from multiple threads — are undefined behavior. Rust's ownership system eliminates data races at compile time.

## Actor Model

The actor model wraps state in isolated actors that communicate exclusively through message passing. No actor can access another's state directly. This eliminates entire classes of synchronization bugs at the cost of indirection.
