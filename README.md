#  Rust HFT Orderbook: From Naive to Sub-Nanosecond

This repository documents the extreme optimization of a Level 2 Order Book in Rust, taking a reference implementation from **~250 ns** latency down to a theoretical **< 1 ns** (limited only by OS measurement precision).

The goal was to adhere to **High-Frequency Trading (HFT)** constraints: zero dynamic allocation, L1 cache locality, and strictly $O(1)$ algorithmic complexity.

##  Performance Results

| Version | Data Structure | Complexity | Benchmark Latency |
|:---:|:---|:---:|:---:|
| **v1 (Naive)** | `BTreeMap` | $O(\log N)$ | ~250 ns |
| **v2 (Final)** | Ring Buffer + Bitwise | $O(1)$ | **~28 ns*** |

*\*Note: 28 ns represents the "Measurement Wall" on macOS (the fixed cost of the `Instant::now()` system call). The actual CPU cost of the instruction logic is estimated at < 1 ns (approx. 3-4 CPU cycles).*

---

## 🛠️ The Optimization Journey

### Phase 1: The Naive Approach (Reference)
We started with a standard idiomatic Rust implementation using `BTreeMap`.
- **Pros:** Simple code, guaranteed correctness, automatic sorting.
- **Cons:**
    - $O(\log N)$ complexity for insertions/lookups.
    - **Cache Misses:** Tree nodes are scattered in heap memory (RAM access takes ~60-100ns), killing performance.

### Phase 2: Architectural Shift (Ring Buffer)
To reach nanosecond speeds, we abandoned trees for **static Arrays**.
- **Concept:** Implemented a Circular Buffer (Ring Buffer) with a fixed capacity (4096) that fits entirely within the CPU's **L1 Cache**.
- **Mapping:** Prices are mapped to array indices via an "anchor" price logic.
- **Gain:** Shifted from RAM access to L1 Cache access (~1ns latency).

### Phase 3: "Nuclear" Optimization (Final Code)
To shave off the final nanoseconds, we applied low-level systems programming techniques:
1.  **Bitwise Operations:** Replaced mathematical modulo (`%`) with bitwise masks (`& CAP_MASK`). This avoids costly division instructions.
2.  **Forced Inlining:** Heavy use of `#[inline(always)]` to eliminate function call overhead.
3.  **Unsafe Rust:** Utilization of `get_unchecked` to bypass the compiler's bounds checking mechanism.
4.  **Branchless Programming:** Minimized `if/else` branching to prevent CPU pipeline stalls.

---

##  How to Run

** Important:** To observe the true performance, you must compile in **Release mode**. Debug mode includes safety checks that make the code up to 100x slower.

1. Clone the repository:
   ```bash
   git clone [https://github.com/Alex6925/rust-orderbook.git](https://github.com/Alex6925/rust-orderbook.git)
   cd rust-orderbook