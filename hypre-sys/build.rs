use std::{env, path::PathBuf};

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

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest_dir.join("vendor/hypre/src");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    // 1. Build and install the vendored hypre (static, sequential, -O3 -march=native).
    let dst = cmake::Config::new(&src)
        .profile("Release")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("CMAKE_INSTALL_LIBDIR", "lib")
        .define("HYPRE_ENABLE_MPI", "OFF")
        .define("HYPRE_ENABLE_OPENMP", "OFF")
        .define("HYPRE_BUILD_EXAMPLES", "OFF")
        .define("HYPRE_BUILD_TESTS", "OFF")
        .cflag("-march=native")
        .build();

    let include = dst.join("include");

    // 2. Link (internal BLAS/LAPACK are inside libHYPRE.a).
    println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
    println!("cargo:rustc-link-lib=static=HYPRE");
    if target_os != "macos" {
        println!("cargo:rustc-link-lib=dylib=m");
    }

    // Exposed to dependents as DEP_HYPRE_INCLUDE (via `links = "hypre"`).
    println!("cargo:include={}", include.display());

    // 3. Generate the bindings.
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", include.display()))
        .allowlist_item("HYPRE_.*|MPI_Comm")
        .parse_callbacks(Box::new(SignedHypreConstants))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("bindgen failed to generate hypre bindings");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    bindings.write_to_file(&out).expect("failed to write bindings.rs");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");
}
