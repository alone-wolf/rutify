fn main() {
    println!("cargo:warning=build.rs is running");

    // println!("cargo:rerun-if-changed=build.rs");
    slint_build::compile("slint/app.slint").expect("Slint UI build failed") ;
}
