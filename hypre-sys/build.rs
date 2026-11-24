use std::{env, path::PathBuf, process::Command};

// hypre's integer #defines (HYPRE_PARCSR, ...) are passed as HYPRE_Int; type them as c_int.
#[derive(Debug)]
struct SignedHypreConstants;

impl bindgen::callbacks::ParseCallbacks for SignedHypreConstants {
    fn int_macro(&self, name: &str, _value: i64) -> Option<bindgen::callbacks::IntKind> {
        if name.starts_with("HYPRE_") {
            Some(bindgen::callbacks::IntKind::Int)
        } else {
            None
        }
    }
}

/// The MPI installation found by rsmpi's probe ($MPICC, mpicc on PATH, or pkg-config).
struct MpiConfig {
    mpicc: Option<String>,
    include_paths: Vec<PathBuf>,
}

#[cfg(feature = "mpi")]
fn mpi_config() -> Option<MpiConfig> {
    let lib = build_probe_mpi::probe().expect("no MPI installation found (set MPICC or put mpicc on PATH)");
    Some(MpiConfig {
        mpicc: lib.mpicc,
        include_paths: lib.include_paths,
    })
}

#[cfg(not(feature = "mpi"))]
fn mpi_config() -> Option<MpiConfig> {
    None
}

/// Homebrew's libomp (Apple clang ships no OpenMP runtime); $LIBOMP_PREFIX overrides.
fn libomp_prefix() -> PathBuf {
    println!("cargo:rerun-if-env-changed=LIBOMP_PREFIX");
    let prefix = match env::var("LIBOMP_PREFIX") {
        Ok(prefix) => PathBuf::from(prefix),
        Err(_) => {
            let out = Command::new("brew")
                .args(["--prefix", "libomp"])
                .output()
                .expect("feature `openmp` needs libomp on macOS: `brew install libomp`, or set LIBOMP_PREFIX");
            PathBuf::from(String::from_utf8(out.stdout).unwrap().trim())
        }
    };
    assert!(
        prefix.join("lib/libomp.dylib").exists(),
        "libomp not found in {} (`brew install libomp`, or set LIBOMP_PREFIX)",
        prefix.display()
    );
    prefix
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest_dir.join("vendor/hypre/src");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let mpi = mpi_config();
    let openmp = env::var_os("CARGO_FEATURE_OPENMP").is_some();
    let libomp = (openmp && target_os == "macos").then(libomp_prefix);

    // 1. Build and install the vendored hypre (static, -O3 -march=native).
    let mut config = cmake::Config::new(&src);
    config
        .profile("Release")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("CMAKE_INSTALL_LIBDIR", "lib")
        .define("HYPRE_ENABLE_MPI", if mpi.is_some() { "ON" } else { "OFF" })
        .define("HYPRE_ENABLE_OPENMP", if openmp { "ON" } else { "OFF" })
        .define("HYPRE_BUILD_EXAMPLES", "OFF")
        .define("HYPRE_BUILD_TESTS", "OFF")
        .cflag("-march=native");
    if let Some(mpicc) = mpi.as_ref().and_then(|m| m.mpicc.as_ref()) {
        // Same MPI as mpi-sys.
        config.define("MPI_C_COMPILER", mpicc);
    }
    if let Some(prefix) = &libomp {
        // Tell CMake's FindOpenMP how Apple clang uses Homebrew's libomp.
        let flags = format!("-Xclang -fopenmp -I{}", prefix.join("include").display());
        for lang in ["C", "CXX"] {
            config.define(format!("OpenMP_{lang}_FLAGS"), &flags);
            config.define(format!("OpenMP_{lang}_LIB_NAMES"), "omp");
        }
        config.define("OpenMP_omp_LIBRARY", prefix.join("lib/libomp.dylib"));
    }
    let dst = config.build();

    let include = dst.join("include");

    // 2. Link (internal BLAS/LAPACK are inside libHYPRE.a; MPI is linked by mpi-sys).
    println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
    println!("cargo:rustc-link-lib=static=HYPRE");
    if target_os != "macos" {
        println!("cargo:rustc-link-lib=dylib=m");
    }
    if openmp {
        match &libomp {
            Some(prefix) => {
                println!("cargo:rustc-link-search=native={}", prefix.join("lib").display());
                println!("cargo:rustc-link-lib=dylib=omp");
            }
            None => println!("cargo:rustc-link-lib=dylib=gomp"),
        }
    }

    // Exposed to dependents as DEP_HYPRE_INCLUDE (via `links = "hypre"`).
    println!("cargo:include={}", include.display());

    // 3. Generate the bindings (with OpenMP, hypre's headers include omp.h).
    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", include.display()))
        .allowlist_item("HYPRE_.*|MPI_Comm")
        .parse_callbacks(Box::new(SignedHypreConstants))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    if let Some(prefix) = &libomp {
        builder = builder.clang_arg(format!("-I{}", prefix.join("include").display()));
    }
    if let Some(mpi) = &mpi {
        for path in &mpi.include_paths {
            builder = builder.clang_arg(format!("-I{}", path.display()));
        }
        // One MPI_Comm type shared with mpi-sys (and the mpi crate).
        builder = builder
            .blocklist_type("MPI_Comm")
            .raw_line("pub use mpi_sys::MPI_Comm;");
    }

    let bindings = builder
        .generate()
        .expect("bindgen failed to generate hypre bindings");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    bindings.write_to_file(&out).expect("failed to write bindings.rs");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=MPICC");
}
