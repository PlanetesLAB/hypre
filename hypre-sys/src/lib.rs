#![allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    improper_ctypes,
    clippy::all
)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;
    use std::ptr;

    /// Communicator for the test: MPI_COMM_WORLD with MPI, 0 in the sequential build.
    unsafe fn world() -> MPI_Comm {
        #[cfg(feature = "mpi")]
        unsafe {
            assert_eq!(mpi_sys::MPI_Init(ptr::null_mut(), ptr::null_mut()), 0);
            mpi_sys::RSMPI_COMM_WORLD
        }
        #[cfg(not(feature = "mpi"))]
        0
    }

    unsafe fn finalize_world() {
        #[cfg(feature = "mpi")]
        unsafe {
            mpi_sys::MPI_Finalize();
        }
    }

    /// 1D Poisson (tridiag -1, 2, -1) with exact solution x = 1, solved with BoomerAMG.
    #[test]
    fn boomeramg_solves_1d_poisson() {
        const N: HYPRE_BigInt = 100;

        unsafe {
            let comm = world();
            assert_eq!(HYPRE_Initialize(), 0);

            // Matrix
            let mut a: HYPRE_IJMatrix = ptr::null_mut();
            assert_eq!(HYPRE_IJMatrixCreate(comm, 0, N - 1, 0, N - 1, &mut a), 0);
            assert_eq!(HYPRE_IJMatrixSetObjectType(a, HYPRE_PARCSR), 0);
            assert_eq!(HYPRE_IJMatrixInitialize(a), 0);
            for i in 0..N {
                let mut cols = vec![i];
                let mut vals = vec![2.0];
                if i > 0 {
                    cols.push(i - 1);
                    vals.push(-1.0);
                }
                if i < N - 1 {
                    cols.push(i + 1);
                    vals.push(-1.0);
                }
                let mut ncols = cols.len() as HYPRE_Int;
                assert_eq!(
                    HYPRE_IJMatrixSetValues(a, 1, &mut ncols, &i, cols.as_ptr(), vals.as_ptr()),
                    0
                );
            }
            assert_eq!(HYPRE_IJMatrixAssemble(a), 0);
            let mut par_a: HYPRE_ParCSRMatrix = ptr::null_mut();
            assert_eq!(
                HYPRE_IJMatrixGetObject(a, &mut par_a as *mut _ as *mut *mut c_void),
                0
            );

            // Vectors: b = A * ones (1 at both ends, 0 inside), x = 0
            let idx: Vec<HYPRE_BigInt> = (0..N).collect();
            let mut b_vals = vec![0.0; N as usize];
            b_vals[0] = 1.0;
            b_vals[N as usize - 1] = 1.0;
            let x_vals = vec![0.0; N as usize];

            let mut b: HYPRE_IJVector = ptr::null_mut();
            let mut x: HYPRE_IJVector = ptr::null_mut();
            for (v, vals) in [(&mut b, &b_vals), (&mut x, &x_vals)] {
                assert_eq!(HYPRE_IJVectorCreate(comm, 0, N - 1, v), 0);
                assert_eq!(HYPRE_IJVectorSetObjectType(*v, HYPRE_PARCSR), 0);
                assert_eq!(HYPRE_IJVectorInitialize(*v), 0);
                assert_eq!(
                    HYPRE_IJVectorSetValues(*v, N as HYPRE_Int, idx.as_ptr(), vals.as_ptr()),
                    0
                );
                assert_eq!(HYPRE_IJVectorAssemble(*v), 0);
            }
            let mut par_b: HYPRE_ParVector = ptr::null_mut();
            let mut par_x: HYPRE_ParVector = ptr::null_mut();
            assert_eq!(HYPRE_IJVectorGetObject(b, &mut par_b as *mut _ as *mut *mut c_void), 0);
            assert_eq!(HYPRE_IJVectorGetObject(x, &mut par_x as *mut _ as *mut *mut c_void), 0);

            // Solve
            let mut solver: HYPRE_Solver = ptr::null_mut();
            assert_eq!(HYPRE_BoomerAMGCreate(&mut solver), 0);
            assert_eq!(HYPRE_BoomerAMGSetTol(solver, 1e-10), 0);
            assert_eq!(HYPRE_BoomerAMGSetup(solver, par_a, par_b, par_x), 0);
            assert_eq!(HYPRE_BoomerAMGSolve(solver, par_a, par_b, par_x), 0);

            let mut sol = vec![0.0; N as usize];
            assert_eq!(
                HYPRE_IJVectorGetValues(x, N as HYPRE_Int, idx.as_ptr(), sol.as_mut_ptr()),
                0
            );
            for (i, s) in sol.iter().enumerate() {
                assert!((s - 1.0).abs() < 1e-6, "x[{i}] = {s}, expected 1");
            }

            HYPRE_BoomerAMGDestroy(solver);
            HYPRE_IJMatrixDestroy(a);
            HYPRE_IJVectorDestroy(b);
            HYPRE_IJVectorDestroy(x);
            HYPRE_Finalize();
            finalize_world();
        }
    }
}
