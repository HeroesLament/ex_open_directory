fn main() {
    println!("cargo:rustc-link-lib=framework=OpenDirectory");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
}
