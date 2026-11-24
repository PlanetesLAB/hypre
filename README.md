# hypre

Rust bindings to [hypre](https://github.com/hypre-space/hypre), the LLNL library of scalable linear solvers and multigrid methods (BoomerAMG, PFMG, SMG, Krylov solvers).

- `hypre-sys`: raw FFI bindings generated with bindgen, plus a vendored static build of hypre v3.2.1.

## Usage

```toml
[dependencies]
hypre-sys = { git = "https://github.com/PlanetesLAB/hypre" }
```

Cargo fetches the vendored submodule automatically.

## Features

| Feature | Effect |
|---|---|
| (none) | Sequential hypre. `MPI_Comm` is hypre's integer stand-in; pass `0` as the communicator. |
| `mpi` | hypre built against your MPI. `MPI_Comm` is re-exported from `mpi-sys`, so communicators from the [`mpi`](https://crates.io/crates/mpi) crate (`world.as_raw()`) can be passed directly. MPI is located like rsmpi does it: `$MPICC`, `mpicc` on `PATH`, or pkg-config. |
| `openmp` | hypre built with OpenMP (Linux: GCC's `libgomp`; macOS: Homebrew's `libomp`, or set `LIBOMP_PREFIX`). Threads follow `OMP_NUM_THREADS`. |

## Requirements

- Rust (edition 2024)
- CMake >= 3.21
- A C compiler (Apple clang or GCC)
- libclang (for bindgen)
- With `mpi`: an MPI installation (e.g. Open MPI)
- With `openmp` on macOS: `brew install libomp`

hypre is pure C and uses its internal BLAS/LAPACK, so no other libraries or network access are needed at build time.

## Build notes

- hypre is linked statically and compiled with `-O3 -march=native`. Binaries only run on CPUs at least as capable as the one that built them; on clusters, build on the compute nodes.
- Types are hypre's defaults: `HYPRE_Int` and `HYPRE_BigInt` are 32-bit, `HYPRE_Real` is `f64`.
- hypre's C comments are kept as Rust doc comments; doc-tests are disabled because the comments are not Rust code.

## Licence

Licensed under either of Apache-2.0 (`LICENSE-APACHE`) or MIT (`LICENSE-MIT`) at your option, the same as hypre itself.
