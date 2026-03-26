fn main() {
    cc::Build::new()
        .file("openbw_wrapper/openbw_stub.c")
        .include("openbw_wrapper")
        .warnings(true)
        .compile("openbw_wrapper");
}
