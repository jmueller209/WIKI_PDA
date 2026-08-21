fn main() {
    println!("cargo:rerun-if-changed=../wiki_pda_api/lib/spatial_z/src");
    println!("cargo:rerun-if-changed=../wiki_pda_api/lib/spatial_z/include");

    println!("cargo:rerun-if-changed=../wiki_pda_api/lib/tempus/src");
    println!("cargo:rerun-if-changed=../wiki_pda_api/lib/tempus/include");

    cc::Build::new()
        .include("../wiki_pda_api/lib/spatial_z/include")
        .file("../wiki_pda_api/lib/spatial_z/src/codec.c")
        .file("../wiki_pda_api/lib/spatial_z/src/context.c")
        .file("../wiki_pda_api/lib/spatial_z/src/utils.c")
        .compile("spatial_z");

    cc::Build::new()
        .include("../wiki_pda_api/lib/tempus/include")
        .file("../wiki_pda_api/lib/tempus/src/codec.c")
        .compile("tempus");
}

