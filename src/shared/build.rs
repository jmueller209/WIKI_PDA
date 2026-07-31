fn main() {
    println!("cargo:rerun-if-changed=c_src/encodings.c");
    println!("cargo:rerun-if-changed=c_src/encodings.h");

    cc::Build::new()
        .file("c_src/encodings.c")
        .compile("encodings");
}
