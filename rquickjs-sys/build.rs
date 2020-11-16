use std::{
    env, fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_PARALLEL");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_EXPORTS");
    println!("cargo:rerun-if-env-changed=CARGO_BINDGEN");
    println!("cargo:rerun-if-env-changed=CARGO_UPDATE_BINDINGS");

    let src_dir = Path::new("quickjs");
    let patches_dir = Path::new("patches");

    let out_dir = env::var("OUT_DIR").expect("No OUT_DIR env var is set by cargo");
    let out_dir = Path::new(&out_dir);

    let header_files = [
        "libbf.h",
        "libregexp-opcode.h",
        "libregexp.h",
        "libunicode-table.h",
        "libunicode.h",
        "list.h",
        "quickjs-atom.h",
        "quickjs-libc.h",
        "quickjs-opcode.h",
        "quickjs.h",
        "cutils.h",
    ];

    let source_files = [
        "libregexp.c",
        "libunicode.c",
        "cutils.c",
        "quickjs-libc.c",
        "quickjs.c",
        "libbf.c",
    ];

    let patch_files = ["rquickjs.patch"];

    let mut defines = vec![
        ("_GNU_SOURCE", None),
        ("CONFIG_VERSION", Some("\"2020-01-19\"")),
        ("CONFIG_BIGNUM", None),
    ];

    if env::var("CARGO_FEATURE_EXPORTS").is_ok() {
        defines.push(("CONFIG_MODULE_EXPORTS", None));
    }

    if env::var("CARGO_FEATURE_PARALLEL").is_ok() {
        defines.push(("CONFIG_PARALLEL", None));
    }

    for file in source_files.iter().chain(header_files.iter()) {
        fs::copy(src_dir.join(file), out_dir.join(file)).expect("Unable to copy source");
    }

    // applying patches
    for file in &patch_files {
        patch(out_dir, patches_dir.join(file));
    }

    // generating bindings
    bindgen(out_dir, out_dir.join("quickjs.h"), &defines);

    let mut builder = cc::Build::new();
    builder
        .extra_warnings(false)
        .flag("-Wno-array-bounds")
        .flag("-Wno-format-truncation");

    for (name, value) in &defines {
        builder.define(name, *value);
    }

    for src in &source_files {
        builder.file(out_dir.join(src));
    }

    builder.compile("libquickjs.a");
}

fn patch<D: AsRef<Path>, P: AsRef<Path>>(out_dir: D, patch: P) {
    let mut child = Command::new("patch")
        .arg("-p1")
        .stdin(Stdio::piped())
        .current_dir(out_dir)
        .spawn()
        .unwrap();

    {
        let patch = fs::read(patch).expect("Unable to read patch");

        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(&patch).expect("Unable to apply patch");
    }

    child.wait_with_output().expect("Unable to apply patch");
}

#[cfg(not(feature = "bindgen"))]
fn bindgen<'a, D, H, X, K, V>(out_dir: D, _header_file: H, _defines: X)
where
    D: AsRef<Path>,
    H: AsRef<Path>,
    X: IntoIterator<Item = &'a (K, Option<V>)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    let target = env::var("TARGET").unwrap();

    let bindings_file = out_dir.as_ref().join("bindings.rs");

    fs::write(
        &bindings_file,
        format!(
            r#"macro_rules! bindings_env {{
                ("TARGET") => {{ "{}" }};
            }}"#,
            target
        ),
    )
    .unwrap();
}

#[cfg(feature = "bindgen")]
fn bindgen<'a, D, H, X, K, V>(out_dir: D, header_file: H, defines: X)
where
    D: AsRef<Path>,
    H: AsRef<Path>,
    X: IntoIterator<Item = &'a (K, Option<V>)>,
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    let target = env::var("TARGET").unwrap();
    let out_dir = out_dir.as_ref();
    let header_file = header_file.as_ref();

    let mut cflags = vec![format!("--target={}", target)];

    //format!("-I{}", out_dir.parent().display()),

    for (name, value) in defines {
        cflags.push(if let Some(value) = value {
            format!("-D{}={}", name.as_ref(), value.as_ref())
        } else {
            format!("-D{}", name.as_ref())
        });
    }

    let bindings = bindgen_rs::Builder::default()
        .detect_include_paths(true)
        .clang_arg("-xc")
        .clang_args(cflags)
        .header(header_file.display().to_string())
        .whitelist_type("JS.*")
        .whitelist_function("js.*")
        .whitelist_function("JS.*")
        .whitelist_function("__JS.*")
        .whitelist_var("JS.*")
        .opaque_type("FILE")
        .blacklist_type("FILE")
        .blacklist_function("JS_DumpMemoryUsage")
        .generate()
        .expect("Unable to generate bindings");

    let bindings_file = out_dir.join("bindings.rs");

    bindings
        .write_to_file(&bindings_file)
        .expect("Couldn't write bindings");

    // Special case to support bundled bindings
    if env::var("CARGO_FEATURE_UPDATE_BINDINGS").is_ok() {
        let dest_dir = Path::new("src").join("bindings");
        fs::create_dir_all(&dest_dir).unwrap();

        let dest_file = format!("{}.rs", target);
        fs::copy(&bindings_file, dest_dir.join(&dest_file)).unwrap();
    }
}
